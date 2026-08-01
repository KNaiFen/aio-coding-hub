use aio_observer_protocol::{
    CliScope, ObserverDescriptorV1, ObserverSnapshotV1, OBSERVER_DESCRIPTOR_FILE_NAME,
    OBSERVER_PROTOCOL_VERSION,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

const DESCRIPTOR_MAX_BYTES: usize = 4 * 1024;
const DESCRIPTOR_TOKEN_MIN_BYTES: usize = 32;
const RESPONSE_MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfflineReason {
    MissingDescriptor,
    InvalidDescriptor,
    Unreachable,
    Unauthorized,
    Busy,
    ProtocolMismatch,
    InvalidResponse,
}

impl OfflineReason {
    pub const fn label(self) -> &'static str {
        match self {
            Self::MissingDescriptor => "AIO 未运行",
            Self::InvalidDescriptor => "连接信息无效",
            Self::Unreachable => "AIO 暂不可达",
            Self::Unauthorized => "本地认证已失效",
            Self::Busy => "观测繁忙",
            Self::ProtocolMismatch => "协议版本不兼容",
            Self::InvalidResponse => "观测数据无效",
        }
    }
}

pub struct ObserverClient {
    http: reqwest::Client,
}

enum SnapshotFetch {
    Ready(ObserverSnapshotV1),
    ProviderQueryUnsupported,
}

impl ObserverClient {
    pub fn new() -> Result<Self, OfflineReason> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_millis(3500))
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|_| OfflineReason::Unreachable)?;
        Ok(Self { http })
    }

    pub async fn snapshot(
        &self,
        scope: CliScope,
        history_limit: u16,
    ) -> Result<ObserverSnapshotV1, OfflineReason> {
        match self.fetch_snapshot(scope, history_limit, false).await? {
            SnapshotFetch::Ready(snapshot) => Ok(snapshot),
            SnapshotFetch::ProviderQueryUnsupported => Err(OfflineReason::InvalidResponse),
        }
    }

    pub async fn snapshot_with_providers(
        &self,
        scope: CliScope,
        history_limit: u16,
    ) -> Result<ObserverSnapshotV1, OfflineReason> {
        match self.fetch_snapshot(scope, history_limit, true).await? {
            SnapshotFetch::Ready(snapshot) => Ok(snapshot),
            SnapshotFetch::ProviderQueryUnsupported => {
                match self.fetch_snapshot(scope, history_limit, false).await? {
                    SnapshotFetch::Ready(snapshot) => Ok(snapshot),
                    SnapshotFetch::ProviderQueryUnsupported => Err(OfflineReason::InvalidResponse),
                }
            }
        }
    }

    async fn fetch_snapshot(
        &self,
        scope: CliScope,
        history_limit: u16,
        include_providers: bool,
    ) -> Result<SnapshotFetch, OfflineReason> {
        let descriptor = read_descriptor()?;
        let mut url = format!(
            "http://127.0.0.1:{}/api/observer/v1/snapshot?cli={}&history_limit={}",
            descriptor.port,
            scope.as_str(),
            history_limit
        );
        if include_providers {
            url.push_str("&include_providers=true");
        }
        let mut response = self
            .http
            .get(url)
            .bearer_auth(&descriptor.token)
            .send()
            .await
            .map_err(|_| OfflineReason::Unreachable)?;
        if include_providers && response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Ok(SnapshotFetch::ProviderQueryUnsupported);
        }
        if let Some(reason) = response_failure_reason(response.status()) {
            return Err(reason);
        }
        if response
            .content_length()
            .is_some_and(|length| length > RESPONSE_MAX_BYTES as u64)
        {
            return Err(OfflineReason::InvalidResponse);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| OfflineReason::InvalidResponse)?
        {
            if bytes.len().saturating_add(chunk.len()) > RESPONSE_MAX_BYTES {
                return Err(OfflineReason::InvalidResponse);
            }
            bytes.extend_from_slice(&chunk);
        }
        let snapshot = serde_json::from_slice::<ObserverSnapshotV1>(&bytes)
            .map_err(|_| OfflineReason::InvalidResponse)?;
        if snapshot.protocol_version != OBSERVER_PROTOCOL_VERSION {
            return Err(OfflineReason::ProtocolMismatch);
        }
        Ok(SnapshotFetch::Ready(snapshot))
    }
}

