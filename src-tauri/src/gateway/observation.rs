//! Read-only gateway projections used by monitoring surfaces.

fn normalized_request_path(path: &str) -> String {
    let without_query = path.trim().split('?').next().unwrap_or_default();
    let normalized = without_query.trim_end_matches('/').to_ascii_lowercase();
    if normalized.is_empty() {
        "/".to_string()
    } else {
        normalized
    }
}

pub(crate) fn is_model_inference_request(cli_key: &str, method: &str, path: &str) -> bool {
    if !method.trim().eq_ignore_ascii_case("POST") {
        return false;
    }

    let cli_key = cli_key.trim().to_ascii_lowercase();
    let path = normalized_request_path(path);
    match cli_key.as_str() {
        "claude" => matches!(path.as_str(), "/v1/messages" | "/messages"),
        "codex" => matches!(
            path.as_str(),
            "/responses"
                | "/v1/responses"
                | "/v1/codex/responses"
                | "/responses/compact"
                | "/v1/responses/compact"
                | "/v1/codex/responses/compact"
        ),
        "grok" => matches!(
            path.as_str(),
            "/chat/completions" | "/v1/chat/completions" | "/responses" | "/v1/responses"
        ),
        "gemini" => path.ends_with(":generatecontent") || path.ends_with(":streamgeneratecontent"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_model_inference_request;

    #[test]
    fn matches_home_inference_endpoint_contract() {
        for (cli, path) in [
            ("claude", "/v1/messages"),
            ("claude", "/messages/"),
            ("codex", "/v1/responses"),
            ("codex", "/v1/codex/responses/compact?x=1"),
            ("grok", "/v1/chat/completions"),
            ("grok", "/responses"),
            ("gemini", "/v1beta/models/gemini:generateContent"),
            ("gemini", "/v1beta/models/gemini:streamGenerateContent"),
        ] {
            assert!(
                is_model_inference_request(cli, "POST", path),
                "{cli} {path}"
            );
        }
    }

    #[test]
    fn excludes_auxiliary_and_non_post_requests() {
        for (cli, method, path) in [
            ("codex", "GET", "/v1/responses"),
            ("codex", "GET", "/v1/models"),
            ("codex", "POST", "/v1/alpha/search"),
            ("claude", "POST", "/v1/messages/count_tokens"),
            ("gemini", "POST", "/v1beta/models"),
        ] {
            assert!(!is_model_inference_request(cli, method, path));
        }
    }

    #[test]
    fn parallel_parent_and_subagent_requests_are_counted_individually() {
        let mut requests = vec![("codex", "POST", "/v1/responses"); 3];
        requests.extend(vec![("codex", "POST", "/v1/responses"); 10]);
        let count = requests
            .iter()
            .filter(|(cli, method, path)| is_model_inference_request(cli, method, path))
            .count();
        assert_eq!(count, 13);

        requests.truncate(11);
        let count = requests
            .iter()
            .filter(|(cli, method, path)| is_model_inference_request(cli, method, path))
            .count();
        assert_eq!(count, 11);
    }
}
