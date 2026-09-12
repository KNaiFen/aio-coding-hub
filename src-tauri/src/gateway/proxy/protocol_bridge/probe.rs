//! Strict probe validation against original upstream events, before translation.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(crate) fn parse_codex_chatgpt_account_id(id_token: Option<&str>) -> Option<String> {
    let token = id_token.map(str::trim).filter(|value| !value.is_empty())?;
    let payload_part = token.split('.').nth(1)?;
    let payload = URL_SAFE_NO_PAD.decode(payload_part).ok().or_else(|| {
        let mut padded = payload_part.to_string();
        while padded.len() % 4 != 0 {
            padded.push('=');
        }
        URL_SAFE_NO_PAD.decode(padded).ok()
    })?;
    let json: Value = serde_json::from_slice(&payload).ok()?;
    json.get("https://api.openai.com/auth")
        .and_then(|value| value.get("chatgpt_account_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeProtocol {
    Responses,
    Chat,
    Anthropic,
    Gemini,
}

fn text(value: &Value) -> bool {
    value.as_str().is_some_and(|value| !value.trim().is_empty())
}

fn no_error(value: &Value) -> Result<(), &'static str> {
    if !value.is_object() {
        return Err("PROBE_STRUCTURE");
    }
    if value.get("error").is_some_and(|error| !error.is_null()) {
        return Err("PROBE_UPSTREAM_ERROR");
    }
    Ok(())
}

fn answer_blocks(value: &Value, kind: &str) -> Result<bool, &'static str> {
    let mut answered = false;
    for part in value.as_array().ok_or("PROBE_STRUCTURE")? {
        no_error(part)?;
        if part["type"] == kind {
            if !part["text"].is_string() {
                return Err("PROBE_STRUCTURE");
            }
            answered |= text(&part["text"]);
        }
    }
    Ok(answered)
}

fn gemini_text(content: &Value) -> Result<bool, &'static str> {
    no_error(content)?;
    if content.get("role").is_some_and(|role| role != "model") {
        return Err("PROBE_STRUCTURE");
    }
    let mut answered = false;
    for part in content["parts"].as_array().ok_or("PROBE_STRUCTURE")? {
        no_error(part)?;
        if part.get("text").is_some() {
            if !part["text"].is_string() {
                return Err("PROBE_STRUCTURE");
            }
            answered |= part["thought"] != true && text(&part["text"]);
        }
    }
    Ok(answered)
}

fn finish(reason: Option<&str>, allowed: &[&str]) -> Result<(), &'static str> {
    match reason {
        Some(reason) if allowed.contains(&reason) => Ok(()),
        Some(_) => Err("PROBE_TERMINATION"),
        None => Err("PROBE_UNFINISHED"),
    }
}