fn response_failure_reason(status: reqwest::StatusCode) -> Option<OfflineReason> {
    match status {
        reqwest::StatusCode::UNAUTHORIZED => Some(OfflineReason::Unauthorized),
        reqwest::StatusCode::TOO_MANY_REQUESTS => Some(OfflineReason::Busy),
        status if !status.is_success() => Some(OfflineReason::InvalidResponse),
        _ => None,
    }
}

fn read_descriptor() -> Result<ObserverDescriptorV1, OfflineReason> {
    let path = descriptor_path().ok_or(OfflineReason::MissingDescriptor)?;
    read_descriptor_at(&path)
}

fn read_descriptor_at(path: &Path) -> Result<ObserverDescriptorV1, OfflineReason> {
    let bytes = std::fs::read(path).map_err(|_| OfflineReason::MissingDescriptor)?;
    if bytes.is_empty() || bytes.len() > DESCRIPTOR_MAX_BYTES {
        return Err(OfflineReason::InvalidDescriptor);
    }
    let descriptor = serde_json::from_slice::<ObserverDescriptorV1>(&bytes)
        .map_err(|_| OfflineReason::InvalidDescriptor)?;
    if descriptor.schema_version != 1
        || descriptor.protocol_version != OBSERVER_PROTOCOL_VERSION
        || descriptor.port == 0
        || descriptor.token.len() < DESCRIPTOR_TOKEN_MIN_BYTES
        || descriptor.token.len() > 256
    {
        return Err(
            if descriptor.protocol_version != OBSERVER_PROTOCOL_VERSION {
                OfflineReason::ProtocolMismatch
            } else {
                OfflineReason::InvalidDescriptor
            },
        );
    }
    Ok(descriptor)
}

fn descriptor_path() -> Option<PathBuf> {
    let home = std::env::var_os("AIO_CODING_HUB_HOME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(platform_home_dir)?;
    let dotdir = std::env::var("AIO_CODING_HUB_DOTDIR_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| safe_dotdir(value))
        .unwrap_or_else(|| ".aio-coding-hub".to_string());
    Some(home.join(dotdir).join(OBSERVER_DESCRIPTOR_FILE_NAME))
}

fn platform_home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let primary = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let primary = std::env::var_os("HOME");

    #[cfg(windows)]
    let fallback = std::env::var_os("HOME");
    #[cfg(not(windows))]
    let fallback = std::env::var_os("USERPROFILE");

    primary
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            fallback
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

fn safe_dotdir(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.starts_with('.')
        && !value.contains('/')
        && !value.contains('\\')
        && value
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '.' | '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_dotdir_names() {
        assert!(safe_dotdir(".aio-coding-hub"));
        assert!(!safe_dotdir("../aio"));
        assert!(!safe_dotdir("aio"));
        assert!(!safe_dotdir("."));
    }

    #[test]
    fn observer_busy_is_distinct_from_invalid_responses() {
        assert_eq!(
            response_failure_reason(reqwest::StatusCode::TOO_MANY_REQUESTS),
            Some(OfflineReason::Busy)
        );
        assert_eq!(
            response_failure_reason(reqwest::StatusCode::INTERNAL_SERVER_ERROR),
            Some(OfflineReason::InvalidResponse)
        );
        assert_eq!(response_failure_reason(reqwest::StatusCode::OK), None);
    }

    #[test]
    fn descriptor_reader_is_bounded_and_versioned() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("aio-tui-test-{}-{unique}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("descriptor.json");
        let descriptor = ObserverDescriptorV1 {
            schema_version: 1,
            protocol_version: OBSERVER_PROTOCOL_VERSION,
            app_version: "0.60.39".to_string(),
            pid: 1,
            port: 37124,
            started_at_ms: 1,
            token: "0123456789abcdef0123456789abcdef".to_string(),
        };
        std::fs::write(&path, serde_json::to_vec(&descriptor).expect("serialize"))
            .expect("write descriptor");
        assert_eq!(read_descriptor_at(&path).map(|value| value.port), Ok(37124));

        let mut weak = descriptor.clone();
        weak.token = "short".to_string();
        std::fs::write(
            &path,
            serde_json::to_vec(&weak).expect("serialize weak descriptor"),
        )
        .expect("write weak descriptor");
        assert!(matches!(
            read_descriptor_at(&path),
            Err(OfflineReason::InvalidDescriptor)
        ));

        std::fs::write(&path, vec![b'x'; DESCRIPTOR_MAX_BYTES + 1]).expect("write oversized");
        assert!(matches!(
            read_descriptor_at(&path),
            Err(OfflineReason::InvalidDescriptor)
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
