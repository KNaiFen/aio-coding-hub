use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub ip: String,
    pub port: u16,
    pub font_size: f64,
    pub background: String,
    pub opacity: f64,
    pub always_on_top: bool,
    pub click_through: bool,
    pub scope: String,
    pub width: f64,
    pub height: f64,
    pub x: Option<f64>,
    pub y: Option<f64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ip: "127.0.0.1".into(),
            port: 13799,
            font_size: 12.0,
            background: "#272B33".into(),
            opacity: 1.0,
            always_on_top: true,
            click_through: false,
            scope: "codex".into(),
            width: 280.0,
            height: 640.0,
            x: None,
            y: None,
        }
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        self.ip
            .parse::<std::net::IpAddr>()
            .map_err(|_| "请输入有效 IP 地址")?;
        if self.port == 0 {
            return Err("端口必须为 1-65535".into());
        }
        if !self.font_size.is_finite() || !(8.0..=32.0).contains(&self.font_size) {
            return Err("字号必须为 8-32".into());
        }
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err("背景不透明度无效".into());
        }
        if self.background.len() != 7
            || !self.background.starts_with('#')
            || !self.background.as_bytes()[1..]
                .iter()
                .all(u8::is_ascii_hexdigit)
        {
            return Err("背景颜色无效".into());
        }
        if aio_observer_protocol::CliScope::parse(&self.scope).is_none() {
            return Err("CLI 范围无效".into());
        }
        if !self.width.is_finite()
            || !self.height.is_finite()
            || !(120.0..=8192.0).contains(&self.width)
            || !(160.0..=8192.0).contains(&self.height)
            || self.x.is_some_and(|v| !v.is_finite())
            || self.y.is_some_and(|v| !v.is_finite())
        {
            return Err("窗口位置或尺寸无效".into());
        }
        Ok(())
    }

    pub fn credential(&self) -> Result<keyring::Entry, String> {
        keyring::Entry::new(
            "io.aio.float.observer",
            &format!("{}:{}", self.ip, self.port),
        )
        .map_err(|_| "无法访问系统凭据存储".into())
    }
}

pub fn load(path: &Path) -> Result<Config, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(_) => return Err("无法读取悬浮窗配置".into()),
    };
    let mut bytes = Vec::new();
    file.take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取悬浮窗配置")?;
    if bytes.len() > 16384 {
        return Err("悬浮窗配置过大".into());
    }
    let config: Config = serde_json::from_slice(&bytes).map_err(|_| "悬浮窗配置无效")?;
    config.validate()?;
    Ok(config)
}

pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    let parent = path.parent().ok_or("配置路径无效")?;
    std::fs::create_dir_all(parent).map_err(|_| "无法创建配置目录")?;
    let temporary: PathBuf = path.with_extension("tmp");
    let result = (|| {
        let bytes = serde_json::to_vec_pretty(config).map_err(|_| "无法编码悬浮窗配置")?;
        let mut file = std::fs::File::create(&temporary).map_err(|_| "无法保存悬浮窗配置")?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| "无法保存悬浮窗配置")?;
        drop(file);
        replace(&temporary, path).map_err(|_| "无法替换悬浮窗配置")
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temporary);
    }
    result.map_err(String::from)
}

#[cfg(not(windows))]
fn replace(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}
#[cfg(windows)]
fn replace(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from: Vec<_> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_valid_and_never_serialize_credentials() {
        let config = Config::default();
        assert!(config.validate().is_ok());
        let value = serde_json::to_value(config).unwrap();
        assert!(value.get("token").is_none());
        assert_eq!(value["port"], 13799);
    }
    #[test]
    fn rejects_invalid_dimensions_colors_and_addresses() {
        for config in [
            Config {
                font_size: f64::NAN,
                ..Config::default()
            },
            Config {
                ip: "http://localhost".into(),
                ..Config::default()
            },
            Config {
                background: "#xxxxxx".into(),
                ..Config::default()
            },
            Config {
                width: 1.0,
                ..Config::default()
            },
        ] {
            assert!(config.validate().is_err());
        }
    }
}
