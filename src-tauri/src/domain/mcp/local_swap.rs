use crate::app_paths;
use crate::mcp_sync;
use crate::shared::error::AppError;
use crate::shared::fs::{read_optional_file_with_max_len, write_file_atomic};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const MCP_LOCAL_STASH_MAX_BYTES: usize = 1024 * 1024;

fn stash_bucket_name(workspace_id: Option<i64>) -> String {
    workspace_id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unassigned".to_string())
}

fn stash_root<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?
        .join("mcp-local")
        .join(cli_key))
}

fn stash_file_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    bucket: &str,
) -> crate::shared::error::AppResult<PathBuf> {
    let dir = stash_root(app, cli_key)?.join(bucket);
    let ext = match cli_key {
        "codex" | "grok" => "toml",
        _ => "json",
    };
    Ok(dir.join(format!("mcpServers.{ext}")))
}

fn ensure_stash_parent(path: &Path) -> crate::shared::error::AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid_stash("MCP local stash path has no parent"))?;
    std::fs::create_dir_all(parent).map_err(|error| -> AppError {
        format!("failed to create {}: {error}", parent.display()).into()
    })
}

fn json_root_from_bytes(bytes: Option<Vec<u8>>) -> Value {
    match bytes {
        Some(b) => serde_json::from_slice::<Value>(&b).unwrap_or_else(|_| serde_json::json!({})),
        None => serde_json::json!({}),
    }
}

fn json_mcp_servers_obj_mut(root: &mut Value) -> &mut serde_json::Map<String, Value> {
    if !root.is_object() {
        *root = serde_json::json!({});
    }
    let root_obj = root.as_object_mut().expect("root must be object");
    let servers_value = root_obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !servers_value.is_object() {
        *servers_value = Value::Object(serde_json::Map::new());
    }
    servers_value
        .as_object_mut()
        .expect("mcpServers must be object")
}

fn json_collect_local_servers(
    servers_obj: &serde_json::Map<String, Value>,
    managed_server_keys: &HashSet<String>,
) -> serde_json::Map<String, Value> {
    let mut out = serde_json::Map::new();
    for (k, v) in servers_obj {
        if !managed_server_keys.contains(k) {
            out.insert(k.to_string(), v.clone());
        }
    }
    out
}

fn json_remove_local_servers(
    servers_obj: &mut serde_json::Map<String, Value>,
    managed_server_keys: &HashSet<String>,
) {
    let keys: Vec<String> = servers_obj
        .keys()
        .filter(|k| !managed_server_keys.contains(*k))
        .cloned()
        .collect();
    for k in keys {
        servers_obj.remove(&k);
    }
}

fn json_write_stash(
    path: &Path,
    map: &serde_json::Map<String, Value>,
) -> crate::shared::error::AppResult<()> {
    let bytes = serde_json::to_vec_pretty(&Value::Object(map.clone()))
        .map_err(|e| format!("failed to serialize stash json: {e}"))?;
    ensure_stash_bytes_len(&bytes)?;
    write_file_atomic(path, &bytes)
}

fn invalid_stash(message: &str) -> AppError {
    AppError::new("RECOVERY_ARTIFACT_INVALID", message)
}

fn read_stash_bytes(path: &Path) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    read_optional_file_with_max_len(path, MCP_LOCAL_STASH_MAX_BYTES)
        .map_err(|_| invalid_stash("MCP local stash is unreadable or exceeds the supported size"))
}

fn json_read_stash(path: &Path) -> crate::shared::error::AppResult<serde_json::Map<String, Value>> {
    let Some(bytes) = read_stash_bytes(path)? else {
        return Ok(serde_json::Map::new());
    };
    let value = serde_json::from_slice::<Value>(&bytes)
        .map_err(|_| invalid_stash("MCP local JSON stash is invalid"))?;
    let servers = value
        .as_object()
        .cloned()
        .ok_or_else(|| invalid_stash("MCP local JSON stash must be an object"))?;
    if servers.values().any(|server| !server.is_object()) {
        return Err(invalid_stash(
            "MCP local JSON stash entries must be objects",
        ));
    }
    Ok(servers)
}

