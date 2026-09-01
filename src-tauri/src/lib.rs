use chrono::{FixedOffset, Utc};
use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, Shutdown, TcpListener, TcpStream, UdpSocket},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};
#[cfg(target_os = "macos")]
use tauri::menu::{PredefinedMenuItem, Submenu};
#[cfg(target_os = "macos")]
use tauri::TitleBarStyle;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, State, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};
use tiny_http::{Header, Method, Response, Server, StatusCode};
use uuid::Uuid;
#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY,
};

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> std::ffi::c_uchar;
}

const DEFAULT_SERVER_PORT: u16 = 17866;
const DEFAULT_DEVICE_ID: &str = "default-iphone";
const DEFAULT_DEVICE_NAME: &str = "iPhone";
const MIN_SERVER_PORT: u16 = 1024;
const MAX_SENDER_DEVICES: usize = 5;
const MAX_MESSAGES: usize = 100;
const MAX_LOG_FILE_BYTES: u64 = 5 * 1024 * 1024;
const TRAY_EXIT_ID: &str = "quit";
const MENU_SETTINGS_ID: &str = "open-settings";
const NOTIFICATION_LABEL: &str = "message-toast";
const NOTIFICATION_WIDTH: f64 = 380.0;
const NOTIFICATION_HEIGHT: f64 = 116.0;
const NOTIFICATION_MARGIN: i32 = 18;
const DEFAULT_RELAY_POLL_INTERVAL: Duration = Duration::from_secs(2);
const RELAY_ERROR_REPORT_THRESHOLD: u32 = 3;
const RECEIVER_START_RETRY_ATTEMPTS: u8 = 6;
const RECEIVER_START_RETRY_DELAY: Duration = Duration::from_millis(150);
const RECEIVER_STOP_RELEASE_ATTEMPTS: u8 = 20;
const RECEIVER_STOP_RELEASE_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum NotificationPosition {
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
    TopCenter,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum NotificationMode {
    All,
    Verification,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IncomingMessage {
    id: String,
    sender: String,
    text: String,
    code: Option<String>,
    copied_text: String,
    received_at: String,
    remote_addr: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IncomingPayload {
    sender: Option<String>,
    text: Option<String>,
    message: Option<String>,
    code: Option<String>,
    #[serde(rename = "id")]
    device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelaySettings {
    enabled: bool,
    base_url: String,
    secret: String,
}

impl Default for RelaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            base_url: String::new(),
            secret: String::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelayMessage {
    #[serde(default)]
    relay_id: String,
    #[serde(default)]
    sender: String,
    text: String,
    #[serde(default)]
    code: String,
    #[serde(rename = "id")]
    device_id: String,
    received_at: String,
    remote_addr: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelayPollResponse {
    #[serde(default)]
    messages: Vec<RelayMessage>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfo {
    os: &'static str,
    is_macos: bool,
    is_windows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SenderDevice {
    id: String,
    name: String,
    #[serde(default, rename = "deviceId")]
    device_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiverStatus {
    port: u16,
    local_ip: Option<String>,
    endpoint: String,
    message_count: usize,
    receiver_running: bool,
    notification_mode: NotificationMode,
    notification_position: NotificationPosition,
    direct_paste_enabled: bool,
    relay_enabled: bool,
    relay_running: bool,
    relay_base_url: String,
    relay_secret: String,
    sender_devices: Vec<SenderDevice>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    port: u16,
    #[serde(default = "default_receiver_enabled")]
    receiver_enabled: bool,
    #[serde(default = "default_notification_mode")]
    notification_mode: NotificationMode,
    #[serde(default)]
    notification_enabled: Option<bool>,
    #[serde(default = "default_notification_position")]
    notification_position: NotificationPosition,
    #[serde(default)]
    direct_paste_enabled: bool,
    #[serde(default)]
    relay: RelaySettings,
    #[serde(default)]
    sender_devices: Vec<SenderDevice>,
}

struct AppState {
    messages: Mutex<VecDeque<IncomingMessage>>,
    storage_path: PathBuf,
    settings_path: PathBuf,
    port: Mutex<u16>,
    receiver_enabled: Mutex<bool>,
    notification_mode: Mutex<NotificationMode>,
    notification_position: Mutex<NotificationPosition>,
    direct_paste_enabled: Mutex<bool>,
    relay: Mutex<RelaySettings>,
    relay_running: Mutex<bool>,
    sender_devices: Mutex<Vec<SenderDevice>>,
    receiver: Mutex<Option<Arc<Server>>>,
    log_path: PathBuf,
    log_lock: Mutex<()>,
}

fn write_log(state: &Arc<AppState>, tag: &str, msg: &str) {
    let _guard = state.log_lock.lock().ok();
    let timestamp = beijing_now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let line = format!("[{}] [{}] {}\n", timestamp, tag, msg);
    rotate_log_if_needed(&state.log_path, line.len() as u64);
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&state.log_path)
    {
        let _ = file.write_all(line.as_bytes());
    }
}

fn beijing_now() -> chrono::DateTime<FixedOffset> {
    Utc::now().with_timezone(&FixedOffset::east_opt(8 * 3600).expect("valid Beijing offset"))
}

fn rotate_log_if_needed(log_path: &PathBuf, next_line_bytes: u64) {
    let Ok(metadata) = fs::metadata(log_path) else {
        return;
    };

    if metadata.len().saturating_add(next_line_bytes) < MAX_LOG_FILE_BYTES {
        return;
    }

    let archived_path = log_path.with_extension("log.old");
    let _ = fs::remove_file(&archived_path);
    let _ = fs::rename(log_path, archived_path);
}

#[tauri::command]
fn log_message(state: State<'_, Arc<AppState>>, tag: String, msg: String) {
    write_log(&state, &tag, &msg);
}

#[tauri::command]
fn get_messages(state: State<'_, Arc<AppState>>) -> Vec<IncomingMessage> {
    state
        .messages
        .lock()
        .expect("message state poisoned")
        .iter()
        .cloned()
        .collect()
}

#[tauri::command]
fn clear_messages(state: State<'_, Arc<AppState>>, app: AppHandle) {
    state
        .messages
        .lock()
        .expect("message state poisoned")
        .clear();
    if let Err(error) = persist_messages(&state) {
        let _ = app.emit("receiver-error", format!("清空本地消息失败: {}", error));
    }
    let _ = app.emit("messages-cleared", ());
}

#[tauri::command]
fn receiver_status(state: State<'_, Arc<AppState>>) -> ReceiverStatus {
    let local_ip = local_ipv4();
    let port = current_port(&state);
    let endpoint = match &local_ip {
        Some(ip) => format!("http://{}:{}/otp", ip, port),
        None => format!("http://<Windows局域网IP>:{}/otp", port),
    };

    ReceiverStatus {
        port,
        local_ip,
        endpoint,
        message_count: state.messages.lock().expect("message state poisoned").len(),
        receiver_running: receiver_is_running(&state),
        notification_mode: current_notification_mode(&state),
        notification_position: current_notification_position(&state),
        direct_paste_enabled: direct_paste_is_enabled(&state),
        relay_enabled: current_relay_settings(&state).enabled,
        relay_running: relay_is_running(&state),
        relay_base_url: current_relay_settings(&state).base_url,
        relay_secret: current_relay_settings(&state).secret,
        sender_devices: current_sender_devices(&state),
    }
}

#[tauri::command]
fn platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS,
        is_macos: cfg!(target_os = "macos"),
        is_windows: cfg!(target_os = "windows"),
    }
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    show_main_window(&app);
    let _ = app.emit("open-settings", ());
}

#[tauri::command]
fn hide_main_window_command(app: AppHandle) {
    hide_main_window(&app);
}

#[tauri::command]
fn set_notification_mode(
    mode: NotificationMode,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state = state.inner().clone();
    *state
        .notification_mode
        .lock()
        .map_err(|_| "通知设置不可用".to_string())? = mode;
    persist_settings(&state)
}

#[tauri::command]
fn set_notification_position(
    position: NotificationPosition,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state = state.inner().clone();
    *state
        .notification_position
        .lock()
        .map_err(|_| "通知位置设置不可用".to_string())? = position;
    persist_settings(&state)
}

#[tauri::command]
fn set_direct_paste_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    if enabled {
        ensure_direct_paste_permission()?;
    }

    let state = state.inner().clone();
    *state
        .direct_paste_enabled
        .lock()
        .map_err(|_| "直接输入设置不可用".to_string())? = enabled;
    persist_settings(&state)
}

#[tauri::command]
fn set_relay_settings(
    relay: RelaySettings,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let state = state.inner().clone();
    let relay = normalize_relay_settings(relay)?;
    let enabled = relay.enabled;
    *state
        .relay
        .lock()
        .map_err(|_| "云端接入设置不可用".to_string())? = relay;
    persist_settings(&state)?;

    if enabled {
        start_relay_client(app, state)?;
    } else {
        stop_relay_client(&state)?;
    }

    Ok(())
}

#[tauri::command]
fn test_relay_connection(relay: RelaySettings) -> Result<(), String> {
    let relay = normalize_relay_settings(RelaySettings {
        enabled: true,
        base_url: relay.base_url,
        secret: relay.secret,
    })?;
    let client = Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|error| format!("云端连接初始化失败: {}", error))?;
    let url = format!("{}/api/verify", relay.base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .bearer_auth(relay.secret)
        .send()
        .map_err(|error| format!("云端服务不可用: {}", error))?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("云端服务返回 HTTP {}", response.status()))
    }
}

#[tauri::command]
fn set_sender_devices(
    devices: Vec<SenderDevice>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if devices.len() > MAX_SENDER_DEVICES {
        return Err(format!("最多只能添加 {} 个设备", MAX_SENDER_DEVICES));
    }

    validate_sender_devices(&devices)?;
    let state = state.inner().clone();
    *state
        .sender_devices
        .lock()
        .map_err(|_| "设备设置不可用".to_string())? = normalize_sender_devices(devices);
    persist_settings(&state)
}

#[tauri::command]
fn start_receiver_command(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = state.inner().clone();
    write_log(
        &state,
        "BACKEND",
        &format!("收到启动监听请求: port={}", current_port(&state)),
    );
    start_receiver(app, state.clone())?;
    set_receiver_enabled(&state, true)?;
    persist_settings(&state).map_err(|error| {
        write_log(&state, "BACKEND", &format!("启动监听状态保存失败: {}", error));
        error
    })?;
    write_log(&state, "BACKEND", "启动监听状态已保存");
    Ok(())
}

#[tauri::command]
fn stop_receiver(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = state.inner().clone();
    write_log(
        &state,
        "BACKEND",
        &format!("收到停止监听请求: port={}", current_port(&state)),
    );
    stop_receiver_inner(&state)?;
    set_receiver_enabled(&state, false)?;
    persist_settings(&state).map_err(|error| {
        write_log(&state, "BACKEND", &format!("停止监听状态保存失败: {}", error));
        error
    })?;
    write_log(&state, "BACKEND", "停止监听状态已保存");
    Ok(())
}

#[tauri::command]
fn type_verification_code(code: String) -> Result<(), String> {
    type_text(&code)
}

#[tauri::command]
fn set_receiver_port(
    port: u16,
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    validate_port(port)?;

    let state = state.inner().clone();
    if current_port(&state) == port {
        return Ok(());
    }

    let was_running = receiver_is_running(&state);
    if was_running {
        write_log(
            &state,
            "BACKEND",
            &format!("端口更新前停止监听: old_port={}, new_port={}", current_port(&state), port),
        );
        stop_receiver_inner(&state)?;
        thread::sleep(Duration::from_millis(350));
    }

    *state
        .port
        .lock()
        .map_err(|_| "端口设置不可用".to_string())? = port;
    persist_settings(&state)?;

    if was_running {
        write_log(
            &state,
            "BACKEND",
            &format!("端口更新后重新启动监听: port={}", port),
        );
        start_receiver(app, state)?;
    }

    Ok(())
}

fn stop_receiver_inner(state: &Arc<AppState>) -> Result<(), String> {
    let port = current_port(state);
    let server = state
        .receiver
        .lock()
        .map_err(|_| "接收服务状态不可用".to_string())?
        .take();

    if let Some(server) = server {
        write_log(state, "BACKEND", &format!("正在停止监听服务: port={}", port));
        server.unblock();
        drop(server);

        if wait_for_receiver_port_release(state, port) {
            write_log(state, "BACKEND", &format!("监听服务已停止: port={}", port));
        } else {
            write_log(
                state,
                "BACKEND",
                &format!("监听服务停止后端口仍被占用: port={}", port),
            );
        }
    } else {
        write_log(state, "BACKEND", &format!("监听服务已是停止状态: port={}", port));
    }

    Ok(())
}

fn start_receiver(app: AppHandle, state: Arc<AppState>) -> Result<(), String> {
    if receiver_is_running(&state) {
        write_log(
            &state,
            "BACKEND",
            &format!("监听服务已在运行，跳过启动: port={}", current_port(&state)),
        );
        return Ok(());
    }

    let port = current_port(&state);
    write_log(&state, "BACKEND", &format!("正在启动监听服务: port={}", port));
    let server = bind_receiver_server(&state, port)?;
    write_log(&state, "BACKEND", &format!("监听端口绑定成功: port={}", port));

    *state
        .receiver
        .lock()
        .map_err(|_| "接收服务状态不可用".to_string())? = Some(server.clone());

    let thread_state = state.clone();
    thread::spawn(move || loop {
        if !receiver_is_running(&thread_state) {
            write_log(&thread_state, "BACKEND", "监听线程退出: 状态已停止");
            break;
        }

        match server.recv_timeout(Duration::from_millis(250)) {
            Ok(Some(request)) => handle_request(request, &app, &thread_state),
            Ok(None) => {}
            Err(error) => {
                write_log(&thread_state, "BACKEND", &format!("监听线程退出: {}", error));
                break;
            }
        }
    });

    write_log(&state, "BACKEND", &format!("监听服务已启动: port={}", port));
    Ok(())
}

fn bind_receiver_server(state: &Arc<AppState>, port: u16) -> Result<Arc<Server>, String> {
    let mut last_error = String::new();

    for attempt in 0..RECEIVER_START_RETRY_ATTEMPTS {
        match Server::http(("0.0.0.0", port)) {
            Ok(server) => return Ok(Arc::new(server)),
            Err(error) => {
                last_error = error.to_string();
                write_log(
                    state,
                    "BACKEND",
                    &format!(
                        "监听端口绑定失败: port={}, attempt={}/{}, error={}",
                        port,
                        attempt + 1,
                        RECEIVER_START_RETRY_ATTEMPTS,
                        last_error
                    ),
                );
                if attempt + 1 < RECEIVER_START_RETRY_ATTEMPTS {
                    thread::sleep(RECEIVER_START_RETRY_DELAY);
                }
            }
        }
    }

    Err(format!("接收服务启动失败: {}", last_error))
}

fn wait_for_receiver_port_release(state: &Arc<AppState>, port: u16) -> bool {
    for attempt in 0..RECEIVER_STOP_RELEASE_ATTEMPTS {
        wake_receiver_listener(port);

        if receiver_port_is_available(port) {
            write_log(
                state,
                "BACKEND",
                &format!(
                    "监听端口已释放: port={}, attempt={}/{}",
                    port,
                    attempt + 1,
                    RECEIVER_STOP_RELEASE_ATTEMPTS
                ),
            );
            return true;
        }

        thread::sleep(RECEIVER_STOP_RELEASE_DELAY);
    }

    false
}

fn wake_receiver_listener(port: u16) {
    if let Ok(stream) = TcpStream::connect((Ipv4Addr::LOCALHOST, port)) {
        let _ = stream.shutdown(Shutdown::Both);
    }
}

fn receiver_port_is_available(port: u16) -> bool {
    TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).is_ok()
}

fn current_port(state: &Arc<AppState>) -> u16 {
    state
        .port
        .lock()
        .map(|port| *port)
        .unwrap_or(DEFAULT_SERVER_PORT)
}

fn current_receiver_enabled(state: &Arc<AppState>) -> bool {
    state
        .receiver_enabled
        .lock()
        .map(|enabled| *enabled)
        .unwrap_or_else(|_| default_receiver_enabled())
}

fn set_receiver_enabled(state: &Arc<AppState>, enabled: bool) -> Result<(), String> {
    *state
        .receiver_enabled
        .lock()
        .map_err(|_| "接收服务设置不可用".to_string())? = enabled;
    Ok(())
}

fn validate_port(port: u16) -> Result<(), String> {
    if port < MIN_SERVER_PORT {
        return Err(format!("端口号需在 {}-65535 之间", MIN_SERVER_PORT));
    }

    Ok(())
}

fn current_notification_mode(state: &Arc<AppState>) -> NotificationMode {
    state
        .notification_mode
        .lock()
        .map(|mode| *mode)
        .unwrap_or_else(|_| default_notification_mode())
}

fn should_show_notification(state: &Arc<AppState>, message: &IncomingMessage) -> bool {
    match current_notification_mode(state) {
        NotificationMode::All => true,
        NotificationMode::Verification => message_is_verification_notice(message),
        NotificationMode::Off => false,
    }
}

fn current_notification_position(state: &Arc<AppState>) -> NotificationPosition {
    state
        .notification_position
        .lock()
        .map(|position| *position)
        .unwrap_or_else(|_| default_notification_position())
}

fn direct_paste_is_enabled(state: &Arc<AppState>) -> bool {
    state
        .direct_paste_enabled
        .lock()
        .map(|enabled| *enabled)
        .unwrap_or(false)
}

fn current_relay_settings(state: &Arc<AppState>) -> RelaySettings {
    state
        .relay
        .lock()
        .map(|relay| relay.clone())
        .unwrap_or_default()
}

fn normalize_relay_settings(relay: RelaySettings) -> Result<RelaySettings, String> {
    let base_url = relay.base_url.trim().trim_end_matches('/').to_string();
    let secret = relay.secret.trim().to_string();

    if relay.enabled {
        if base_url.is_empty() {
            return Err("请填写云端服务地址".to_string());
        }
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err("云端服务地址需以 http:// 或 https:// 开头".to_string());
        }
        if secret.len() < 12 {
            return Err("云端密钥至少需要 12 个字符".to_string());
        }
    }

    Ok(RelaySettings {
        enabled: relay.enabled,
        base_url,
        secret,
    })
}

