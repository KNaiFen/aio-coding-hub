//! LAN lifecycle is serialized by the existing observer runtime lock.
use super::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

const DEFAULT_PORT: u16 = 13799;
const CONFIG_FILE: &str = "observer-lan-v1.json";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanConfig {
    enabled: bool,
    port: u16,
    token: String,
}

impl Default for LanConfig {
    fn default() -> Self {
        Self { enabled: false, port: DEFAULT_PORT, token: descriptor::new_descriptor(DEFAULT_PORT, "", 0).token }
    }
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObserverLanStatus {
    enabled: bool,
    port: u16,
    running: bool,
    addresses: Vec<String>,
    error: Option<String>,
}

pub(super) struct LanRuntime {
    config: LanConfig,
    listener: Option<LanListener>,
    error: Option<String>,
}

struct LanListener {
    token: Arc<RwLock<String>>,
    accepting: Arc<AtomicBool>,
    shutdown: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl LanRuntime {
    pub(super) async fn stop(&mut self) {
        if let Some(listener) = self.listener.take() {
            listener.accepting.store(false, Ordering::Release);
            let _ = listener.shutdown.send(());
            // The listener owns only LAN requests, never the shared observer activity.
            let mut task = listener.task;
            if tokio::time::timeout(STOP_TIMEOUT, &mut task).await.is_err() {
                task.abort();
                let _ = task.await;
            }
        }
    }

    fn status(&self) -> ObserverLanStatus {
        let running = self.listener.as_ref().is_some_and(|listener| !listener.task.is_finished());
        let mut addresses = if_addrs::get_if_addrs().unwrap_or_default().into_iter()
            .filter_map(|interface| match interface.ip() {
                std::net::IpAddr::V4(ip) if !ip.is_loopback() && !ip.is_unspecified() => Some(ip.to_string()),
                _ => None,
            }).collect::<Vec<_>>();
        addresses.sort();
        addresses.dedup();
        ObserverLanStatus {
            enabled: self.config.enabled, port: self.config.port, running, addresses,
            error: self.error.clone().or_else(|| (self.config.enabled && !running).then(|| "局域网观察服务未运行".to_string())),
        }
    }
}

fn read_config(path: &Path) -> Result<LanConfig, String> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(LanConfig::default()),
        Err(_) => return Err("无法读取局域网观察配置".into()),
    };
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes).map_err(|_| "无法读取局域网观察配置")?;
    if bytes.len() > 4096 { return Err("局域网观察配置过大".into()); }
    let config: LanConfig = serde_json::from_slice(&bytes).map_err(|_| "局域网观察配置无效")?;
    if config.port < 1024 || config.token.len() != 43 || !config.token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return Err("局域网观察配置无效".into());
    }
    Ok(config)
}

fn write_config(path: &Path, config: &LanConfig) -> Result<(), String> {
    let parent = path.parent().ok_or("局域网观察配置路径无效")?;
    std::fs::create_dir_all(parent).map_err(|_| "无法创建局域网观察配置目录")?;
    let temp = parent.join(format!(".observer-lan-{}.tmp", rand::random::<u64>()));
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        let mut file = options.open(&temp).map_err(|_| "无法保存局域网观察配置")?;
        let bytes = serde_json::to_vec(config).map_err(|_| "无法编码局域网观察配置")?;
        file.write_all(&bytes).and_then(|()| file.sync_all()).map_err(|_| "无法保存局域网观察配置")?;
        drop(file);
        replace_file(&temp, path).map_err(|_| "无法替换局域网观察配置")
    })();
    if result.is_err() { let _ = std::fs::remove_file(temp); }
    result.map_err(String::from)
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> { std::fs::rename(from, to) }

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let from: Vec<_> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<_> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
        Err(std::io::Error::last_os_error())
    } else { Ok(()) }
}

async fn bind(port: u16) -> Result<tokio::net::TcpListener, String> {
    tokio::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, port)).await
        .map_err(|_| "无法监听局域网观察端口，请检查端口占用和系统权限".to_string())
}

fn serve<R: tauri::Runtime>(shared: &ObserverHttpState<R>, config: &LanConfig, listener: tokio::net::TcpListener) -> LanListener {
    let mut http = shared.clone();
    http.token = Arc::new(RwLock::new(config.token.clone()));
    http.accepting = Arc::new(AtomicBool::new(true));
    http.limiter = Arc::new(Semaphore::new(OBSERVER_MAX_CONCURRENT_REQUESTS));
    http.probe_limiter = Arc::new(Semaphore::new(OBSERVER_MAX_CONCURRENT_PROBES));
    let token = http.token.clone();
    let accepting = http.accepting.clone();
    let (shutdown, rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, observer_router(http)).with_graceful_shutdown(async { let _ = rx.await; }).await;
    });
    LanListener { token, accepting, shutdown, task }
}