fn parse_codex_mcp_key_from_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("[mcp_servers.") || !trimmed.ends_with(']') {
        return None;
    }
    let inner = trimmed
        .trim_start_matches("[mcp_servers.")
        .trim_end_matches(']');
    let inner = inner.strip_suffix(".env").unwrap_or(inner);
    let unquoted = inner
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(inner);
    Some(unquoted.to_string())
}

fn codex_split_local_blocks(
    input: &str,
    managed_server_keys: &HashSet<String>,
) -> (Vec<String>, Vec<String>) {
    let lines: Vec<String> = if input.is_empty() {
        Vec::new()
    } else {
        input.lines().map(|l| l.to_string()).collect()
    };

    let mut kept: Vec<String> = Vec::with_capacity(lines.len());
    let mut local: Vec<String> = Vec::new();

    let mut idx = 0;
    while idx < lines.len() {
        let line = &lines[idx];
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(key) = parse_codex_mcp_key_from_header(trimmed) {
                let is_local = !managed_server_keys.contains(&key);
                let start = idx;
                idx += 1;
                while idx < lines.len() {
                    let t = lines[idx].trim();
                    if t.starts_with('[') {
                        break;
                    }
                    idx += 1;
                }
                let block = &lines[start..idx];
                if is_local {
                    local.extend(block.iter().cloned());
                    local.push(String::new());
                } else {
                    kept.extend(block.iter().cloned());
                }
                continue;
            }
        }

        kept.push(line.clone());
        idx += 1;
    }

    while local.last().is_some_and(|v| v.trim().is_empty()) {
        local.pop();
    }
    while kept.last().is_some_and(|v| v.trim().is_empty()) {
        kept.pop();
    }
    (kept, local)
}

fn codex_write_stash(path: &Path, lines: &[String]) -> crate::shared::error::AppResult<()> {
    let mut out = lines.join("\n");
    out.push('\n');
    ensure_stash_bytes_len(out.as_bytes())?;
    write_file_atomic(path, out.as_bytes())
}

fn codex_read_stash(path: &Path) -> crate::shared::error::AppResult<Vec<String>> {
    let Some(bytes) = read_stash_bytes(path)? else {
        return Ok(Vec::new());
    };
    let content = String::from_utf8(bytes)
        .map_err(|_| invalid_stash("MCP local Codex stash is not valid UTF-8"))?;
    let document = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| invalid_stash("MCP local Codex stash is invalid TOML"))?;
    if document.iter().any(|(key, item)| {
        key != "mcp_servers"
            || item.as_table_like().is_none()
            || item.as_table_like().is_some_and(|servers| {
                servers
                    .iter()
                    .any(|(_, server)| server.as_table_like().is_none())
            })
    }) {
        return Err(invalid_stash(
            "MCP local Codex stash contains unsupported entries",
        ));
    }
    Ok(content.lines().map(|line| line.to_string()).collect())
}

fn grok_read_stash(path: &Path) -> crate::shared::error::AppResult<Option<Vec<u8>>> {
    let bytes = read_stash_bytes(path)?;
    if let Some(bytes) = bytes.as_deref() {
        mcp_sync::validate_grok_local_stash(bytes)
            .map_err(|_| invalid_stash("MCP local Grok stash is invalid"))?;
    }
    Ok(bytes)
}

fn ensure_stash_bytes_len(bytes: &[u8]) -> crate::shared::error::AppResult<()> {
    if bytes.len() > MCP_LOCAL_STASH_MAX_BYTES {
        return Err(format!(
            "SEC_INVALID_INPUT: MCP local stash too large (max {MCP_LOCAL_STASH_MAX_BYTES} bytes)"
        )
        .into());
    }
    Ok(())
}

