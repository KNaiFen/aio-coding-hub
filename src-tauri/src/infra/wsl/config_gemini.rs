//! WSL Gemini CLI gateway authentication compatibility.

pub(super) fn configure_wsl_gemini(
    _distro: &str,
    _proxy_origin: &str,
    _gateway_bearer_token: &str,
) -> crate::shared::error::AppResult<()> {
    Err(
        "WSL_GATEWAY_TOKEN_UNSUPPORTED: Gemini CLI cannot send the required Gateway Bearer token"
            .to_string()
            .into(),
    )
}