fn relay_is_running(state: &Arc<AppState>) -> bool {
    state
        .relay_running
        .lock()
        .map(|running| *running)
        .unwrap_or(false)
}

fn set_relay_running(state: &Arc<AppState>, running: bool) -> Result<(), String> {
    *state
        .relay_running
        .lock()
        .map_err(|_| "云端接入状态不可用".to_string())? = running;
    Ok(())
}

fn current_sender_devices(state: &Arc<AppState>) -> Vec<SenderDevice> {
    state
        .sender_devices
        .lock()
        .map(|devices| devices.clone())
        .unwrap_or_default()
}

fn start_relay_client(app: AppHandle, state: Arc<AppState>) -> Result<(), String> {
    let relay = current_relay_settings(&state);
    if !relay.enabled {
        return Ok(());
    }
    normalize_relay_settings(relay.clone())?;
    if relay_is_running(&state) {
        return Ok(());
    }

    set_relay_running(&state, true)?;
    thread::spawn(move || relay_poll_loop(app, state));
    Ok(())
}

fn stop_relay_client(state: &Arc<AppState>) -> Result<(), String> {
    set_relay_running(state, false)
}

fn relay_poll_loop(app: AppHandle, state: Arc<AppState>) {
    let relay = current_relay_settings(&state);
    write_log(
        &state,
        "BACKEND",
        &format!(
            "云端轮询启动: base_url={}, secret_len={}",
            relay.base_url,
            relay.secret.len()
        ),
    );

    let client = match Client::builder().timeout(Duration::from_secs(35)).build() {
        Ok(client) => client,
        Err(error) => {
            write_log(&state, "BACKEND", &format!("云端客户端创建失败: {}", error));
            let _ = set_relay_running(&state, false);
            let _ = app.emit("receiver-error", format!("云端接入初始化失败: {}", error));
            return;
        }
    };
    let mut after = String::new();
    let mut consecutive_errors = 0_u32;

    while relay_is_running(&state) {
        let relay = current_relay_settings(&state);
        if !relay.enabled {
            write_log(&state, "BACKEND", "云端已手动关闭");
            let _ = set_relay_running(&state, false);
            break;
        }

        let mut poll_url = format!("{}/api/poll", relay.base_url.trim_end_matches('/'));
        if !after.is_empty() {
            poll_url.push_str("?after=");
            poll_url.push_str(&url_encode_query_value(&after));
        }
        write_log(&state, "BACKEND", &format!("发送轮询请求: {}", poll_url));
        let request = client.get(&poll_url).bearer_auth(relay.secret);

        match request.send() {
            Ok(response) => {
                let status = response.status();
                write_log(
                    &state,
                    "BACKEND",
                    &format!("收到响应: HTTP {}", status.as_u16()),
                );

                if status.is_success() {
                    match response.text() {
                        Ok(body) => {
                            write_log(
                                &state,
                                "BACKEND",
                                &format!("响应体 (前500字符): {}", &body[..body.len().min(500)]),
                            );
                            match serde_json::from_str::<RelayPollResponse>(&body) {
                                Ok(payload) => {
                                    consecutive_errors = 0;
                                    write_log(
                                        &state,
                                        "BACKEND",
                                        &format!("解析成功, 消息数: {}", payload.messages.len()),
                                    );
                                    for relay_message in payload.messages {
                                        if !relay_is_running(&state) {
                                            break;
                                        }
                                        write_log(&state, "BACKEND", &format!(
                                            "处理消息: relay_id={}, sender={}, text={}, id={}, received_at={}, remote_addr={}",
                                            relay_message.relay_id,
                                            relay_message.sender,
                                            relay_message.text,
                                            relay_message.device_id,
                                            relay_message.received_at,
                                            relay_message.remote_addr,
                                        ));
                                        after = relay_message.received_at.clone();
                                        if let Some(message) =
                                            message_from_relay(relay_message, &state)
                                        {
                                            write_log(&state, "BACKEND", "消息已存储并发送到前端");
                                            store_message(&app, &state, message);
                                        } else {
                                            write_log(
                                                &state,
                                                "BACKEND",
                                                "消息被过滤: 设备ID未匹配",
                                            );
                                        }
                                    }
                                }
                                Err(error) => {
                                    consecutive_errors += 1;
                                    write_log(
                                        &state,
                                        "BACKEND",
                                        &format!(
                                            "JSON解析失败(连续{}次): {} | 原始响应体: {}",
                                            consecutive_errors, error, body
                                        ),
                                    );
                                    emit_relay_error_if_needed(
                                        &app,
                                        consecutive_errors,
                                        format!("云端消息解析失败: {}", error),
                                    );
                                    thread::sleep(DEFAULT_RELAY_POLL_INTERVAL);
                                }
                            }
                        }
                        Err(error) => {
                            consecutive_errors += 1;
                            write_log(
                                &state,
                                "BACKEND",
                                &format!("读取响应体失败(连续{}次): {}", consecutive_errors, error),
                            );
                            emit_relay_error_if_needed(
                                &app,
                                consecutive_errors,
                                format!("云端消息解析失败: {}", error),
                            );
                            thread::sleep(DEFAULT_RELAY_POLL_INTERVAL);
                        }
                    }
                } else {
                    consecutive_errors += 1;
                    write_log(
                        &state,
                        "BACKEND",
                        &format!(
                            "HTTP错误(连续{}次): {}",
                            consecutive_errors,
                            status.as_u16()
                        ),
                    );
                    emit_relay_error_if_needed(
                        &app,
                        consecutive_errors,
                        format!("云端接入失败: HTTP {}", status.as_u16()),
                    );
                    thread::sleep(DEFAULT_RELAY_POLL_INTERVAL);
                }
            }
            Err(error) => {
                consecutive_errors += 1;
                write_log(
                    &state,
                    "BACKEND",
                    &format!("网络请求失败(连续{}次): {}", consecutive_errors, error),
                );
                if relay_is_running(&state) {
                    emit_relay_error_if_needed(
                        &app,
                        consecutive_errors,
                        format!("云端接入连接失败: {}", error),
                    );
                    thread::sleep(DEFAULT_RELAY_POLL_INTERVAL);
                }
            }
        }
    }
    write_log(&state, "BACKEND", "云端轮询线程退出");
}

