use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "tui-config-v1.json";
const CONFIG_SCHEMA_VERSION: u64 = 1;
const CONFIG_MAX_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusItem {
    Gateway,
    Scope,
    PreferredProvider,
    LastRequest,
    LastStatus,
    LastProvider,
    LastRoute,
    LastModel,
    LastFolder,
    LastDuration,
    LastTtfb,
    LastCost,
    RecentProvider,
    Concurrency,
    TodayCost,
    TodayTokens,
    AppVersion,
}

impl StatusItem {
    pub const ALL: [Self; 17] = [
        Self::Gateway,
        Self::Scope,
        Self::PreferredProvider,
        Self::LastRequest,
        Self::LastStatus,
        Self::LastProvider,
        Self::LastRoute,
        Self::LastModel,
        Self::LastFolder,
        Self::LastDuration,
        Self::LastTtfb,
        Self::LastCost,
        Self::RecentProvider,
        Self::Concurrency,
        Self::TodayCost,
        Self::TodayTokens,
        Self::AppVersion,
    ];

    pub const DEFAULT: [Self; 6] = [
        Self::PreferredProvider,
        Self::LastRequest,
        Self::RecentProvider,
        Self::Concurrency,
        Self::TodayCost,
        Self::TodayTokens,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Gateway => "gateway",
            Self::Scope => "scope",
            Self::PreferredProvider => "preferred-provider",
            Self::LastRequest => "last-request",
            Self::LastStatus => "last-status",
            Self::LastProvider => "last-provider",
            Self::LastRoute => "last-route",
            Self::LastModel => "last-model",
            Self::LastFolder => "last-folder",
            Self::LastDuration => "last-duration",
            Self::LastTtfb => "last-ttfb",
            Self::LastCost => "last-cost",
            Self::RecentProvider => "recent-provider",
            Self::Concurrency => "concurrency",
            Self::TodayCost => "today-cost",
            Self::TodayTokens => "today-tokens",
            Self::AppVersion => "app-version",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Gateway => "网关状态",
            Self::Scope => "CLI 范围",
            Self::PreferredProvider => "首选供应商",
            Self::LastRequest => "上次请求摘要",
            Self::LastStatus => "上次状态码",
            Self::LastProvider => "上次供应商",
            Self::LastRoute => "上次路由结果",
            Self::LastModel => "上次模型",
            Self::LastFolder => "上次工作区",
            Self::LastDuration => "上次耗时",
            Self::LastTtfb => "上次首字",
            Self::LastCost => "上次费用",
            Self::RecentProvider => "近 10 次主供应商",
            Self::Concurrency => "当前并发",
            Self::TodayCost => "今日费用",
            Self::TodayTokens => "今日 Token",
            Self::AppVersion => "AIO 版本",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|item| item.key() == raw.trim())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiConfig {
    pub status_items: Vec<StatusItem>,
    pub use_colors: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            status_items: StatusItem::DEFAULT.to_vec(),
            use_colors: true,
        }
    }
}

pub fn parse_status_items(raw: &str) -> Result<Vec<StatusItem>, String> {
    let mut items = Vec::new();
    for key in raw.split(',') {
        let key = key.trim();
        if key.is_empty() {
            return Err("--items 不能包含空项目".to_string());
        }
        let item = StatusItem::parse(key).ok_or_else(|| format!("未知状态栏项目：{key}"))?;
        if items.contains(&item) {
            return Err(format!("状态栏项目重复：{key}"));
        }
        items.push(item);
    }
    if items.is_empty() {
        return Err("--items 至少需要一个项目".to_string());
    }
    Ok(items)
}

pub fn load() -> TuiConfig {
    config_path()
        .as_deref()
        .and_then(read_config_at)
        .unwrap_or_default()
}

pub fn save(config: &TuiConfig) -> Result<PathBuf, String> {
    let path = config_path().ok_or_else(|| "无法确定用户目录，未保存状态栏配置".to_string())?;
    write_config_at(&path, config).map_err(|_| "状态栏配置保存失败".to_string())?;
    Ok(path)
}

pub fn colors_enabled(use_colors: bool) -> bool {
    use_colors && std::env::var_os("NO_COLOR").is_none()
}

