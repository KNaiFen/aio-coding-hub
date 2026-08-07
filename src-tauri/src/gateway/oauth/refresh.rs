//! Usage: Background token refresh loop for OAuth providers.

use super::provider_trait::OAuthTokenSet;
use super::token_exchange::{refresh_access_token, TokenRefreshRequest};
use crate::blocking;
use crate::shared::time::now_unix_seconds;

const REFRESH_LINEAR_RETRY_MAX_ATTEMPTS: u32 = 3;
const REFRESH_LINEAR_RETRY_BASE_DELAY_SECS: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SavedOAuthTokenError {
    Invalid,
    Timeout,
    Network,
    Unauthorized,
}

pub(crate) fn should_refresh_now(expires_at: Option<i64>, refresh_lead_s: i64) -> bool {
    let Some(expires_at) = expires_at else {
        // Unknown expiry → assume the token needs refreshing now so we don't
        // silently serve a potentially-expired token forever.
        return true;
    };
    let now = now_unix_seconds();
    now >= (expires_at - refresh_lead_s)
}

pub(crate) async fn refresh_provider_token_with_retry(
    client: &reqwest::Client,
    token_uri: &str,
    client_id: &str,
    client_secret: Option<&str>,
    refresh_token: &str,
) -> Result<OAuthTokenSet, String> {
    let req = TokenRefreshRequest {
        token_uri: token_uri.to_string(),
        client_id: client_id.to_string(),
        client_secret: client_secret.map(str::to_string),
        refresh_token: refresh_token.to_string(),
    };

    let mut last_err = String::new();
    for attempt in 0..REFRESH_LINEAR_RETRY_MAX_ATTEMPTS {
        match refresh_access_token(client, &req).await {
            Ok(token_set) => return Ok(token_set),
            Err(e) => {
                if e.starts_with("AUTH_RELOGIN_REQUIRED") {
                    return Err(e);
                }
                last_err = e;
                if attempt + 1 < REFRESH_LINEAR_RETRY_MAX_ATTEMPTS {
                    let delay = REFRESH_LINEAR_RETRY_BASE_DELAY_SECS * (attempt as u64 + 1);
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    }

    Err(format!(
        "token refresh failed after {REFRESH_LINEAR_RETRY_MAX_ATTEMPTS} attempts: {last_err}"
    ))
}

/// Resolve a saved OAuth credential using the same proactive-refresh boundary
/// as gateway requests. The caller owns the request deadline; refresh and CAS
/// persistence consume that same budget.
pub(crate) async fn resolve_saved_oauth_token(
    db: &crate::db::Db,
    client: &reqwest::Client,
    details: &crate::providers::ProviderOAuthDetails,
    adapter: &'static dyn super::provider_trait::OAuthProvider,
    deadline: tokio::time::Instant,
) -> Result<OAuthTokenSet, SavedOAuthTokenError> {
    let current = OAuthTokenSet {
        access_token: details.oauth_access_token.clone(),
        refresh_token: details.oauth_refresh_token.clone(),
        expires_at: details.oauth_expires_at,
        id_token: details.oauth_id_token.clone(),
    };
    let (current_token, current_id_token) =
        adapter.resolve_effective_token(&current, details.oauth_id_token.as_deref());
    let current_token = current_token.trim().to_string();
    if current_token.is_empty() {
        return Err(SavedOAuthTokenError::Invalid);
    }

    let current_effective = || OAuthTokenSet {
        access_token: current_token.clone(),
        refresh_token: current.refresh_token.clone(),
        expires_at: current.expires_at,
        id_token: current_id_token.clone(),
    };
    if !should_refresh_now(details.oauth_expires_at, details.oauth_refresh_lead_s) {
        return Ok(current_effective());
    }

    let now = now_unix_seconds();
    let still_valid = details
        .oauth_expires_at
        .map(|expires_at| expires_at > now)
        .unwrap_or(false);
    let Some(refresh_token) = details
        .oauth_refresh_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return if still_valid {
            Ok(current_effective())
        } else {
            Err(SavedOAuthTokenError::Unauthorized)
        };
    };
    let Some(token_uri) = details
        .oauth_token_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return if still_valid {
            Ok(current_effective())
        } else {
            Err(SavedOAuthTokenError::Invalid)
        };
    };
    let client_id = details
        .oauth_client_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(SavedOAuthTokenError::Invalid)?;

    if deadline <= tokio::time::Instant::now() {
        return Err(SavedOAuthTokenError::Timeout);
    }
    let refreshed = match tokio::time::timeout_at(
        deadline,
        refresh_provider_token_with_retry(
            client,
            token_uri,
            client_id,
            details.oauth_client_secret.as_deref(),
            refresh_token,
        ),
    )
    .await
    {
        Ok(Ok(token_set)) => token_set,
        Ok(Err(error)) => {
            if still_valid {
                return Ok(current_effective());
            }
            if error.starts_with("AUTH_RELOGIN_REQUIRED")
                || error.contains("401 Unauthorized")
                || error.contains("403 Forbidden")
            {
                return Err(SavedOAuthTokenError::Unauthorized);
            }
            return Err(SavedOAuthTokenError::Network);
        }
        Err(_) => return Err(SavedOAuthTokenError::Timeout),
    };

    let (refreshed_token, refreshed_id_token) =
        adapter.resolve_effective_token(&refreshed, details.oauth_id_token.as_deref());
    let refreshed_token = refreshed_token.trim().to_string();
    if refreshed_token.is_empty() {
        return Err(SavedOAuthTokenError::Unauthorized);
    }

    let provider_id = details.id;
    let provider_type = if details.oauth_provider_type.trim().is_empty() {
        adapter.provider_type().to_string()
    } else {
        details.oauth_provider_type.clone()
    };
    let expires_at = refreshed.expires_at.or(details.oauth_expires_at);
    let refresh_token_to_save = refreshed.refresh_token.as_deref().or(Some(refresh_token));
    let id_token_to_save = refreshed_id_token.as_deref();
    let persisted = tokio::time::timeout_at(
        deadline,
        blocking::run("provider_oauth_discovery_refresh_save", {
            let db = db.clone();
            let provider_type = provider_type.clone();
            let refreshed_token = refreshed_token.clone();
            let refresh_token_to_save = refresh_token_to_save.map(str::to_string);
            let id_token_to_save = id_token_to_save.map(str::to_string);
            let token_uri = token_uri.to_string();
            let client_id = client_id.to_string();
            let client_secret = details.oauth_client_secret.clone();
            let email = details.oauth_email.clone();
            let expected_last_refreshed_at = details.oauth_last_refreshed_at;
            move || {
                crate::providers::update_oauth_tokens_if_last_refreshed_matches(
                    &db,
                    provider_id,
                    "oauth",
                    &provider_type,
                    &refreshed_token,
                    refresh_token_to_save.as_deref(),
                    id_token_to_save.as_deref(),
                    &token_uri,
                    &client_id,
                    client_secret.as_deref(),
                    expires_at,
                    email.as_deref(),
                    expected_last_refreshed_at,
                )
            }
        }),
    )
    .await;
    match persisted {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            tracing::warn!(
                provider_id,
                "provider model discovery OAuth refresh CAS did not persist: {error}"
            );
        }
        Err(_) => {
            tracing::warn!(
                provider_id,
                "provider model discovery OAuth refresh CAS timed out"
            );
        }
    }

    Ok(OAuthTokenSet {
        access_token: refreshed_token,
        refresh_token: refreshed.refresh_token,
        expires_at,
        id_token: refreshed_id_token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::oauth::registry::resolve_oauth_adapter_for_details;
    use crate::providers::{ProviderAuthMode, ProviderBaseUrlMode, ProviderUpsertParams};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn token_fixture(
        status: &str,
        response_body: &str,
    ) -> (String, tokio::task::JoinHandle<String>) {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind token fixture");
        let address = listener.local_addr().expect("token fixture address");
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept token request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let read = stream.read(&mut buffer).await.expect("read token request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write token response");
            String::from_utf8_lossy(&request).into_owned()
        });
        (format!("http://{address}/token"), task)
    }

    fn oauth_provider_params(name: &str) -> ProviderUpsertParams {
        ProviderUpsertParams {
            provider_id: None,
            cli_key: "codex".to_string(),
            name: name.to_string(),
            base_urls: vec![],
            base_url_mode: ProviderBaseUrlMode::Order,
            auth_mode: Some(ProviderAuthMode::Oauth),
            api_key: None,
            enabled: true,
            cost_multiplier: 1.0,
            priority: Some(100),
            claude_models: None,
            model_policy: None,
            limit_5h_usd: None,
            limit_daily_usd: None,
            daily_reset_mode: None,
            daily_reset_time: None,
            limit_weekly_usd: None,
            limit_monthly_usd: None,
            limit_total_usd: None,
            tags: None,
            note: None,
            source_provider_id: None,
            bridge_type: None,
            stream_idle_timeout_seconds: None,
            extension_values: None,
        }
    }

    fn seed_oauth_provider(
        db: &crate::db::Db,
        name: &str,
        token_uri: &str,
        expires_at: i64,
    ) -> crate::providers::ProviderOAuthDetails {
        let provider = crate::providers::upsert(db, oauth_provider_params(name))
            .expect("create OAuth provider");
        crate::providers::update_oauth_tokens(
            db,
            provider.id,
            "oauth",
            "codex_oauth",
            "old-access",
            Some("old-refresh"),
            Some("old-id"),
            token_uri,
            "client-id",
            None,
            Some(expires_at),
            Some("oauth@example.com"),
        )
        .expect("seed OAuth tokens");
        crate::providers::get_oauth_details(db, provider.id).expect("load OAuth details")
    }

    #[tokio::test]
    async fn resolve_saved_oauth_token_uses_refreshed_tokens_and_preserves_expiry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("oauth-discovery-refresh.db"))
            .expect("init test db");
        let (token_uri, request_task) = token_fixture(
            "200 OK",
            r#"{"access_token":"new-access","refresh_token":"new-refresh","id_token":"new-id"}"#,
        )
        .await;
        let old_expiry = now_unix_seconds() - 10;
        let details = seed_oauth_provider(&db, "oauth-discovery", &token_uri, old_expiry);
        let adapter = resolve_oauth_adapter_for_details(&details).expect("resolve adapter");
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build test client");

        let resolved = resolve_saved_oauth_token(
            &db,
            &client,
            &details,
            adapter,
            tokio::time::Instant::now() + std::time::Duration::from_secs(5),
        )
        .await
        .expect("resolve refreshed token");

        assert_eq!(resolved.access_token, "new-access");
        assert_eq!(resolved.refresh_token.as_deref(), Some("new-refresh"));
        assert_eq!(resolved.id_token.as_deref(), Some("new-id"));
        assert_eq!(resolved.expires_at, Some(old_expiry));
        let request = request_task.await.expect("token fixture request");
        assert!(request.starts_with("POST /token HTTP/1.1"));
        assert!(request.contains("refresh_token=old-refresh"));

        let saved =
            crate::providers::get_oauth_details(&db, details.id).expect("reload OAuth details");
        assert_eq!(saved.oauth_access_token, "new-access");
        assert_eq!(saved.oauth_refresh_token.as_deref(), Some("new-refresh"));
        assert_eq!(saved.oauth_id_token.as_deref(), Some("new-id"));
        assert_eq!(saved.oauth_expires_at, Some(old_expiry));
    }

    #[tokio::test]
    async fn resolve_saved_oauth_token_maps_invalid_grant_to_unauthorized() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = crate::db::init_for_tests(&dir.path().join("oauth-discovery-invalid-grant.db"))
            .expect("init test db");
        let (token_uri, request_task) = token_fixture(
            "400 Bad Request",
            r#"{"error":"invalid_grant","error_description":"refresh_token expired"}"#,
        )
        .await;
        let details = seed_oauth_provider(
            &db,
            "oauth-invalid-grant",
            &token_uri,
            now_unix_seconds() - 10,
        );
        let adapter = resolve_oauth_adapter_for_details(&details).expect("resolve adapter");
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build test client");

        let error = resolve_saved_oauth_token(
            &db,
            &client,
            &details,
            adapter,
            tokio::time::Instant::now() + std::time::Duration::from_secs(5),
        )
        .await
        .expect_err("invalid grant must require reauthentication");

        assert_eq!(error, SavedOAuthTokenError::Unauthorized);
        let request = request_task.await.expect("token fixture request");
        assert!(request.starts_with("POST /token HTTP/1.1"));
    }
}