pub(crate) fn validate_json(protocol: ProbeProtocol, value: &Value) -> Result<(), &'static str> {
    no_error(value)?;
    let answered = match protocol {
        ProbeProtocol::Responses => {
            if value["object"] != "response" {
                return Err("PROBE_STRUCTURE");
            }
            match value["status"].as_str() {
                Some("completed") => {}
                Some("incomplete")
                    if value["incomplete_details"]["reason"] == "max_output_tokens" => {}
                Some("incomplete") => return Err("PROBE_TERMINATION"),
                Some("failed" | "cancelled") => return Err("PROBE_UPSTREAM_ERROR"),
                _ => return Err("PROBE_UNFINISHED"),
            }
            let mut answered = false;
            for item in value["output"].as_array().ok_or("PROBE_STRUCTURE")? {
                no_error(item)?;
                if item["type"] == "message" {
                    if item["role"] != "assistant" {
                        return Err("PROBE_STRUCTURE");
                    }
                    answered |= answer_blocks(&item["content"], "output_text")?;
                }
            }
            answered
        }
        ProbeProtocol::Chat => {
            if value["object"] != "chat.completion" {
                return Err("PROBE_STRUCTURE");
            }
            let choices = value["choices"].as_array().ok_or("PROBE_STRUCTURE")?;
            let mut answered = false;
            for choice in choices {
                no_error(choice)?;
                finish(choice["finish_reason"].as_str(), &["stop", "length"])?;
                no_error(&choice["message"])?;
                if choice["message"]["role"] != "assistant" {
                    return Err("PROBE_STRUCTURE");
                }
                if !choice["message"]["content"].is_null()
                    && !choice["message"]["content"].is_string()
                {
                    return Err("PROBE_STRUCTURE");
                }
                answered |= text(&choice["message"]["content"]);
            }
            answered
        }
        ProbeProtocol::Anthropic => {
            if value["type"] != "message" || value["role"] != "assistant" {
                return Err("PROBE_STRUCTURE");
            }
            finish(
                value["stop_reason"].as_str(),
                &["end_turn", "stop_sequence", "max_tokens"],
            )?;
            answer_blocks(&value["content"], "text")?
        }
        ProbeProtocol::Gemini => {
            if value.pointer("/promptFeedback/blockReason").is_some() {
                return Err("PROBE_TERMINATION");
            }
            let candidates = value["candidates"].as_array().ok_or("PROBE_STRUCTURE")?;
            let mut answered = false;
            for candidate in candidates {
                no_error(candidate)?;
                finish(candidate["finishReason"].as_str(), &["STOP", "MAX_TOKENS"])?;
                if !candidate["content"].is_null() {
                    answered |= gemini_text(&candidate["content"])?;
                }
            }
            answered
        }
    };
    if answered {
        Ok(())
    } else {
        Err("PROBE_NO_TEXT")
    }
}

#[derive(Default)]
struct Choice {
    assistant: bool,
    text: bool,
    finished: bool,
}

pub(crate) struct ProbeStream {
    protocol: ProbeProtocol,
    response_id: Option<String>,
    choices: HashMap<u64, Choice>,
    text_blocks: HashSet<u64>,
    open_blocks: HashSet<u64>,
    anthropic_started: bool,
    anthropic_text: bool,
    anthropic_finished: bool,
    terminal: bool,
}

impl ProbeStream {
    pub(crate) fn new(protocol: ProbeProtocol) -> Self {
        Self {
            protocol,
            response_id: None,
            choices: HashMap::new(),
            text_blocks: HashSet::new(),
            open_blocks: HashSet::new(),
            anthropic_started: false,
            anthropic_text: false,
            anthropic_finished: false,
            terminal: false,
        }
    }

    pub(crate) fn terminal(&self) -> bool {
        self.terminal
    }