fn emit_relay_error_if_needed(app: &AppHandle, consecutive_errors: u32, message: String) {
    if consecutive_errors >= RELAY_ERROR_REPORT_THRESHOLD {
        let _ = app.emit("receiver-error", message);
    }
}

fn url_encode_query_value(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{:02X}", byte).chars().collect(),
        })
        .collect()
}

fn normalize_sender_devices(devices: Vec<SenderDevice>) -> Vec<SenderDevice> {
    let mut seen_device_ids = HashSet::new();
    let normalized: Vec<SenderDevice> = devices
        .into_iter()
        .filter_map(|device| {
            let device_id = unique_device_id(&device.device_id, &mut seen_device_ids);

            let id = if device.id.trim().is_empty() {
                Uuid::new_v4().to_string()
            } else {
                device.id.trim().to_string()
            };
            let name = normalize_device_name(&device.name);

            Some(SenderDevice {
                id,
                name,
                device_id,
            })
        })
        .collect();

    if normalized.is_empty() {
        vec![default_sender_device()]
    } else {
        normalized
    }
}

fn validate_sender_devices(devices: &[SenderDevice]) -> Result<(), String> {
    let mut seen_device_ids = HashSet::new();
    for device in devices {
        let device_id = device.device_id.trim();
        if !is_valid_device_id(device_id) {
            return Err("设备 ID 必须为 7 位数字".to_string());
        }
        if !seen_device_ids.insert(device_id.to_string()) {
            return Err("设备 ID 不能重复".to_string());
        }
    }

    Ok(())
}

