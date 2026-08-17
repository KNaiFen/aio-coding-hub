//! Bounded, frame-aware rewriting for native Codex Responses overload errors.

use axum::body::Bytes;

const MAX_PENDING_FRAME_BYTES: usize = 1024 * 1024;
const TARGET_ERROR_CODES: [&str; 2] = ["server_is_overloaded", "slow_down"];
const REWRITTEN_ERROR_CODE: &str = "server_error";

pub(super) struct CodexResponsesOverloadErrorRewriter {
    pending: Vec<u8>,
    bypass: bool,
}

impl CodexResponsesOverloadErrorRewriter {
    pub(super) fn new() -> Self {
        Self {
            pending: Vec::new(),
            bypass: false,
        }
    }

    pub(super) fn ingest(&mut self, chunk: Bytes) -> Vec<Bytes> {
        if chunk.is_empty() {
            return Vec::new();
        }
        if self.bypass {
            return vec![chunk];
        }

        let mut output = Vec::new();
        let mut input = chunk.as_ref();

        loop {
            if let Some(event_end) =
                crate::gateway::proxy::sse::find_sse_event_end(self.pending.as_slice())
            {
                let mut buffered = std::mem::take(&mut self.pending);
                let tail = buffered.split_off(event_end);
                output.push(rewrite_frame(buffered.as_slice()));
                self.pending = tail;
                continue;
            }

            if input.is_empty() {
                break;
            }

            if self.pending.is_empty() {
                if let Some(event_end) = crate::gateway::proxy::sse::find_sse_event_end(input) {
                    if event_end > MAX_PENDING_FRAME_BYTES {
                        output.push(Bytes::copy_from_slice(input));
                        self.bypass = true;
                        break;
                    }
                    output.push(rewrite_frame(&input[..event_end]));
                    input = &input[event_end..];
                    continue;
                }

                if input.len() > MAX_PENDING_FRAME_BYTES {
                    output.push(Bytes::copy_from_slice(input));
                    self.bypass = true;
                } else {
                    self.pending.extend_from_slice(input);
                }
                break;
            }

            let remaining_capacity = MAX_PENDING_FRAME_BYTES.saturating_sub(self.pending.len());
            if remaining_capacity == 0 {
                self.fail_open(&mut output, input);
                break;
            }
            let take = remaining_capacity.min(input.len());
            self.pending.extend_from_slice(&input[..take]);
            input = &input[take..];

            if crate::gateway::proxy::sse::find_sse_event_end(self.pending.as_slice()).is_none()
                && self.pending.len() == MAX_PENDING_FRAME_BYTES
            {
                self.fail_open(&mut output, input);
                break;
            }
        }

        output
    }

    pub(super) fn finish(&mut self) -> Option<Bytes> {
        (!self.pending.is_empty()).then(|| Bytes::from(std::mem::take(&mut self.pending)))
    }

    fn fail_open(&mut self, output: &mut Vec<Bytes>, remaining: &[u8]) {
        if !self.pending.is_empty() {
            output.push(Bytes::from(std::mem::take(&mut self.pending)));
        }
        if !remaining.is_empty() {
            output.push(Bytes::copy_from_slice(remaining));
        }
        self.bypass = true;
    }
}

fn rewrite_frame(frame: &[u8]) -> Bytes {
    let Ok(text) = std::str::from_utf8(frame) else {
        return Bytes::copy_from_slice(frame);
    };
    let Some((event_name, mut data)) = crate::gateway::proxy::sse::parse_sse_frame(text) else {
        return Bytes::copy_from_slice(frame);
    };
    if event_name != "response.failed" {
        return Bytes::copy_from_slice(frame);
    }

    let Some(code) = data
        .pointer("/response/error/code")
        .and_then(serde_json::Value::as_str)
    else {
        return Bytes::copy_from_slice(frame);
    };
    if !TARGET_ERROR_CODES.contains(&code) {
        return Bytes::copy_from_slice(frame);
    }
    let Some(code) = data.pointer_mut("/response/error/code") else {
        return Bytes::copy_from_slice(frame);
    };
    *code = serde_json::Value::String(REWRITTEN_ERROR_CODE.to_string());
    let Ok(json) = serde_json::to_string(&data) else {
        return Bytes::copy_from_slice(frame);
    };

    serialize_rewritten_frame(text, json.as_str())
        .map(Bytes::from)
        .unwrap_or_else(|| Bytes::copy_from_slice(frame))
}

