//! Private lifecycle and verifier for non-loopback gateway bearer tokens.

use crate::shared::mutex_ext::MutexExt;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use rand::RngCore as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use tauri::Manager as _;

const SIDECAR_FILE_NAME: &str = "gateway-bearer-token.json";
const SIDECAR_SCHEMA_VERSION: u32 = 1;
const SIDECAR_MAX_BYTES: usize = 4 * 1024;
const TOKEN_BYTES: usize = 32;
const TOKEN_ENCODED_LEN: usize = 43;

#[derive(Clone, Default)]
pub(crate) struct GatewayAccessControl {
    digest: Arc<RwLock<Option<[u8; TOKEN_BYTES]>>>,
}

impl std::fmt::Debug for GatewayAccessControl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GatewayAccessControl")
            .field("configured", &self.configured())
            .finish()
    }
}

impl GatewayAccessControl {
    pub(crate) fn configured(&self) -> bool {
        self.digest
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    pub(crate) fn verify(&self, token: &str) -> bool {
        if !is_strict_generated_token(token) {
            return false;
        }
        let actual = sha256(token.as_bytes());
        let expected = self
            .digest
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        expected
            .as_ref()
            .is_some_and(|expected| crate::shared::security::constant_time_eq(expected, &actual))
    }

    fn replace_digest(&self, digest: Option<[u8; TOKEN_BYTES]>) {
        *self
            .digest
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = digest;
    }

    #[cfg(test)]
    pub(crate) fn from_token_for_tests(token: &str) -> Self {
        let control = Self::default();
        control.replace_digest(Some(sha256(token.as_bytes())));
        control
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedGatewayBearerToken {
    schema_version: u32,
    token_sha256: String,
    generation: u64,
    confirmed: bool,
}

impl PersistedGatewayBearerToken {
    fn decoded_digest(&self) -> Result<[u8; TOKEN_BYTES], String> {
        if self.schema_version != SIDECAR_SCHEMA_VERSION || self.generation == 0 {
            return Err(
                "GATEWAY_BEARER_STATE_INVALID: unsupported private token state".to_string(),
            );
        }
        let decoded = URL_SAFE_NO_PAD
            .decode(self.token_sha256.as_bytes())
            .map_err(|_| "GATEWAY_BEARER_STATE_INVALID: invalid token digest".to_string())?;
        decoded
            .try_into()
            .map_err(|_| "GATEWAY_BEARER_STATE_INVALID: invalid token digest length".to_string())
    }
}

#[derive(Clone)]
struct PendingGatewayBearerToken {
    generation: u64,
    plaintext: String,
    wsl_sync_error: Option<String>,
}

#[derive(Default)]
struct GatewayBearerTokenRuntime {
    initialized: bool,
    persisted: Option<PersistedGatewayBearerToken>,
    owned_unconfirmed_generation: Option<u64>,
    revealed_generation: Option<u64>,
    pending: Option<PendingGatewayBearerToken>,
}

#[derive(Default)]
pub(crate) struct GatewayBearerTokenState {
    access: GatewayAccessControl,
    runtime: Mutex<GatewayBearerTokenRuntime>,
}

#[derive(Clone, Serialize, specta::Type)]
pub(crate) struct GatewayBearerTokenReveal {
    pub token: String,
    pub wsl_sync_error: Option<String>,
}

pub(crate) fn access_control<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> GatewayAccessControl {
    app.state::<GatewayBearerTokenState>().access.clone()
}

pub(crate) fn ensure_for_settings<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    settings: &crate::settings::AppSettings,
) -> crate::shared::error::AppResult<bool> {
    let requires_token = super::listener_accepts_non_loopback(settings)?;
    let path = sidecar_path(app)?;
    let state = app.state::<GatewayBearerTokenState>();
    let mut runtime = state.runtime.lock_or_recover();
    initialize_locked(&state.access, &mut runtime, &path, requires_token)?;

    let needs_rotation = requires_token
        && runtime.persisted.as_ref().is_none_or(|persisted| {
            !persisted.confirmed
                && runtime.owned_unconfirmed_generation != Some(persisted.generation)
        });
    if needs_rotation {
        rotate_locked(&state.access, &mut runtime, &path)?;
        return Ok(true);
    }
    Ok(false)
}

pub(crate) fn rotate<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<()> {
    let settings = crate::settings::read(app)?;
    if !super::listener_accepts_non_loopback(&settings)? {
        return Err(
            "SEC_INVALID_INPUT: gateway bearer token is only used by non-loopback listeners"
                .to_string()
                .into(),
        );
    }

    let path = sidecar_path(app)?;
    let state = app.state::<GatewayBearerTokenState>();
    let mut runtime = state.runtime.lock_or_recover();
    initialize_locked(&state.access, &mut runtime, &path, true)?;
    rotate_locked(&state.access, &mut runtime, &path)
}

pub(crate) fn reveal_pending<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<Option<GatewayBearerTokenReveal>> {
    let state = app.state::<GatewayBearerTokenState>();
    let mut runtime = state.runtime.lock_or_recover();
    let Some(pending) = runtime.pending.take() else {
        return Ok(None);
    };
    runtime.revealed_generation = Some(pending.generation);
    Ok(Some(GatewayBearerTokenReveal {
        token: pending.plaintext,
        wsl_sync_error: pending.wsl_sync_error,
    }))
}

pub(crate) fn acknowledge_reveal<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<bool> {
    let path = sidecar_path(app)?;
    let state = app.state::<GatewayBearerTokenState>();
    let mut runtime = state.runtime.lock_or_recover();
    let generation = runtime.revealed_generation.ok_or_else(|| {
        "GATEWAY_BEARER_REVEAL_REQUIRED: no one-time token reveal is awaiting confirmation"
            .to_string()
    })?;
    let mut persisted = runtime.persisted.clone().ok_or_else(|| {
        "GATEWAY_BEARER_STATE_INVALID: private token state is unavailable".to_string()
    })?;
    if persisted.generation != generation {
        return Err(
            "GATEWAY_BEARER_REVEAL_STALE: token generation changed before confirmation"
                .to_string()
                .into(),
        );
    }
    persisted.confirmed = true;
    write_persisted(&path, &persisted)?;
    runtime.persisted = Some(persisted);
    runtime.revealed_generation = None;
    runtime.owned_unconfirmed_generation = None;
    Ok(true)
}

pub(crate) fn pending_plaintext_for_internal_sync<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<String> {
    app.state::<GatewayBearerTokenState>()
        .runtime
        .lock_or_recover()
        .pending
        .as_ref()
        .map(|pending| pending.plaintext.clone())
}

#[cfg(windows)]
pub(crate) fn record_wsl_sync_error<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    error: Option<String>,
) {
    if let Some(pending) = app
        .state::<GatewayBearerTokenState>()
        .runtime
        .lock_or_recover()
        .pending
        .as_mut()
    {
        pending.wsl_sync_error = error;
    }
}

fn initialize_locked(
    access: &GatewayAccessControl,
    runtime: &mut GatewayBearerTokenRuntime,
    path: &Path,
    requires_token: bool,
) -> crate::shared::error::AppResult<()> {
    if runtime.initialized {
        return Ok(());
    }

    match read_persisted(path) {
        Ok(Some(persisted)) => {
            let digest = persisted.decoded_digest();
            match digest {
                Ok(digest) => {
                    access.replace_digest(Some(digest));
                    runtime.persisted = Some(persisted);
                }
                Err(error) if requires_token => {
                    tracing::warn!(error = %error, "invalid gateway bearer token state will be rotated");
                }
                Err(error) => {
                    tracing::warn!(error = %error, "ignoring invalid gateway bearer token state for loopback listener");
                    access.replace_digest(None);
                }
            }
        }
        Ok(None) => access.replace_digest(None),
        Err(error) if requires_token => {
            tracing::warn!(error = %error, "unreadable gateway bearer token state will be rotated");
        }
        Err(error) => {
            tracing::warn!(error = %error, "ignoring unreadable gateway bearer token state for loopback listener");
            access.replace_digest(None);
        }
    }
    runtime.initialized = true;
    Ok(())
}

fn rotate_locked(
    access: &GatewayAccessControl,
    runtime: &mut GatewayBearerTokenRuntime,
    path: &Path,
) -> crate::shared::error::AppResult<()> {
    let plaintext = generate_token();
    let digest = sha256(plaintext.as_bytes());
    let generation = runtime
        .persisted
        .as_ref()
        .map(|persisted| persisted.generation)
        .unwrap_or(0)
        .saturating_add(1)
        .max(1);
    let persisted = PersistedGatewayBearerToken {
        schema_version: SIDECAR_SCHEMA_VERSION,
        token_sha256: URL_SAFE_NO_PAD.encode(digest),
        generation,
        confirmed: false,
    };
    write_persisted(path, &persisted)?;
    access.replace_digest(Some(digest));
    runtime.persisted = Some(persisted);
    runtime.owned_unconfirmed_generation = Some(generation);
    runtime.revealed_generation = None;
    runtime.pending = Some(PendingGatewayBearerToken {
        generation,
        plaintext,
        wsl_sync_error: None,
    });
    Ok(())
}

fn sidecar_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(crate::infra::app_paths::app_data_dir(app)?.join(SIDECAR_FILE_NAME))
}