fn unique_device_id(preferred: &str, seen_device_ids: &mut HashSet<String>) -> String {
    let preferred = preferred.trim();
    if is_valid_device_id(preferred) && seen_device_ids.insert(preferred.to_string()) {
        return preferred.to_string();
    }

    loop {
        let device_id = generate_device_id();
        if seen_device_ids.insert(device_id.clone()) {
            return device_id;
        }
    }
}

fn is_valid_device_id(device_id: &str) -> bool {
    device_id.len() == 7
        && device_id
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn generate_device_id() -> String {
    (1_000_000 + (Uuid::new_v4().as_u128() % 9_000_000)).to_string()
}

fn receiver_is_running(state: &Arc<AppState>) -> bool {
    state
        .receiver
        .lock()
        .map(|receiver| receiver.is_some())
        .unwrap_or(false)
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let tray_settings = MenuItem::with_id(app, MENU_SETTINGS_ID, "设置...", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, TRAY_EXIT_ID, "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&tray_settings, &quit])?;
    #[cfg(target_os = "macos")]
    let icon = Image::from_bytes(include_bytes!("../icons/menu.png"))?;
    #[cfg(not(target_os = "macos"))]
    let icon = Image::from_bytes(include_bytes!("../icons/icon.ico"))?;

    #[cfg(target_os = "macos")]
    {
        let menu_settings =
            MenuItem::with_id(app, MENU_SETTINGS_ID, "设置...", true, Some("CmdOrCtrl+,"))?;
        let menu_quit =
            MenuItem::with_id(app, TRAY_EXIT_ID, "退出程序", true, Some("CmdOrCtrl+Q"))?;
        let app_menu =
            Submenu::with_items(app, "验证码接收器", true, &[&menu_settings, &menu_quit])?;
        let edit_menu = Submenu::with_items(
            app,
            "Edit",
            true,
            &[
                &PredefinedMenuItem::undo(app, None)?,
                &PredefinedMenuItem::redo(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::cut(app, None)?,
                &PredefinedMenuItem::copy(app, None)?,
                &PredefinedMenuItem::paste(app, None)?,
                &PredefinedMenuItem::select_all(app, None)?,
            ],
        )?;
        let main_menu = Menu::with_items(app, &[&app_menu, &edit_menu])?;
        app.set_menu(main_menu)?;
    }

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("验证码接收器")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SETTINGS_ID => {
                let _ = app.emit("open-settings", ());
                show_main_window(app);
            }
            TRAY_EXIT_ID => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                show_main_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

fn handle_request(mut request: tiny_http::Request, app: &AppHandle, state: &Arc<AppState>) {
    let origin = request
        .remote_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    if request.method() == &Method::Options {
        respond(request, StatusCode(204), "");
        return;
    }

    if request.method() != &Method::Post || request.url() != "/otp" {
        respond(
            request,
            StatusCode(404),
            r#"{"ok":false,"error":"not_found"}"#,
        );
        return;
    }

    let mut body = String::new();
    if request
        .as_reader()
        .take(32 * 1024)
        .read_to_string(&mut body)
        .is_err()
    {
        respond(
            request,
            StatusCode(400),
            r#"{"ok":false,"error":"bad_body"}"#,
        );
        return;
    }

    let payload = match serde_json::from_str::<IncomingPayload>(&body) {
        Ok(payload) => payload,
        Err(_) => {
            respond(
                request,
                StatusCode(400),
                r#"{"ok":false,"error":"bad_json"}"#,
            );
            return;
        }
    };

    if !device_id_is_valid(&payload, state) {
        respond(request, StatusCode(401), r#"{"ok":false,"error":"bad_id"}"#);
        return;
    }

    let message = message_from_payload(payload, origin, state);
    match message {
        Ok(message) => {
            store_message(app, state, message);
            respond(request, StatusCode(200), r#"{"ok":true}"#);
        }
        Err((status, body)) => respond(request, status, body),
    }
}

fn message_from_payload(
    payload: IncomingPayload,
    origin: String,
    state: &Arc<AppState>,
) -> Result<IncomingMessage, (StatusCode, &'static str)> {
    let text = payload
        .text
        .or(payload.message)
        .unwrap_or_default()
        .trim()
        .to_string();

    if text.is_empty()
        && payload
            .code
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    {
        return Err((StatusCode(400), r#"{"ok":false,"error":"empty_message"}"#));
    }

    let code = payload
        .code
        .and_then(|code| normalize_code(&code))
        .or_else(|| extract_code(&text));
    let copied_text = code.clone().unwrap_or_else(|| text.clone());

    let id_for_sender = payload.device_id.clone();
    Ok(IncomingMessage {
        id: Uuid::new_v4().to_string(),
        sender: resolve_sender(payload.sender, id_for_sender.as_deref(), state),
        text,
        code,
        copied_text,
        received_at: Utc::now().to_rfc3339(),
        remote_addr: origin,
    })
}

fn message_from_relay(
    relay_message: RelayMessage,
    state: &Arc<AppState>,
) -> Option<IncomingMessage> {
    let device_id = relay_message.device_id.trim().to_string();
    let text = relay_message.text.trim().to_string();
    let code = normalize_code(&relay_message.code).or_else(|| extract_code(&text));
    let copied_text = code.clone().unwrap_or_else(|| text.clone());
    let received_at = if relay_message.received_at.trim().is_empty() {
        Utc::now().to_rfc3339()
    } else {
        relay_message.received_at
    };

    Some(IncomingMessage {
        id: if relay_message.relay_id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            relay_message.relay_id
        },
        sender: resolve_sender(Some(relay_message.sender), Some(device_id.as_str()), state),
        text,
        code,
        copied_text,
        received_at,
        remote_addr: relay_message.remote_addr,
    })
}

fn store_message(app: &AppHandle, state: &Arc<AppState>, message: IncomingMessage) {
    {
        let mut messages = state.messages.lock().expect("message state poisoned");
        if messages.iter().any(|existing| existing.id == message.id) {
            return;
        }
        messages.push_front(message.clone());
        while messages.len() > MAX_MESSAGES {
            messages.pop_back();
        }
    }

    if let Err(error) = persist_messages(state) {
        let _ = app.emit("receiver-error", format!("保存本地消息失败: {}", error));
    }
    if should_show_notification(state, &message) {
        show_message_notification(app, &message, current_notification_position(state));
    }
    let _ = app.emit("message-received", message);
}

fn show_message_notification(
    app: &AppHandle,
    message: &IncomingMessage,
    mut position: NotificationPosition,
) {
    let app_for_thread = app.clone();
    let app = app.clone();
    let message = message.clone();
    if cfg!(target_os = "macos") {
        position = NotificationPosition::TopRight;
    }

    let _ = app_for_thread.run_on_main_thread(move || {
        let window = match ensure_notification_window(&app) {
            Ok(window) => window,
            Err(error) => {
                let _ = app.emit("receiver-error", format!("通知窗口创建失败: {}", error));
                return;
            }
        };

        let _ = position_notification_window(&window, position);
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let _ = app.emit_to(
            EventTarget::webview_window(NOTIFICATION_LABEL),
            "notification-message",
            message,
        );
    });
}

fn ensure_notification_window(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window(NOTIFICATION_LABEL) {
        return Ok(window);
    }

    let builder = WebviewWindowBuilder::new(
        app,
        NOTIFICATION_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("验证码接收器")
    .inner_size(NOTIFICATION_WIDTH, NOTIFICATION_HEIGHT)
    .resizable(false)
    .decorations(false);

    #[cfg(not(target_os = "macos"))]
    let builder = builder.transparent(true);

    builder
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .build()
}

fn position_notification_window(
    window: &tauri::WebviewWindow,
    position: NotificationPosition,
) -> tauri::Result<()> {
    let monitor = window
        .current_monitor()?
        .or(window.primary_monitor()?)
        .or_else(|| {
            window
                .available_monitors()
                .ok()
                .and_then(|mut monitors| monitors.pop())
        });

    if let Some(monitor) = monitor {
        let work_area = monitor.work_area();
        let scale_factor = monitor.scale_factor();
        let physical_width = (NOTIFICATION_WIDTH * scale_factor).round() as i32;
        let physical_height = (NOTIFICATION_HEIGHT * scale_factor).round() as i32;
        let x = match position {
            NotificationPosition::BottomRight | NotificationPosition::TopRight => {
                work_area.position.x + work_area.size.width as i32
                    - physical_width
                    - NOTIFICATION_MARGIN
            }
            NotificationPosition::BottomLeft | NotificationPosition::TopLeft => {
                work_area.position.x + NOTIFICATION_MARGIN
            }
            NotificationPosition::TopCenter => {
                work_area.position.x + ((work_area.size.width as i32 - physical_width) / 2)
            }
        };
        let y = match position {
            NotificationPosition::BottomRight | NotificationPosition::BottomLeft => {
                work_area.position.y + work_area.size.height as i32
                    - physical_height
                    - NOTIFICATION_MARGIN
            }
            NotificationPosition::TopRight
            | NotificationPosition::TopLeft
            | NotificationPosition::TopCenter => work_area.position.y + NOTIFICATION_MARGIN,
        };
        window.set_position(PhysicalPosition::new(x, y))?;
    }

    window.set_size(LogicalSize::new(NOTIFICATION_WIDTH, NOTIFICATION_HEIGHT))?;
    Ok(())
}

fn load_messages(path: PathBuf) -> VecDeque<IncomingMessage> {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Vec<IncomingMessage>>(&content).ok())
        .map(|messages| messages.into_iter().take(MAX_MESSAGES).collect())
        .unwrap_or_default()
}

fn load_settings(path: PathBuf) -> AppSettings {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<AppSettings>(&content).ok())
        .map(normalize_loaded_settings)
        .filter(|settings| validate_port(settings.port).is_ok())
        .unwrap_or(AppSettings {
            port: DEFAULT_SERVER_PORT,
            receiver_enabled: default_receiver_enabled(),
            notification_mode: default_notification_mode(),
            notification_enabled: None,
            notification_position: default_notification_position(),
            direct_paste_enabled: false,
            relay: RelaySettings::default(),
            sender_devices: vec![default_sender_device()],
        })
}

fn persist_settings(state: &Arc<AppState>) -> Result<(), String> {
    let settings = AppSettings {
        port: current_port(state),
        receiver_enabled: current_receiver_enabled(state),
        notification_mode: current_notification_mode(state),
        notification_enabled: None,
        notification_position: current_notification_position(state),
        direct_paste_enabled: direct_paste_is_enabled(state),
        relay: current_relay_settings(state),
        sender_devices: current_sender_devices(state),
    };

    if let Some(parent) = state.settings_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(&state.settings_path, content).map_err(|error| error.to_string())
}

fn normalize_loaded_settings(mut settings: AppSettings) -> AppSettings {
    if let Some(enabled) = settings.notification_enabled {
        settings.notification_mode = if enabled {
            NotificationMode::Verification
        } else {
            NotificationMode::Off
        };
        settings.notification_enabled = None;
    }

    settings
}

fn default_notification_mode() -> NotificationMode {
    NotificationMode::Verification
}

fn default_receiver_enabled() -> bool {
    true
}

fn default_notification_position() -> NotificationPosition {
    NotificationPosition::BottomRight
}

#[cfg(target_os = "macos")]
fn ensure_direct_paste_permission() -> Result<(), String> {
    if macos_accessibility_permission_granted() {
        return Ok(());
    }

    open_macos_accessibility_settings();
    Err("请在系统设置中为验证码接收器开启辅助功能权限后，再启用直接输入".to_string())
}

#[cfg(not(target_os = "macos"))]
fn ensure_direct_paste_permission() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_accessibility_permission_granted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(target_os = "macos")]
fn open_macos_accessibility_settings() {
    let opened = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !opened {
        let _ = std::process::Command::new("open")
            .arg("/System/Library/PreferencePanes/Security.prefPane")
            .status();
    }
}

#[cfg(target_os = "windows")]
fn type_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    let mut inputs: Vec<INPUT> = text
        .encode_utf16()
        .flat_map(|unit| {
            [
                unicode_input(unit, KEYEVENTF_UNICODE),
                unicode_input(unit, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP),
            ]
        })
        .collect();
    let sent = unsafe { SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32) };

    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err("模拟输入失败".to_string())
    }
}

#[cfg(target_os = "macos")]
fn type_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    let status = std::process::Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .status()
        .map_err(|error| format!("执行 Command+V 失败: {}", error))?;

    if status.success() {
        Ok(())
    } else {
        Err("执行 Command+V 失败，请检查辅助功能权限".to_string())
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn type_text(_text: &str) -> Result<(), String> {
    Err("当前平台暂不支持直接输入".to_string())
}

#[cfg(target_os = "windows")]
fn unicode_input(unit: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn default_sender() -> String {
    DEFAULT_DEVICE_NAME.to_string()
}

fn default_sender_device() -> SenderDevice {
    SenderDevice {
        id: DEFAULT_DEVICE_ID.to_string(),
        name: DEFAULT_DEVICE_NAME.to_string(),
        device_id: generate_device_id(),
    }
}

fn normalize_device_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        default_sender()
    } else {
        name.to_string()
    }
}

fn resolve_sender(
    request_sender: Option<String>,
    request_device_id: Option<&str>,
    state: &Arc<AppState>,
) -> String {
    request_device_id
        .and_then(|device_id| sender_for_device_id(device_id, state))
        .unwrap_or_else(|| normalize_request_sender(request_sender))
}

fn normalize_request_sender(sender: Option<String>) -> String {
    sender
        .map(|sender| sender.trim().to_string())
        .filter(|sender| !sender.is_empty())
        .unwrap_or_else(default_sender)
}

fn sender_for_device_id(device_id: &str, state: &Arc<AppState>) -> Option<String> {
    let device_id = device_id.trim();
    if device_id.is_empty() {
        return None;
    }

    current_sender_devices(state)
        .into_iter()
        .find(|device| device.device_id == device_id)
        .map(|device| device.name)
}

fn persist_messages(state: &Arc<AppState>) -> Result<(), String> {
    let messages: Vec<IncomingMessage> = state
        .messages
        .lock()
        .map_err(|_| "消息状态不可用".to_string())?
        .iter()
        .cloned()
        .collect();

    if let Some(parent) = state.storage_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(&messages).map_err(|error| error.to_string())?;
    fs::write(&state.storage_path, content).map_err(|error| error.to_string())
}

fn respond(request: tiny_http::Request, status: StatusCode, body: &str) {
    let mut response = Response::from_string(body.to_string()).with_status_code(status);
    for (name, value) in [
        ("Content-Type", "application/json; charset=utf-8"),
        ("Access-Control-Allow-Origin", "*"),
        ("Access-Control-Allow-Headers", "content-type"),
        ("Access-Control-Allow-Methods", "POST, OPTIONS"),
    ] {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response = response.with_header(header);
        }
    }
    let _ = request.respond(response);
}

fn device_id_is_valid(payload: &IncomingPayload, state: &Arc<AppState>) -> bool {
    payload
        .device_id
        .as_deref()
        .and_then(|device_id| sender_for_device_id(device_id, state))
        .is_some()
}

fn normalize_code(value: &str) -> Option<String> {
    let code = value.trim();
    if code.is_empty() {
        None
    } else {
        Some(code.to_string())
    }
}

fn extract_code(text: &str) -> Option<String> {
    static CODE_RE: OnceLock<Regex> = OnceLock::new();
    CODE_RE
        .get_or_init(|| Regex::new(r"(?:^|[^\d])(\d{4,8})(?:[^\d]|$)").expect("valid code regex"))
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|matched| matched.as_str().to_string())
}