    pub(crate) fn frame(&mut self, bytes: &[u8]) -> Result<(), &'static str> {
        let frame = std::str::from_utf8(bytes).map_err(|_| "PROBE_STRUCTURE")?;
        let data = frame
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .collect::<Vec<_>>();
        if data.is_empty() {
            return Ok(());
        }
        if data.len() == 1 && data[0].trim() == "[DONE]" {
            if self.protocol == ProbeProtocol::Chat {
                if self.choices.is_empty() || self.choices.values().any(|choice| !choice.finished) {
                    return Err("PROBE_UNFINISHED");
                }
                if !self
                    .choices
                    .values()
                    .any(|choice| choice.assistant && choice.text)
                {
                    return Err("PROBE_NO_TEXT");
                }
                self.terminal = true;
                return Ok(());
            }
            return if self.terminal {
                Ok(())
            } else {
                Err("PROBE_UNFINISHED")
            };
        }
        let (event, value) =
            crate::gateway::proxy::sse::parse_sse_frame(frame).ok_or("PROBE_STRUCTURE")?;
        no_error(&value)?;
        if event == "error" {
            return Err("PROBE_UPSTREAM_ERROR");
        }
        if self.terminal {
            return Err("PROBE_STRUCTURE");
        }
        match self.protocol {
            ProbeProtocol::Responses => {
                if !event.starts_with("response.") {
                    return Err("PROBE_STRUCTURE");
                }
                if value
                    .get("type")
                    .is_some_and(|value| value.as_str() != Some(event.as_str()))
                {
                    return Err("PROBE_STRUCTURE");
                }
                if let Some(response) = value.get("response") {
                    no_error(response)?;
                }
                if let Some(id) = value.get("response_id").and_then(Value::as_str) {
                    if self
                        .response_id
                        .as_deref()
                        .is_some_and(|current| current != id)
                    {
                        return Err("PROBE_STRUCTURE");
                    }
                    self.response_id = Some(id.to_string());
                }
                if matches!(event.as_str(), "response.failed" | "response.cancelled") {
                    return Err("PROBE_UPSTREAM_ERROR");
                }
                if event == "response.created"
                    || event == "response.in_progress"
                    || event == "response.completed"
                    || event == "response.incomplete"
                {
                    let response = value.get("response").ok_or("PROBE_STRUCTURE")?;
                    let id = response["id"]
                        .as_str()
                        .filter(|id| !id.is_empty())
                        .ok_or("PROBE_STRUCTURE")?;
                    if self
                        .response_id
                        .as_deref()
                        .is_some_and(|current| current != id)
                    {
                        return Err("PROBE_STRUCTURE");
                    }
                    self.response_id = Some(id.to_string());
                    if matches!(event.as_str(), "response.completed" | "response.incomplete") {
                        if response["status"].as_str() != event.strip_prefix("response.") {
                            return Err("PROBE_STRUCTURE");
                        }
                        validate_json(self.protocol, response)?;
                        self.terminal = true;
                    }
                }
            }
            ProbeProtocol::Chat => {
                if value["object"] != "chat.completion.chunk" {
                    return Err("PROBE_STRUCTURE");
                }
                for value in value["choices"].as_array().ok_or("PROBE_STRUCTURE")? {
                    no_error(value)?;
                    let index = value["index"].as_u64().ok_or("PROBE_STRUCTURE")?;
                    let choice = self.choices.entry(index).or_default();
                    no_error(&value["delta"])?;
                    let delta = value["delta"].as_object().ok_or("PROBE_STRUCTURE")?;
                    if choice.finished && !delta.is_empty() {
                        return Err("PROBE_STRUCTURE");
                    }
                    if let Some(role) = delta.get("role") {
                        if role != "assistant" {
                            return Err("PROBE_STRUCTURE");
                        }
                        choice.assistant = true;
                    }
                    if delta
                        .get("content")
                        .is_some_and(|content| !content.is_null() && !content.is_string())
                    {
                        return Err("PROBE_STRUCTURE");
                    }
                    choice.text |= delta.get("content").is_some_and(text);
                    if !value["finish_reason"].is_null() {
                        finish(value["finish_reason"].as_str(), &["stop", "length"])?;
                        choice.finished = true;
                    }
                }
            }
            ProbeProtocol::Anthropic => {
                if value
                    .get("type")
                    .is_some_and(|kind| kind.as_str() != Some(event.as_str()))
                {
                    return Err("PROBE_STRUCTURE");
                }
                match event.as_str() {
                    "message_start" => {
                        let message = &value["message"];
                        no_error(message)?;
                        if self.anthropic_started
                            || message["type"] != "message"
                            || message["role"] != "assistant"
                        {
                            return Err("PROBE_STRUCTURE");
                        }
                        self.anthropic_started = true;
                    }
                    "content_block_start" => {
                        if !self.anthropic_started {
                            return Err("PROBE_STRUCTURE");
                        }
                        let index = value["index"].as_u64().ok_or("PROBE_STRUCTURE")?;
                        no_error(&value["content_block"])?;
                        if !self.open_blocks.insert(index) {
                            return Err("PROBE_STRUCTURE");
                        }
                        if value["content_block"]["type"] == "text" {
                            if !value["content_block"]["text"].is_string() {
                                return Err("PROBE_STRUCTURE");
                            }
                            self.text_blocks.insert(index);
                            self.anthropic_text |= text(&value["content_block"]["text"]);
                        }
                    }
                    "content_block_delta" => {
                        let index = value["index"].as_u64().ok_or("PROBE_STRUCTURE")?;
                        if !self.open_blocks.contains(&index) {
                            return Err("PROBE_STRUCTURE");
                        }
                        no_error(&value["delta"])?;
                        if value["delta"]["type"] == "text_delta" {
                            if !self.text_blocks.contains(&index) {
                                return Err("PROBE_STRUCTURE");
                            }
                            if !value["delta"]["text"].is_string() {
                                return Err("PROBE_STRUCTURE");
                            }
                            self.anthropic_text |= text(&value["delta"]["text"]);
                        }
                    }
                    "message_delta" => {
                        if !self.anthropic_started {
                            return Err("PROBE_STRUCTURE");
                        }
                        if !self.open_blocks.is_empty() {
                            return Err("PROBE_UNFINISHED");
                        }
                        no_error(&value["delta"])?;
                        finish(
                            value["delta"]["stop_reason"].as_str(),
                            &["end_turn", "stop_sequence", "max_tokens"],
                        )?;
                        self.anthropic_finished = true;
                    }
                    "message_stop" => {
                        if !self.anthropic_finished {
                            return Err("PROBE_UNFINISHED");
                        }
                        if !self.anthropic_text {
                            return Err("PROBE_NO_TEXT");
                        }
                        self.terminal = true;
                    }
                    "content_block_stop" => {
                        let index = value["index"].as_u64().ok_or("PROBE_STRUCTURE")?;
                        if !self.open_blocks.remove(&index) {
                            return Err("PROBE_STRUCTURE");
                        }
                    }
                    "ping" => {}
                    _ => return Err("PROBE_STRUCTURE"),
                }
            }
            ProbeProtocol::Gemini => {
                if value.pointer("/promptFeedback/blockReason").is_some() {
                    return Err("PROBE_TERMINATION");
                }
                for candidate in value["candidates"].as_array().ok_or("PROBE_STRUCTURE")? {
                    no_error(candidate)?;
                    let index = candidate["index"].as_u64().ok_or("PROBE_STRUCTURE")?;
                    let choice = self.choices.entry(index).or_default();
                    if !candidate["content"].is_null() {
                        if choice.finished {
                            return Err("PROBE_STRUCTURE");
                        }
                        choice.text |= gemini_text(&candidate["content"])?;
                    }
                    if !candidate["finishReason"].is_null() {
                        finish(candidate["finishReason"].as_str(), &["STOP", "MAX_TOKENS"])?;
                        choice.finished = true;
                    }
                }
                if !self.choices.is_empty() && self.choices.values().all(|choice| choice.finished) {
                    if !self.choices.values().any(|choice| choice.text) {
                        return Err("PROBE_NO_TEXT");
                    }
                    self.terminal = true;
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn next_frame_end(buffer: &[u8]) -> Option<usize> {
    crate::gateway::proxy::sse::find_sse_event_end(buffer)
}

pub(crate) fn gemini_oauth_frame(
    stream: &mut ProbeStream,
    bytes: &[u8],
) -> Result<(), &'static str> {
    let frame = std::str::from_utf8(bytes).map_err(|_| "PROBE_STRUCTURE")?;
    if !frame.lines().any(|line| line.starts_with("data:")) {
        return Ok(());
    }
    let (_, value) = crate::gateway::proxy::sse::parse_sse_frame(frame).ok_or("PROBE_STRUCTURE")?;
    let response = gemini_oauth_response(&value)?;
    let frame = format!("data: {response}\n\n");
    stream.frame(frame.as_bytes())
}

pub(crate) fn gemini_oauth_response(value: &Value) -> Result<&Value, &'static str> {
    no_error(value)?;
    value.get("response").ok_or("PROBE_STRUCTURE")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn responses(status: &str, text: &str) -> Value {
        json!({"id":"resp_1", "object":"response", "status":status,
            "incomplete_details":{"reason":"max_output_tokens"},
            "output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":text}]}]})
    }

    #[test]
    fn original_json_protocols_require_text_and_explicit_supported_termination() {
        for value in [
            responses("completed", "OK"),
            responses("incomplete", "partial"),
        ] {
            assert_eq!(validate_json(ProbeProtocol::Responses, &value), Ok(()));
        }
        for status in ["completed", "incomplete"] {
            assert_eq!(
                validate_json(ProbeProtocol::Responses, &responses(status, " \n")),
                Err("PROBE_NO_TEXT")
            );
        }
        let mut invalid = responses("incomplete", "OK");
        invalid["incomplete_details"]["reason"] = json!("content_filter");
        assert_eq!(
            validate_json(ProbeProtocol::Responses, &invalid),
            Err("PROBE_TERMINATION")
        );
        invalid = responses("completed", "OK");
        invalid["output"][0]["type"] = json!("reasoning");
        assert_eq!(
            validate_json(ProbeProtocol::Responses, &invalid),
            Err("PROBE_NO_TEXT")
        );
        for reason in ["stop", "length"] {
            let value = json!({"object":"chat.completion","choices":[{"message":{"role":"assistant","content":"OK"},"finish_reason":reason}]});
            assert_eq!(validate_json(ProbeProtocol::Chat, &value), Ok(()));
        }
        for reason in ["tool_calls", "content_filter", "unknown"] {
            let value = json!({"object":"chat.completion","choices":[{"message":{"role":"assistant","content":"OK"},"finish_reason":reason}]});
            assert!(validate_json(ProbeProtocol::Chat, &value).is_err());
        }
        for reason in ["end_turn", "stop_sequence", "max_tokens"] {
            let value = json!({"type":"message","role":"assistant","stop_reason":reason,"content":[{"type":"text","text":"OK"}]});
            assert_eq!(validate_json(ProbeProtocol::Anthropic, &value), Ok(()));
        }
        for reason in ["STOP", "MAX_TOKENS"] {
            let mut value =
                json!({"candidates":[{"finishReason":reason,"content":{"parts":[{"text":"OK"}]}}]});
            assert_eq!(validate_json(ProbeProtocol::Gemini, &value), Ok(()));
            value["candidates"][0]["content"]["parts"][0]["thought"] = json!(true);
            assert_eq!(
                validate_json(ProbeProtocol::Gemini, &value),
                Err("PROBE_NO_TEXT")
            );
        }
        for protocol in [
            ProbeProtocol::Responses,
            ProbeProtocol::Chat,
            ProbeProtocol::Anthropic,
            ProbeProtocol::Gemini,
        ] {
            for value in [
                json!({"error":{"message":"failed"}}),
                json!({}),
                json!([]),
                json!(null),
            ] {
                assert!(validate_json(protocol, &value).is_err());
            }
        }
    }

    #[test]
    fn gemini_json_and_sse_only_accept_model_or_missing_role() {
        for role in [
            None,
            Some(json!("model")),
            Some(json!("user")),
            Some(json!("assistant")),
            Some(json!("system")),
            Some(json!(null)),
            Some(json!(42)),
        ] {
            let mut value = json!({"candidates":[{"index":0,"finishReason":"STOP","content":{"parts":[{"text":"OK"}]}}]});
            if let Some(role) = &role {
                value["candidates"][0]["content"]["role"] = role.clone();
            }
            let expected = if role.as_ref().is_none_or(|role| role == "model") {
                Ok(())
            } else {
                Err("PROBE_STRUCTURE")
            };
            assert_eq!(validate_json(ProbeProtocol::Gemini, &value), expected);
            let mut stream = ProbeStream::new(ProbeProtocol::Gemini);
            assert_eq!(
                stream.frame(format!("data: {value}\n\n").as_bytes()),
                expected
            );
            assert_eq!(stream.terminal(), expected.is_ok());
            let mut wrapped_stream = ProbeStream::new(ProbeProtocol::Gemini);
            let wrapped = json!({"response": value});
            assert_eq!(
                gemini_oauth_frame(
                    &mut wrapped_stream,
                    format!("data: {wrapped}\n\n").as_bytes()
                ),
                expected
            );
            assert_eq!(wrapped_stream.terminal(), expected.is_ok());
        }
    }

    #[test]
    fn responses_sse_is_chunk_and_utf8_boundary_independent_and_requires_matching_terminal() {
        let value = responses("incomplete", "\u{597d}");
        let frames = format!(": heartbeat\r\n\r\nevent: response.created\ndata: {{\"response\":{{\"id\":\"resp_1\"}}}}\n\nevent: response.incomplete\ndata: {{\"response\":{value}}}\n\n");
        for split in 1..frames.len() {
            let mut buffer = Vec::new();
            let mut stream = ProbeStream::new(ProbeProtocol::Responses);
            for chunk in [&frames.as_bytes()[..split], &frames.as_bytes()[split..]] {
                buffer.extend_from_slice(chunk);
                while let Some(end) = next_frame_end(&buffer) {
                    stream.frame(&buffer[..end]).unwrap();
                    buffer.drain(..end);
                }
            }
            assert!(stream.terminal());
            assert!(buffer.is_empty());
        }
        let mut stream = ProbeStream::new(ProbeProtocol::Responses);
        stream
            .frame(b"event: response.created\ndata: {\"response\":{\"id\":\"other\"}}\n\n")
            .unwrap();
        assert_eq!(
            stream.frame(
                format!("event: response.incomplete\ndata: {{\"response\":{value}}}\n\n")
                    .as_bytes()
            ),
            Err("PROBE_STRUCTURE")
        );
        assert!(ProbeStream::new(ProbeProtocol::Responses)
            .frame(b"data: [DONE]\n\n")
            .is_err());
    }

    #[test]
    fn chat_stream_requires_same_choice_answer_finish_and_done() {
        let mut stream = ProbeStream::new(ProbeProtocol::Chat);
        stream.frame(b"data: {\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"OK\"},\"finish_reason\":null}]}\n\n").unwrap();
        assert!(!stream.terminal());
        assert_eq!(stream.frame(b"data: [DONE]\n\n"), Err("PROBE_UNFINISHED"));
        stream.frame(b"data: {\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"length\"}]}\n\n").unwrap();
        assert!(!stream.terminal());
        stream.frame(b"data: [DONE]\n\n").unwrap();
        assert!(stream.terminal());
        assert_eq!(
            stream.frame(b"data: {\"error\":{\"message\":\"late error\"}}\n\n"),
            Err("PROBE_UPSTREAM_ERROR")
        );
    }

    #[test]
    fn anthropic_stream_requires_text_block_stop_reason_and_message_stop() {
        let mut stream = ProbeStream::new(ProbeProtocol::Anthropic);
        for frame in [
            "event: message_start\ndata: {\"message\":{\"type\":\"message\",\"role\":\"assistant\"}}\n\n",
            "event: content_block_start\ndata: {\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"OK\"}}\n\n",
            "event: content_block_stop\ndata: {\"index\":0}\n\n",
            "event: message_delta\ndata: {\"delta\":{\"stop_reason\":\"max_tokens\"}}\n\n",
        ] { stream.frame(frame.as_bytes()).unwrap(); }
        assert!(!stream.terminal());
        stream
            .frame(b"event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n")
            .unwrap();
        assert!(stream.terminal());
    }

    #[test]
    fn streaming_errors_and_gemini_wrapper_do_not_become_answers() {
        for frame in [
            b"data: {broken}\n\n".as_slice(),
            b"event: error\ndata: {}\n\n",
            b"data: [DONE]\n\n",
        ] {
            assert!(ProbeStream::new(ProbeProtocol::Anthropic)
                .frame(frame)
                .is_err());
        }
        let mut stream = ProbeStream::new(ProbeProtocol::Gemini);
        gemini_oauth_frame(&mut stream, b"data: {\"response\":{\"candidates\":[{\"index\":0,\"finishReason\":\"MAX_TOKENS\",\"content\":{\"parts\":[{\"text\":\"OK\"}]}}]}}\n\n").unwrap();
        assert!(stream.terminal());
        assert!(gemini_oauth_frame(&mut stream, b"data: {\"error\":\"failed\"}\n\n").is_err());
    }

    #[test]
    fn answer_fields_cannot_hide_errors_or_invalid_structure() {
        let cases = [
            (
                ProbeProtocol::Responses,
                json!({"object":"response","status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OK","error":"failed"}]}]}),
            ),
            (
                ProbeProtocol::Responses,
                json!({"object":"response","status":"completed","output":[{"type":"reasoning","summary":[{"text":"OK"}]}]}),
            ),
            (
                ProbeProtocol::Chat,
                json!({"object":"chat.completion","choices":[{"message":{"role":"assistant","content":null,"tool_calls":[{"function":{"arguments":"OK"}}]},"finish_reason":"stop"}]}),
            ),
            (
                ProbeProtocol::Chat,
                json!({"object":"chat.completion","choices":[{"message":{"role":"assistant","content":"OK","error":"failed"},"finish_reason":"stop"}]}),
            ),
            (
                ProbeProtocol::Anthropic,
                json!({"type":"message","role":"assistant","content":[{"type":"thinking","thinking":"OK"}],"stop_reason":"max_tokens"}),
            ),
            (
                ProbeProtocol::Anthropic,
                json!({"type":"message","role":"assistant","content":[{"type":"text","text":42}],"stop_reason":"end_turn"}),
            ),
            (
                ProbeProtocol::Gemini,
                json!({"candidates":[{"finishReason":"MAX_TOKENS","content":{"parts":[{"functionCall":{"args":{"text":"OK"}}}]}}]}),
            ),
            (
                ProbeProtocol::Gemini,
                json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"text":"OK","error":"failed"}]}}]}),
            ),
        ];
        for (protocol, value) in cases {
            assert!(
                validate_json(protocol, &value).is_err(),
                "{protocol:?}: {value}"
            );
        }
        assert_eq!(
            gemini_oauth_response(&json!({"error":"failed","response":{"candidates":[]}})),
            Err("PROBE_UPSTREAM_ERROR")
        );
        assert_eq!(
            gemini_oauth_response(&json!({"candidates":[]})),
            Err("PROBE_STRUCTURE")
        );
    }

    #[test]
    fn streams_reject_text_then_errors_missing_terminals_and_token_only_output() {
        let mut responses_stream = ProbeStream::new(ProbeProtocol::Responses);
        responses_stream.frame(b"data: {\"type\":\"response.output_text.delta\",\"response_id\":\"resp_1\",\"delta\":\"OK\"}\n\n").unwrap();
        assert!(!responses_stream.terminal());
        assert_eq!(
            responses_stream.frame(
                b"data: {\"type\":\"response.failed\",\"response\":{\"error\":\"failed\"}}\n\n"
            ),
            Err("PROBE_UPSTREAM_ERROR")
        );
        for status in ["completed", "incomplete"] {
            let response = responses(status, " ");
            assert_eq!(
                ProbeStream::new(ProbeProtocol::Responses).frame(
                    format!("event: response.{status}\ndata: {{\"response\":{response}}}\n\n")
                        .as_bytes()
                ),
                Err("PROBE_NO_TEXT")
            );
        }
        let mut chat = ProbeStream::new(ProbeProtocol::Chat);
        chat.frame(b"data: {\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"reasoning_content\":\"OK\"},\"finish_reason\":\"length\"}]}\n\n").unwrap();
        assert_eq!(chat.frame(b"data: [DONE]\n\n"), Err("PROBE_NO_TEXT"));
        let mut gemini = ProbeStream::new(ProbeProtocol::Gemini);
        gemini.frame(b"data: {\"candidates\":[{\"index\":0,\"content\":{\"parts\":[{\"text\":\"OK\"}]}}]}\n\n").unwrap();
        assert!(!gemini.terminal());
        assert_eq!(
            gemini.frame(b"data: {\"candidates\":[{\"index\":0,\"finishReason\":\"SAFETY\"}]}\n\n"),
            Err("PROBE_TERMINATION")
        );
    }

    #[test]
    fn independent_choices_cannot_combine_answer_and_termination() {
        let chat = json!({"object":"chat.completion","choices":[
            {"message":{"role":"assistant","content":"OK"},"finish_reason":null},
            {"message":{"role":"assistant","content":""},"finish_reason":"stop"}]});
        assert_eq!(
            validate_json(ProbeProtocol::Chat, &chat),
            Err("PROBE_UNFINISHED")
        );
        let gemini = json!({"candidates":[
            {"content":{"parts":[{"text":"OK"}]}},
            {"content":{"parts":[]},"finishReason":"STOP"}]});
        assert_eq!(
            validate_json(ProbeProtocol::Gemini, &gemini),
            Err("PROBE_UNFINISHED")
        );
    }
}