fn read_config_at(path: &Path) -> Option<TuiConfig> {
    let file = File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take((CONFIG_MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    parse_config_bytes(&bytes)
}

fn parse_config_bytes(bytes: &[u8]) -> Option<TuiConfig> {
    if bytes.is_empty() || bytes.len() > CONFIG_MAX_BYTES {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(bytes).ok()?;
    let object = value.as_object()?;
    if object.get("schemaVersion")?.as_u64()? != CONFIG_SCHEMA_VERSION {
        return None;
    }
    let raw_items = object.get("statusItems")?.as_array()?;
    if raw_items.is_empty() || raw_items.len() > StatusItem::ALL.len() {
        return None;
    }
    let mut status_items = Vec::with_capacity(raw_items.len());
    for raw_item in raw_items {
        let item = StatusItem::parse(raw_item.as_str()?)?;
        if status_items.contains(&item) {
            return None;
        }
        status_items.push(item);
    }
    let use_colors = object.get("useColors")?.as_bool()?;
    Some(TuiConfig {
        status_items,
        use_colors,
    })
}

fn write_config_at(path: &Path, config: &TuiConfig) -> std::io::Result<()> {
    if config.status_items.is_empty()
        || config.status_items.len() > StatusItem::ALL.len()
        || config
            .status_items
            .iter()
            .enumerate()
            .any(|(index, item)| config.status_items[..index].contains(item))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid status-line configuration",
        ));
    }
    let status_items = config
        .status_items
        .iter()
        .map(|item| item.key())
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "schemaVersion": CONFIG_SCHEMA_VERSION,
        "statusItems": status_items,
        "useColors": config.use_colors,
    }))
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    if bytes.len() > CONFIG_MAX_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "status-line configuration is too large",
        ));
    }

    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "configuration path has no parent",
        )
    })?;
    fs::create_dir_all(parent)?;
    let (temporary_path, mut temporary_file) = create_temporary_file(parent)?;
    let result = (|| {
        temporary_file.write_all(&bytes)?;
        temporary_file.sync_all()?;
        drop(temporary_file);
        replace_file(&temporary_path, path)?;
        sync_directory(parent);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn create_temporary_file(parent: &Path) -> std::io::Result<(PathBuf, File)> {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    for attempt in 0..16_u8 {
        let path = parent.join(format!(
            ".{CONFIG_FILE_NAME}.{}.{nonce}.{attempt}.tmp",
            std::process::id()
        ));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "unable to create a unique temporary configuration file",
    ))
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    #[link(name = "kernel32")]
    extern "system" {
        #[link_name = "MoveFileExW"]
        fn move_file_ex_w(existing: *const u16, replacement: *const u16, flags: u32) -> i32;
    }

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: Both path buffers are NUL-terminated and remain alive for the call.
    let result = unsafe {
        move_file_ex_w(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) {
    if let Ok(directory) = File::open(path) {
        let _ = directory.sync_all();
    }
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) {}

fn config_path() -> Option<PathBuf> {
    let home = std::env::var_os("AIO_CODING_HUB_HOME_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(platform_home_dir)?;
    let dotdir = std::env::var("AIO_CODING_HUB_DOTDIR_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| safe_dotdir(value))
        .unwrap_or_else(|| ".aio-coding-hub".to_string());
    Some(home.join(dotdir).join(CONFIG_FILE_NAME))
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
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_test_dir() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("aio-tui-config-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn explicit_items_are_strict_and_ordered() {
        assert_eq!(
            parse_status_items("gateway,last-model,today-cost"),
            Ok(vec![
                StatusItem::Gateway,
                StatusItem::LastModel,
                StatusItem::TodayCost,
            ])
        );
        assert!(parse_status_items("gateway,future-item").is_err());
        assert!(parse_status_items("gateway,gateway").is_err());
        assert!(parse_status_items("").is_err());
        for item in StatusItem::ALL {
            assert_eq!(StatusItem::parse(item.key()), Some(item));
        }
    }

    #[test]
    fn invalid_persisted_values_fail_open() {
        assert!(parse_config_bytes(
            br#"{"schemaVersion":1,"statusItems":["future"],"useColors":true}"#
        )
        .is_none());
        assert!(
            parse_config_bytes(br#"{"schemaVersion":1,"statusItems":[],"useColors":true}"#)
                .is_none()
        );
        assert!(parse_config_bytes(b"not-json").is_none());
        assert!(parse_config_bytes(&[b'x'; CONFIG_MAX_BYTES + 1]).is_none());
    }

    #[test]
    fn config_write_round_trips_and_replaces_existing_file() {
        let directory = unique_test_dir();
        let path = directory.join(CONFIG_FILE_NAME);
        let first = TuiConfig::default();
        write_config_at(&path, &first).expect("write default config");
        assert_eq!(read_config_at(&path), Some(first));

        let second = TuiConfig {
            status_items: vec![StatusItem::Gateway, StatusItem::AppVersion],
            use_colors: false,
        };
        write_config_at(&path, &second).expect("replace config");
        assert_eq!(read_config_at(&path), Some(second));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path)
                .expect("config metadata")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let _ = fs::remove_dir_all(directory);
    }
}