fn message_is_verification_notice(message: &IncomingMessage) -> bool {
    let content = format!("{} {}", message.text, message.copied_text);
    message_has_verification_keyword(&content) && message_has_short_code(&content)
}

fn message_has_verification_keyword(content: &str) -> bool {
    content.contains("验证码") || content.contains("校验码")
}

fn message_has_short_code(content: &str) -> bool {
    static SHORT_CODE_RE: OnceLock<Regex> = OnceLock::new();
    SHORT_CODE_RE
        .get_or_init(|| {
            Regex::new(r"(?:^|[^\d])\d{4,6}(?:[^\d]|$)").expect("valid short code regex")
        })
        .is_match(content)
}

fn local_ipv4() -> Option<String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(("8.8.8.8", 80)).ok()?;
    let local_addr = socket.local_addr().ok()?;
    match local_addr.ip() {
        std::net::IpAddr::V4(ip) if !ip.is_loopback() => Some(ip.to_string()),
        _ => None,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(move |app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("derek-msg-sync"));
            let storage_path = app_data_dir.join("messages.json");
            let settings_path = app_data_dir.join("settings.json");
            let settings = load_settings(settings_path.clone());
            let sender_devices = normalize_sender_devices(settings.sender_devices);
            let state = Arc::new(AppState {
                messages: Mutex::new(load_messages(storage_path.clone())),
                storage_path,
                settings_path,
                port: Mutex::new(settings.port),
                receiver_enabled: Mutex::new(settings.receiver_enabled),
                notification_mode: Mutex::new(settings.notification_mode),
                notification_position: Mutex::new(settings.notification_position),
                direct_paste_enabled: Mutex::new(settings.direct_paste_enabled),
                relay: Mutex::new(settings.relay),
                relay_running: Mutex::new(false),
                sender_devices: Mutex::new(sender_devices),
                receiver: Mutex::new(None),
                log_path: app_data_dir.join("derek-msg-sync.log"),
                log_lock: Mutex::new(()),
            });

            app.manage(state.clone());
            if let Err(error) = persist_settings(&state) {
                let _ = app.emit("receiver-error", format!("保存本地设置失败: {}", error));
            }
            setup_tray(app.handle())?;
            if let Err(error) = ensure_notification_window(app.handle()) {
                let _ = app.emit("receiver-error", format!("通知窗口初始化失败: {}", error));
            }
            if let Some(window) = app.get_webview_window("main") {
                #[cfg(target_os = "macos")]
                {
                    let _ = window.set_decorations(true);
                    let _ = window.set_title_bar_style(TitleBarStyle::Overlay);
                }
                let app_for_close = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        hide_main_window(&app_for_close);
                    }
                });
            }
            if current_receiver_enabled(&state) {
                match start_receiver(app.handle().clone(), state.clone()) {
                    Ok(()) if receiver_is_running(&state) => hide_main_window(app.handle()),
                    Ok(()) => {}
                    Err(error) => {
                        let _ = app.emit("receiver-error", error);
                    }
                }
            }
            if current_relay_settings(&state).enabled {
                if let Err(error) = start_relay_client(app.handle().clone(), state.clone()) {
                    let _ = app.emit("receiver-error", error);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_messages,
            clear_messages,
            platform_info,
            open_settings,
            hide_main_window_command,
            receiver_status,
            set_notification_mode,
            set_notification_position,
            set_direct_paste_enabled,
            set_relay_settings,
            test_relay_connection,
            set_sender_devices,
            set_receiver_port,
            type_verification_code,
            start_receiver_command,
            stop_receiver,
            log_message
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_main_window(app: &AppHandle) {
    restore_macos_app_icon(app);

    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(true);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }

    restore_macos_app_icon(app);
}

fn hide_main_window(app: &AppHandle) {
    restore_macos_app_icon(app);

    #[cfg(target_os = "macos")]
    let _ = app.set_dock_visibility(false);

    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[cfg(target_os = "macos")]
fn restore_macos_app_icon(app: &AppHandle) {
    let _ = app.run_on_main_thread(|| {
        use objc2::{AllocAnyThread, MainThreadMarker};
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;

        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        let ns_app = NSApplication::sharedApplication(mtm);
        let icon_data = NSData::with_bytes(include_bytes!("../icons/icon.png"));

        if let Some(icon) = NSImage::initWithData(NSImage::alloc(), &icon_data) {
            unsafe {
                ns_app.setApplicationIconImage(Some(&icon));
            }
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn restore_macos_app_icon(_app: &AppHandle) {
}

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_poll_response_accepts_camel_case_message_fields() {
        let json = r#"{
            "messages": [
                {
                    "relayId": "relay-1",
                    "sender": "iPhone",
                    "text": "您的验证码是 215164，5 分钟内有效",
                    "code": "215164",
                    "id": "1234567",
                    "receivedAt": "2026-05-10T17:17:50.821834468Z",
                    "remoteAddr": "74.48.90.147"
                }
            ]
        }"#;

        let payload: RelayPollResponse =
            serde_json::from_str(json).expect("relay poll payload should parse");

        assert_eq!(payload.messages.len(), 1);
        let message = &payload.messages[0];
        assert_eq!(message.relay_id, "relay-1");
        assert_eq!(message.received_at, "2026-05-10T17:17:50.821834468Z");
        assert_eq!(message.remote_addr, "74.48.90.147");
    }

    #[test]
    fn receiver_port_release_wakes_unspecified_listener() {
        let server = Arc::new(Server::http((Ipv4Addr::UNSPECIFIED, 0)).unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let server_for_thread = server.clone();

        let handle = thread::spawn(move || {
            let _ = server_for_thread.recv_timeout(Duration::from_secs(10));
        });

        server.unblock();
        drop(server);
        wake_receiver_listener(port);
        handle.join().unwrap();

        for _ in 0..RECEIVER_STOP_RELEASE_ATTEMPTS {
            if receiver_port_is_available(port) {
                return;
            }
            thread::sleep(RECEIVER_STOP_RELEASE_DELAY);
        }

        panic!("receiver port should be available after stop");
    }
}
