//! Usage: Apply a ready Provider model policy to one upstream attempt.

use super::context::{CommonCtx, ProviderCtx};
use crate::gateway::events::ModelRedirect;
use crate::gateway::proxy::model_rewrite::{
    replace_model_in_body_json, replace_model_in_path, replace_model_in_query,
};
use crate::gateway::util::RequestedModelLocation;
use crate::providers;
use axum::body::Bytes;

pub(super) struct UpstreamRequestMut<'a> {
    pub(super) forwarded_path: &'a mut String,
    pub(super) query: &'a mut Option<String>,
    pub(super) body_bytes: &'a mut Bytes,
    pub(super) strip_request_content_encoding: &'a mut bool,
}

pub(super) fn resolve_target_model(
    provider: &providers::ProviderForGateway,
    requested_model: Option<&str>,
) -> Option<String> {
    if provider.model_policy_status != providers::ProviderModelPolicyStatus::Ready {
        return None;
    }
    let requested_model = requested_model?;
    let effective_model = provider
        .model_policy
        .as_ref()?
        .resolve_mapping(requested_model);
    (effective_model != requested_model).then_some(effective_model)
}

pub(super) fn apply_if_needed<R: tauri::Runtime>(
    ctx: CommonCtx<'_, R>,
    provider: &providers::ProviderForGateway,
    provider_ctx: ProviderCtx<'_>,
    requested_model_location: Option<RequestedModelLocation>,
    effective_model: Option<&str>,
    model_already_applied: bool,
    upstream: UpstreamRequestMut<'_>,
) -> Option<ModelRedirect> {
    let requested_model = ctx.requested_model.as_deref()?;
    let effective_model = effective_model?;

    let UpstreamRequestMut {
        forwarded_path,
        query,
        body_bytes,
        strip_request_content_encoding,
    } = upstream;
    if !model_already_applied {
        let location = requested_model_location.unwrap_or(RequestedModelLocation::BodyJson);
        if !rewrite_model(
            location,
            effective_model,
            forwarded_path,
            query,
            body_bytes,
            strip_request_content_encoding,
        ) {
            return None;
        }
    }

    let stage = if provider.is_cx2cc_bridge() {
        "bridge"
    } else {
        "provider"
    };
    let redirect = ModelRedirect {
        stage: stage.to_string(),
        provider_id: provider_ctx.provider_id,
        provider_name: provider_ctx.provider_name_base.clone(),
        source_model: requested_model.to_string(),
        target_model: effective_model.to_string(),
    };
    crate::gateway::response_fixer::push_special_setting(
        ctx.special_settings,
        serde_json::json!({
            "type": "model_redirect",
            "scope": "attempt",
            "hit": true,
            "stage": stage,
            "providerId": provider_ctx.provider_id,
            "providerName": provider_ctx.provider_name_base,
            "sourceModel": requested_model,
            "targetModel": effective_model,
        }),
    );
    Some(redirect)
}

fn rewrite_model(
    location: RequestedModelLocation,
    effective_model: &str,
    forwarded_path: &mut String,
    query: &mut Option<String>,
    body_bytes: &mut Bytes,
    strip_request_content_encoding: &mut bool,
) -> bool {
    match location {
        RequestedModelLocation::BodyJson => {
            let Ok(mut root) = serde_json::from_slice::<serde_json::Value>(body_bytes) else {
                return false;
            };
            if !replace_model_in_body_json(&mut root, effective_model) {
                return false;
            }
            let Ok(bytes) = serde_json::to_vec(&root) else {
                return false;
            };
            *body_bytes = Bytes::from(bytes);
            *strip_request_content_encoding = true;
            true
        }
        RequestedModelLocation::Query => {
            let Some(current) = query.as_deref() else {
                return false;
            };
            let next = replace_model_in_query(current, effective_model);
            let changed = next != current;
            if changed {
                *query = Some(next);
            }
            changed
        }
        RequestedModelLocation::Path => {
            let Some(next) = replace_model_in_path(forwarded_path, effective_model) else {
                return false;
            };
            let changed = next != *forwarded_path;
            if changed {
                *forwarded_path = next;
            }
            changed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rewrite_model;
    use crate::gateway::util::RequestedModelLocation;
    use axum::body::Bytes;

    #[test]
    fn rewrite_model_updates_body_query_and_path() {
        let mut path = "/v1/models/gpt-5.4".to_string();
        let mut query = Some("model=gpt-5.4&stream=true".to_string());
        let mut body = Bytes::from(r#"{"model":"gpt-5.4","input":[]}"#);
        let mut strip = false;

        assert!(rewrite_model(
            RequestedModelLocation::BodyJson,
            "upstream-5.4",
            &mut path,
            &mut query,
            &mut body,
            &mut strip,
        ));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body)
                .expect("rewritten body")
                .get("model")
                .and_then(|value| value.as_str()),
            Some("upstream-5.4")
        );
        assert!(strip);

        assert!(rewrite_model(
            RequestedModelLocation::Query,
            "query-model",
            &mut path,
            &mut query,
            &mut body,
            &mut strip,
        ));
        assert_eq!(query.as_deref(), Some("model=query-model&stream=true"));

        assert!(rewrite_model(
            RequestedModelLocation::Path,
            "path-model",
            &mut path,
            &mut query,
            &mut body,
            &mut strip,
        ));
        assert_eq!(path, "/v1/models/path-model");
    }
}