pub(crate) fn capture_local_mcp_servers_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    managed_server_keys: &HashSet<String>,
    from_workspace_id: Option<i64>,
) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)?;

    let from_bucket = stash_bucket_name(from_workspace_id);
    let from_path = stash_file_path(app, cli_key, &from_bucket)?;
    ensure_stash_parent(&from_path)?;

    match cli_key {
        "grok" => {
            let stash =
                mcp_sync::capture_grok_local_servers_for_workspace(app, managed_server_keys)?;
            ensure_stash_bytes_len(&stash)?;
            write_file_atomic(&from_path, &stash)?;
        }
        "codex" => {
            let current_bytes = mcp_sync::read_target_bytes(app, cli_key)?;
            let input = current_bytes
                .as_deref()
                .map(|b| String::from_utf8_lossy(b).to_string())
                .unwrap_or_default();
            let (_, local_current) = codex_split_local_blocks(&input, managed_server_keys);
            codex_write_stash(&from_path, &local_current)?;
        }
        _ => {
            let current_bytes = mcp_sync::read_target_bytes(app, cli_key)?;
            let mut root = json_root_from_bytes(current_bytes);
            let local_current = {
                let servers_obj = json_mcp_servers_obj_mut(&mut root);
                json_collect_local_servers(servers_obj, managed_server_keys)
            };
            json_write_stash(&from_path, &local_current)?;
        }
    }

    Ok(())
}

pub(crate) fn validate_local_mcp_stash_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)?;

    let to_path = stash_file_path(app, cli_key, &to_workspace_id.to_string())?;
    match cli_key {
        "grok" => {
            grok_read_stash(&to_path)?;
        }
        "codex" => {
            codex_read_stash(&to_path)?;
        }
        _ => {
            json_read_stash(&to_path)?;
        }
    }
    Ok(())
}

pub(crate) fn restore_local_mcp_servers_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    managed_server_keys: &HashSet<String>,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    crate::shared::cli_key::validate_cli_key(cli_key)?;

    let to_path = stash_file_path(app, cli_key, &to_workspace_id.to_string())?;
    match cli_key {
        "grok" => {
            let target_stash = grok_read_stash(&to_path)?;
            mcp_sync::restore_grok_local_servers_for_workspace(
                app,
                managed_server_keys,
                target_stash,
            )?;
        }
        "codex" => {
            let current_bytes = mcp_sync::read_target_bytes(app, cli_key)?;
            let input = current_bytes
                .as_deref()
                .map(|bytes| String::from_utf8_lossy(bytes).to_string())
                .unwrap_or_default();
            let (mut kept, _) = codex_split_local_blocks(&input, managed_server_keys);
            let local_to = codex_read_stash(&to_path)?;
            if !local_to.is_empty() {
                if !kept.is_empty() {
                    kept.push(String::new());
                }
                kept.extend(local_to);
            }
            let mut out = kept.join("\n");
            out.push('\n');
            mcp_sync::restore_target_bytes(app, cli_key, Some(out.into_bytes()))?;
        }
        _ => {
            let current_bytes = mcp_sync::read_target_bytes(app, cli_key)?;
            let mut root = json_root_from_bytes(current_bytes);
            let local_to = json_read_stash(&to_path)?;
            let servers_obj = json_mcp_servers_obj_mut(&mut root);
            json_remove_local_servers(servers_obj, managed_server_keys);
            for (key, value) in local_to {
                servers_obj.insert(key, value);
            }
            let mut bytes = serde_json::to_vec_pretty(&root)
                .map_err(|error| format!("failed to serialize mcp config json: {error}"))?;
            bytes.push(b'\n');
            mcp_sync::restore_target_bytes(app, cli_key, Some(bytes))?;
        }
    }
    Ok(())
}