fn read_persisted(
    path: &Path,
) -> crate::shared::error::AppResult<Option<PersistedGatewayBearerToken>> {
    let Some(bytes) = crate::shared::fs::read_optional_file_with_max_len(path, SIDECAR_MAX_BYTES)?
    else {
        return Ok(None);
    };
    let persisted =
        serde_json::from_slice::<PersistedGatewayBearerToken>(&bytes).map_err(|_| {
            "GATEWAY_BEARER_STATE_INVALID: private token state is malformed".to_string()
        })?;
    persisted.decoded_digest()?;
    Ok(Some(persisted))
}

fn write_persisted(
    path: &Path,
    persisted: &PersistedGatewayBearerToken,
) -> crate::shared::error::AppResult<()> {
    let encoded = serde_json::to_vec(persisted)
        .map_err(|_| "GATEWAY_BEARER_STATE_INVALID: failed to encode private token state")?;
    if encoded.len() > SIDECAR_MAX_BYTES {
        return Err(
            "GATEWAY_BEARER_STATE_INVALID: private token state is too large"
                .to_string()
                .into(),
        );
    }
    crate::shared::fs::write_file_atomic(path, &encoded)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(
            |error| format!("failed to secure gateway bearer token state permissions: {error}"),
        )?;
    }
    Ok(())
}

