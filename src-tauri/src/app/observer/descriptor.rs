//! Runtime descriptor for the loopback observer service.

use crate::app_paths;
use crate::shared::error::AppResult;
use aio_observer_protocol::{ObserverDescriptorV1, OBSERVER_DESCRIPTOR_FILE_NAME};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const DESCRIPTOR_MAX_BYTES: usize = 4 * 1024;
const TOKEN_BYTES: usize = 32;

pub fn path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> AppResult<PathBuf> {
    Ok(app_paths::app_data_dir(app)?.join(OBSERVER_DESCRIPTOR_FILE_NAME))
}

pub fn new_descriptor(port: u16, app_version: &str, started_at_ms: i64) -> ObserverDescriptorV1 {
    let mut token_bytes = [0_u8; TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut token_bytes);
    ObserverDescriptorV1 {
        schema_version: 1,
        protocol_version: aio_observer_protocol::OBSERVER_PROTOCOL_VERSION,
        app_version: app_version.to_string(),
        pid: std::process::id(),
        port,
        started_at_ms,
        token: URL_SAFE_NO_PAD.encode(token_bytes),
    }
}

pub fn write(path: &Path, descriptor: &ObserverDescriptorV1) -> AppResult<()> {
    let encoded = serde_json::to_vec(descriptor)
        .map_err(|_| "failed to serialize observer descriptor".to_string())?;
    if encoded.len() > DESCRIPTOR_MAX_BYTES {
        return Err("observer descriptor is too large".into());
    }

    let parent = path
        .parent()
        .ok_or_else(|| "observer descriptor has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create observer directory: {err}"))?;

    let temp_path = parent.join(format!(
        ".{}.{}.tmp",
        OBSERVER_DESCRIPTOR_FILE_NAME, descriptor.pid
    ));
    let _ = fs::remove_file(&temp_path);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temp_path)
        .map_err(|err| format!("failed to create observer descriptor temp file: {err}"))?;
    file.write_all(&encoded)
        .map_err(|err| format!("failed to write observer descriptor: {err}"))?;
    file.sync_all()
        .map_err(|err| format!("failed to flush observer descriptor: {err}"))?;
    drop(file);

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)
            .map_err(|err| format!("failed to replace observer descriptor: {err}"))?;
    }
    fs::rename(&temp_path, path)
        .map_err(|err| format!("failed to install observer descriptor: {err}"))?;
    Ok(())
}

pub fn read(path: &Path) -> Option<ObserverDescriptorV1> {
    let bytes = fs::read(path).ok()?;
    if bytes.is_empty() || bytes.len() > DESCRIPTOR_MAX_BYTES {
        return None;
    }
    let descriptor = serde_json::from_slice::<ObserverDescriptorV1>(&bytes).ok()?;
    if descriptor.schema_version != 1
        || descriptor.protocol_version != aio_observer_protocol::OBSERVER_PROTOCOL_VERSION
        || descriptor.port == 0
        || descriptor.token.len() < TOKEN_BYTES
        || descriptor.token.len() > 256
    {
        return None;
    }
    Some(descriptor)
}

pub fn remove_if_owned(path: &Path, pid: u32, token: &str) {
    let Some(current) = read(path) else {
        return;
    };
    if current.pid == pid && current.token == token {
        if let Err(err) = fs::remove_file(path) {
            tracing::debug!(error = %err, "observer descriptor already removed or unavailable");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_round_trips_and_rejects_oversized_content() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(OBSERVER_DESCRIPTOR_FILE_NAME);
        let descriptor = new_descriptor(37124, "0.60.39", 1_700_000_000_000);
        write(&path, &descriptor).expect("write descriptor");
        assert!(read(&path) == Some(descriptor.clone()));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).expect("metadata").permissions().mode() & 0o777,
                0o600
            );
        }

        fs::write(&path, vec![b'x'; DESCRIPTOR_MAX_BYTES + 1]).expect("overwrite descriptor");
        assert!(read(&path).is_none());
    }

    #[test]
    fn remove_only_deletes_matching_owner() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(OBSERVER_DESCRIPTOR_FILE_NAME);
        let descriptor = new_descriptor(37124, "0.60.39", 1_700_000_000_000);
        write(&path, &descriptor).expect("write descriptor");
        remove_if_owned(&path, descriptor.pid.saturating_add(1), &descriptor.token);
        assert!(path.exists());
        remove_if_owned(&path, descriptor.pid, &descriptor.token);
        assert!(!path.exists());
    }
}
