//! Credential handling and on-disk workspace cache.
//!
//! Workspace metadata (id + name) is cached so you can pick a known workspace
//! instead of retyping its id. By default API keys are entered every startup,
//! but the user may *opt in* to storing a key per workspace. Such keys are
//! written in **plaintext** to the cache file — a security risk the user is
//! warned about before storing. The key prompt can also be pre-filled from
//! `LINKLY_API_KEY` / `LINKLY_WORKSPACE_ID`.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A cached workspace: id, a human-friendly name, and optionally a stored API
/// key (plaintext, opt-in).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedWorkspace {
    pub id: i64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Returns `(api_key, workspace_id)` prefill values read from the environment.
pub fn env_prefill() -> (String, String) {
    (
        std::env::var("LINKLY_API_KEY").unwrap_or_default(),
        std::env::var("LINKLY_WORKSPACE_ID").unwrap_or_default(),
    )
}

/// `~/.config/linkly-tui/workspaces.json` (honouring `XDG_CONFIG_HOME`).
pub fn cache_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("linkly-tui").join("workspaces.json"))
}

/// Load cached workspaces. Any error (missing/corrupt file) yields an empty list.
pub fn load_workspaces() -> Vec<CachedWorkspace> {
    let Some(path) = cache_path() else {
        return Vec::new();
    };
    let Ok(data) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

/// Persist cached workspaces. Errors are ignored — the cache is best-effort.
pub fn save_workspaces(workspaces: &[CachedWorkspace]) {
    let Some(path) = cache_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(data) = serde_json::to_string_pretty(workspaces) {
        let _ = std::fs::write(path, data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspaces_round_trip_through_disk() {
        // Point the cache at a unique temp dir for this test.
        let dir = std::env::temp_dir().join(format!("linkly-tui-test-{}", std::process::id()));
        // SAFETY: single-threaded within this test; no other test reads the env.
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &dir);
        }

        let ws = vec![
            CachedWorkspace {
                id: 42,
                name: "Marketing".to_string(),
                api_key: Some("secret-key".to_string()),
            },
            CachedWorkspace {
                id: 7,
                name: "Personal".to_string(),
                api_key: None,
            },
        ];
        save_workspaces(&ws);
        let loaded = load_workspaces();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, 42);
        assert_eq!(loaded[0].name, "Marketing");
        assert_eq!(loaded[0].api_key.as_deref(), Some("secret-key"));
        assert_eq!(loaded[1].id, 7);
        assert_eq!(loaded[1].api_key, None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