pub(crate) fn swap_local_mcp_servers_for_workspace_switch<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    cli_key: &str,
    managed_server_keys: &HashSet<String>,
    from_workspace_id: Option<i64>,
    to_workspace_id: i64,
) -> crate::shared::error::AppResult<()> {
    capture_local_mcp_servers_for_workspace_switch(
        app,
        cli_key,
        managed_server_keys,
        from_workspace_id,
    )?;
    restore_local_mcp_servers_for_workspace_switch(
        app,
        cli_key,
        managed_server_keys,
        to_workspace_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_read_stash_rejects_oversized_or_invalid_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("mcpServers.json");
        std::fs::write(&path, vec![b'x'; MCP_LOCAL_STASH_MAX_BYTES + 1]).expect("write stash");

        let oversized = json_read_stash(&path).unwrap_err().to_string();
        assert!(oversized.contains("RECOVERY_ARTIFACT_INVALID"));

        std::fs::write(&path, b"[]").expect("write invalid stash shape");
        let invalid = json_read_stash(&path).unwrap_err().to_string();
        assert!(invalid.contains("RECOVERY_ARTIFACT_INVALID"));

        std::fs::write(&path, br#"{"invalid":true}"#).expect("write invalid server entry");
        let invalid_entry = json_read_stash(&path).unwrap_err().to_string();
        assert!(invalid_entry.contains("RECOVERY_ARTIFACT_INVALID"));
    }

    #[test]
    fn codex_read_stash_rejects_oversized_or_invalid_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("mcpServers.toml");
        std::fs::write(&path, vec![b'x'; MCP_LOCAL_STASH_MAX_BYTES + 1]).expect("write stash");

        let oversized = codex_read_stash(&path).unwrap_err().to_string();
        assert!(oversized.contains("RECOVERY_ARTIFACT_INVALID"));

        std::fs::write(&path, b"[mcp_servers.invalid").expect("write invalid TOML");
        let invalid = codex_read_stash(&path).unwrap_err().to_string();
        assert!(invalid.contains("RECOVERY_ARTIFACT_INVALID"));

        std::fs::write(&path, b"unrelated = true\n").expect("write unsupported TOML");
        let unsupported = codex_read_stash(&path).unwrap_err().to_string();
        assert!(unsupported.contains("RECOVERY_ARTIFACT_INVALID"));
    }

    #[test]
    fn grok_read_stash_uses_the_same_recovery_artifact_error_contract() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("mcpServers.toml");
        std::fs::write(&path, b"[mcp_servers\ninvalid = true\n").expect("write invalid TOML");

        let invalid = grok_read_stash(&path).unwrap_err().to_string();
        assert!(invalid.contains("RECOVERY_ARTIFACT_INVALID"));

        std::fs::write(&path, vec![b'x'; MCP_LOCAL_STASH_MAX_BYTES + 1])
            .expect("write oversized stash");
        let oversized = grok_read_stash(&path).unwrap_err().to_string();
        assert!(oversized.contains("RECOVERY_ARTIFACT_INVALID"));
    }

    #[test]
    fn missing_stashes_remain_valid_empty_workspace_state() {
        let temp = tempfile::tempdir().expect("tempdir");

        assert!(json_read_stash(&temp.path().join("missing.json"))
            .expect("missing JSON stash")
            .is_empty());
        assert!(codex_read_stash(&temp.path().join("missing.toml"))
            .expect("missing Codex stash")
            .is_empty());
        assert!(grok_read_stash(&temp.path().join("missing-grok.toml"))
            .expect("missing Grok stash")
            .is_none());
    }

    #[test]
    fn non_file_stash_is_a_recovery_artifact_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("mcpServers.json");
        std::fs::create_dir(&path).expect("create invalid stash directory");

        let error = json_read_stash(&path).unwrap_err().to_string();

        assert!(error.contains("RECOVERY_ARTIFACT_INVALID"));
    }

    #[test]
    fn codex_write_stash_rejects_oversized_payload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("mcpServers.toml");
        let lines = vec!["x".repeat(MCP_LOCAL_STASH_MAX_BYTES + 1)];

        let err = codex_write_stash(&path, &lines).unwrap_err().to_string();

        assert!(err.contains("MCP local stash too large"));
        assert!(!path.exists());
    }
}
