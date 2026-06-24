//! Credential handling.
//!
//! Per design, credentials are never persisted to disk — they are entered on
//! every startup. As a convenience the masked prompt can be pre-filled from the
//! `LINKLY_API_KEY` / `LINKLY_WORKSPACE_ID` environment variables, but the user
//! still has to confirm with Enter.

/// Returns `(api_key, workspace_id)` prefill values read from the environment.
/// Missing variables yield empty strings.
pub fn env_prefill() -> (String, String) {
    (
        std::env::var("LINKLY_API_KEY").unwrap_or_default(),
        std::env::var("LINKLY_WORKSPACE_ID").unwrap_or_default(),
    )
}