fn serialize_rewritten_frame(frame: &str, json: &str) -> Option<Vec<u8>> {
    let (line_ending, terminator) = if frame.ends_with("\r\n\r\n") {
        ("\r\n", "\r\n\r\n")
    } else if frame.ends_with("\n\n") {
        ("\n", "\n\n")
    } else {
        return None;
    };
    let body = frame.strip_suffix(terminator)?;
    let mut lines = Vec::new();
    let mut replaced_data = false;
    for line in body.split(line_ending) {
        if line.starts_with("data:") {
            if !replaced_data {
                lines.push(format!("data: {json}"));
                replaced_data = true;
            }
        } else {
            lines.push(line.to_string());
        }
    }
    if !replaced_data {
        return None;
    }

    let mut rewritten = lines.join(line_ending).into_bytes();
    rewritten.extend_from_slice(terminator.as_bytes());
    Some(rewritten)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_frame(code: &str, line_ending: &str) -> String {
        format!(
            "event: response.failed{line_ending}id: evt-1{line_ending}data: {{\"type\":\"response.failed\",\"response\":{{\"id\":\"resp-1\",\"error\":{{\"code\":\"{code}\",\"message\":\"busy\"}}}}}}{line_ending}{line_ending}"
        )
    }

    fn rewrite_chunks(chunks: &[&[u8]]) -> Vec<u8> {
        let mut rewriter = CodexResponsesOverloadErrorRewriter::new();
        let mut output = Vec::new();
        for chunk in chunks {
            for rewritten in rewriter.ingest(Bytes::copy_from_slice(chunk)) {
                output.extend_from_slice(rewritten.as_ref());
            }
        }
        if let Some(tail) = rewriter.finish() {
            output.extend_from_slice(tail.as_ref());
        }
        output
    }

    #[test]
    fn rewrites_both_target_codes_and_preserves_other_fields() {
        for code in TARGET_ERROR_CODES {
            let input = target_frame(code, "\n");
            let output = rewrite_chunks(&[input.as_bytes()]);
            let text = std::str::from_utf8(&output).expect("rewritten utf8");
            assert!(text.contains("event: response.failed\nid: evt-1\n"));
            assert!(text.contains("\"code\":\"server_error\""));
            assert!(text.contains("\"message\":\"busy\""));
            assert!(text.ends_with("\n\n"));
        }
    }

    #[test]
    fn handles_chunk_boundaries_crlf_and_multiple_frames() {
        let target = target_frame("slow_down", "\r\n");
        let untouched = "event: response.completed\r\ndata: {\"type\":\"response.completed\"}\r\n\r\n";
        let input = format!(": keepalive\r\n\r\n{target}{untouched}");
        let split = input.len() / 2;
        let output = rewrite_chunks(&[&input.as_bytes()[..split], &input.as_bytes()[split..]]);
        let text = std::str::from_utf8(&output).expect("rewritten utf8");
        assert!(text.starts_with(": keepalive\r\n\r\n"));
        assert!(text.contains("\"code\":\"server_error\""));
        assert!(text.ends_with(untouched));
    }

    #[test]
    fn leaves_all_non_targets_byte_exact() {
        let cases = [
            "event: response.error\ndata: {\"response\":{\"error\":{\"code\":\"slow_down\"}}}\n\n",
            "event: response.failed\ndata: {\"error\":{\"code\":\"slow_down\"}}\n\n",
            "event: response.failed\ndata: {\"response\":{\"error\":{\"code\":\"other\"}}}\n\n",
            "event: response.failed\ndata: not-json\n\n",
        ];
        for input in cases {
            assert_eq!(rewrite_chunks(&[input.as_bytes()]), input.as_bytes());
        }

        let invalid_utf8 = b"event: response.failed\ndata: \xff\n\n";
        assert_eq!(rewrite_chunks(&[invalid_utf8]), invalid_utf8);
    }

    #[test]
    fn oversized_frame_and_following_data_fail_open_without_loss() {
        let oversized = vec![b'x'; MAX_PENDING_FRAME_BYTES + 1];
        let target = target_frame("server_is_overloaded", "\n");
        let mut rewriter = CodexResponsesOverloadErrorRewriter::new();
        let mut output = Vec::new();
        for item in rewriter.ingest(Bytes::from(oversized.clone())) {
            output.extend_from_slice(item.as_ref());
        }
        for item in rewriter.ingest(Bytes::copy_from_slice(target.as_bytes())) {
            output.extend_from_slice(item.as_ref());
        }

        let mut expected = oversized;
        expected.extend_from_slice(target.as_bytes());
        assert_eq!(output, expected);
        assert!(rewriter.finish().is_none());
    }

    #[test]
    fn unterminated_tail_is_flushed_byte_exact_at_eof() {
        let tail = b"event: response.failed\ndata: {\"response\":{\"error\":{\"code\":\"slow_down\"}}}";
        assert_eq!(rewrite_chunks(&[tail]), tail);
    }
}