fn generate_token() -> String {
    let mut bytes = [0_u8; TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn sha256(value: &[u8]) -> [u8; TOKEN_BYTES] {
    Sha256::digest(value).into()
}

pub(crate) fn is_strict_generated_token(token: &str) -> bool {
    token.len() == TOKEN_ENCODED_LEN
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_token_is_high_entropy_shape_and_digest_only_persists() {
        let token_a = generate_token();
        let token_b = generate_token();
        assert_ne!(token_a, token_b);
        assert!(is_strict_generated_token(&token_a));

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(SIDECAR_FILE_NAME);
        let persisted = PersistedGatewayBearerToken {
            schema_version: SIDECAR_SCHEMA_VERSION,
            token_sha256: URL_SAFE_NO_PAD.encode(sha256(token_a.as_bytes())),
            generation: 1,
            confirmed: false,
        };
        write_persisted(&path, &persisted).expect("write sidecar");
        let bytes = std::fs::read(&path).expect("read sidecar");
        assert!(!bytes
            .windows(token_a.len())
            .any(|window| window == token_a.as_bytes()));
        assert_eq!(
            read_persisted(&path)
                .expect("read state")
                .unwrap()
                .generation,
            1
        );
    }

    #[test]
    fn verifier_rejects_malformed_wrong_and_stale_tokens() {
        let first = generate_token();
        let second = generate_token();
        let verifier = GatewayAccessControl::from_token_for_tests(&first);
        assert!(verifier.verify(&first));
        assert!(!verifier.verify(&second));
        assert!(!verifier.verify("short"));

        verifier.replace_digest(Some(sha256(second.as_bytes())));
        assert!(!verifier.verify(&first));
        assert!(verifier.verify(&second));
    }

    #[test]
    fn persisted_state_rejects_plaintext_and_unknown_fields() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(SIDECAR_FILE_NAME);
        std::fs::write(
            &path,
            br#"{"schema_version":1,"token_sha256":"bad","generation":1,"confirmed":false,"token":"secret"}"#,
        )
        .expect("write malformed sidecar");
        assert!(read_persisted(&path).is_err());
    }
}