pub(super) async fn start<R: tauri::Runtime>(shared: &ObserverHttpState<R>) -> LanRuntime {
    let mut runtime = LanRuntime { config: LanConfig::default(), listener: None, error: None };
    let app = shared.app.clone();
    let loaded = crate::blocking::run("observer_lan_read", move || {
        let path = crate::app_paths::app_data_dir(&app)?.join(CONFIG_FILE);
        read_config(&path).map_err(Into::into)
    }).await;
    match loaded {
        Ok(config) => runtime.config = config,
        Err(_) => { runtime.error = Some("无法读取局域网观察配置".into()); return runtime; }
    }
    if runtime.config.enabled {
        match bind(runtime.config.port).await {
            Ok(listener) => runtime.listener = Some(serve(shared, &runtime.config, listener)),
            Err(error) => runtime.error = Some(error),
        }
    }
    runtime
}

pub(crate) async fn status(app: tauri::AppHandle) -> Result<ObserverLanStatus, String> {
    let state = app.state::<ObserverRuntimeState>();
    let runtime = state.runtime.lock().await;
    Ok(runtime.as_ref().ok_or("观察服务尚未启动")?.lan.status())
}

pub(crate) async fn configure(app: tauri::AppHandle, enabled: bool, port: u16) -> Result<ObserverLanStatus, String> {
    if port < 1024 { return Err("端口号必须为 1024-65535".into()); }
    let state = app.state::<ObserverRuntimeState>();
    let mut guard = state.runtime.lock().await;
    let runtime = guard.as_mut().ok_or("观察服务尚未启动")?;
    let next = LanConfig { enabled, port, token: runtime.lan.config.token.clone() };
    let keep = enabled && runtime.lan.config.port == port && runtime.lan.listener.as_ref().is_some_and(|l| !l.task.is_finished());
    let listener = if enabled && !keep { Some(bind(port).await?) } else { None };
    let path = crate::app_paths::app_data_dir(&app).map_err(|_| "无法确定配置目录")?.join(CONFIG_FILE);
    let persisted = next.clone();
    crate::blocking::run("observer_lan_write", move || write_config(&path, &persisted).map_err(Into::into)).await.map_err(|_| "保存局域网观察配置失败")?;
    if !keep { runtime.lan.stop().await; }
    if let Some(listener) = listener { runtime.lan.listener = Some(serve(&runtime.http_state, &next, listener)); }
    runtime.lan.config = next;
    runtime.lan.error = None;
    Ok(runtime.lan.status())
}

pub(crate) async fn token(app: tauri::AppHandle, rotate: bool) -> Result<String, String> {
    let state = app.state::<ObserverRuntimeState>();
    let mut guard = state.runtime.lock().await;
    let runtime = guard.as_mut().ok_or("观察服务尚未启动")?;
    let mut next = runtime.lan.config.clone();
    if rotate { next.token = LanConfig::default().token; }
    let path = crate::app_paths::app_data_dir(&app).map_err(|_| "无法确定配置目录")?.join(CONFIG_FILE);
    let persisted = next.clone();
    crate::blocking::run("observer_lan_token", move || write_config(&path, &persisted).map_err(Into::into)).await.map_err(|_| "保存观察令牌失败")?;
    if let Some(listener) = &runtime.lan.listener {
        *listener.token.write().unwrap_or_else(|error| error.into_inner()) = next.token.clone();
    }
    runtime.lan.config = next;
    Ok(runtime.lan.config.token.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_is_private_persistent_and_disabled_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONFIG_FILE);
        let config = read_config(&path).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.port, DEFAULT_PORT);
        write_config(&path, &config).unwrap();
        assert_eq!(read_config(&path).unwrap().token, config.token);
        #[cfg(unix)]
        { use std::os::unix::fs::PermissionsExt; assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600); }
        std::fs::write(&path, vec![b'x'; 4097]).unwrap();
        assert!(read_config(&path).is_err());
    }

    #[test]
    fn local_and_lan_tokens_are_not_interchangeable() {
        let local = descriptor::new_descriptor(12345, "test", 0);
        let lan = LanConfig::default();
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, format!("Bearer {}", lan.token).parse().unwrap());
        assert!(authorized(&headers, &lan.token));
        assert!(!authorized(&headers, &local.token));
    }
}
