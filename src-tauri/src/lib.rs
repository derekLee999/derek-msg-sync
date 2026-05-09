use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashSet, VecDeque},
    fs,
    io::Read,
    net::{Ipv4Addr, UdpSocket},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, EventTarget, LogicalSize, Manager, PhysicalPosition, State, WebviewUrl,
    WebviewWindowBuilder, WindowEvent,
};
use tiny_http::{Header, Method, Response, Server, StatusCode};
use uuid::Uuid;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY,
};

const DEFAULT_SERVER_PORT: u16 = 17866;
const DEFAULT_DEVICE_ID: &str = "default-iphone";
const DEFAULT_DEVICE_NAME: &str = "iPhone";
const MIN_SERVER_PORT: u16 = 1024;
const MAX_SENDER_DEVICES: usize = 5;
const MAX_MESSAGES: usize = 100;
const TRAY_EXIT_ID: &str = "quit";
const NOTIFICATION_LABEL: &str = "message-toast";
const NOTIFICATION_WIDTH: f64 = 380.0;
const NOTIFICATION_HEIGHT: f64 = 116.0;
const NOTIFICATION_MARGIN: i32 = 18;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum NotificationPosition {
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
    TopCenter,
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
    notification_enabled: bool,
    notification_position: NotificationPosition,
    direct_paste_enabled: bool,
    sender_devices: Vec<SenderDevice>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
    port: u16,
    #[serde(default = "default_notification_enabled")]
    notification_enabled: bool,
    #[serde(default = "default_notification_position")]
    notification_position: NotificationPosition,
    #[serde(default)]
    direct_paste_enabled: bool,
    #[serde(default)]
    sender_devices: Vec<SenderDevice>,
}

struct AppState {
    messages: Mutex<VecDeque<IncomingMessage>>,
    storage_path: PathBuf,
    settings_path: PathBuf,
    port: Mutex<u16>,
    notification_enabled: Mutex<bool>,
    notification_position: Mutex<NotificationPosition>,
    direct_paste_enabled: Mutex<bool>,
    sender_devices: Mutex<Vec<SenderDevice>>,
    receiver: Mutex<Option<Arc<Server>>>,
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
        notification_enabled: notifications_are_enabled(&state),
        notification_position: current_notification_position(&state),
        direct_paste_enabled: direct_paste_is_enabled(&state),
        sender_devices: current_sender_devices(&state),
    }
}

#[tauri::command]
fn set_notification_enabled(enabled: bool, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let state = state.inner().clone();
    *state
        .notification_enabled
        .lock()
        .map_err(|_| "通知设置不可用".to_string())? = enabled;
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
    let state = state.inner().clone();
    *state
        .direct_paste_enabled
        .lock()
        .map_err(|_| "直接输入设置不可用".to_string())? = enabled;
    persist_settings(&state)
}

#[tauri::command]
fn set_sender_devices(
    devices: Vec<SenderDevice>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if devices.len() > MAX_SENDER_DEVICES {
        return Err(format!("最多只能添加 {} 个设备", MAX_SENDER_DEVICES));
    }

    let state = state.inner().clone();
    *state
        .sender_devices
        .lock()
        .map_err(|_| "设备设置不可用".to_string())? = normalize_sender_devices(devices);
    persist_settings(&state)
}

#[tauri::command]
fn start_receiver_command(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    start_receiver(app, state.inner().clone())
}

#[tauri::command]
fn stop_receiver(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    stop_receiver_inner(state.inner())
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
        stop_receiver_inner(&state)?;
        thread::sleep(Duration::from_millis(350));
    }

    *state
        .port
        .lock()
        .map_err(|_| "端口设置不可用".to_string())? = port;
    persist_settings(&state)?;

    if was_running {
        start_receiver(app, state)?;
    }

    Ok(())
}

fn stop_receiver_inner(state: &Arc<AppState>) -> Result<(), String> {
    let server = state
        .receiver
        .lock()
        .map_err(|_| "接收服务状态不可用".to_string())?
        .take();

    if let Some(server) = server {
        server.unblock();
    }

    Ok(())
}

fn start_receiver(app: AppHandle, state: Arc<AppState>) -> Result<(), String> {
    if receiver_is_running(&state) {
        return Ok(());
    }

    let port = current_port(&state);
    let server = Arc::new(
        Server::http(("0.0.0.0", port)).map_err(|error| format!("接收服务启动失败: {}", error))?,
    );

    *state
        .receiver
        .lock()
        .map_err(|_| "接收服务状态不可用".to_string())? = Some(server.clone());

    thread::spawn(move || loop {
        if !receiver_is_running(&state) {
            break;
        }

        match server.recv_timeout(Duration::from_millis(250)) {
            Ok(Some(request)) => handle_request(request, &app, &state),
            Ok(None) => {}
            Err(_) => break,
        }
    });

    Ok(())
}

fn current_port(state: &Arc<AppState>) -> u16 {
    state
        .port
        .lock()
        .map(|port| *port)
        .unwrap_or(DEFAULT_SERVER_PORT)
}

fn validate_port(port: u16) -> Result<(), String> {
    if port < MIN_SERVER_PORT {
        return Err(format!("端口号需在 {}-65535 之间", MIN_SERVER_PORT));
    }

    Ok(())
}

fn notifications_are_enabled(state: &Arc<AppState>) -> bool {
    state
        .notification_enabled
        .lock()
        .map(|enabled| *enabled)
        .unwrap_or(true)
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

fn current_sender_devices(state: &Arc<AppState>) -> Vec<SenderDevice> {
    state
        .sender_devices
        .lock()
        .map(|devices| devices.clone())
        .unwrap_or_default()
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
    let quit = MenuItem::with_id(app, TRAY_EXIT_ID, "退出程序", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit])?;
    let icon = Image::from_bytes(include_bytes!("../icons/icon.ico"))?;

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("验证码接收器")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id().as_ref() == TRAY_EXIT_ID {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
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
        respond(
            request,
            StatusCode(400),
            r#"{"ok":false,"error":"empty_message"}"#,
        );
        return;
    }

    let code = payload
        .code
        .and_then(|code| normalize_code(&code))
        .or_else(|| extract_code(&text));
    let copied_text = code.clone().unwrap_or_else(|| text.clone());

    let id_for_sender = payload.device_id.clone();
    let message = IncomingMessage {
        id: Uuid::new_v4().to_string(),
        sender: resolve_sender(payload.sender, id_for_sender.as_deref(), state),
        text,
        code,
        copied_text,
        received_at: Utc::now().to_rfc3339(),
        remote_addr: origin,
    };

    {
        let mut messages = state.messages.lock().expect("message state poisoned");
        messages.push_front(message.clone());
        while messages.len() > MAX_MESSAGES {
            messages.pop_back();
        }
    }

    if let Err(error) = persist_messages(state) {
        let _ = app.emit("receiver-error", format!("保存本地消息失败: {}", error));
    }
    if notifications_are_enabled(state) {
        show_message_notification(app, &message, current_notification_position(state));
    }
    let _ = app.emit("message-received", message);
    respond(request, StatusCode(200), r#"{"ok":true}"#);
}

fn show_message_notification(
    app: &AppHandle,
    message: &IncomingMessage,
    position: NotificationPosition,
) {
    let app_for_thread = app.clone();
    let app = app.clone();
    let message = message.clone();

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

    WebviewWindowBuilder::new(
        app,
        NOTIFICATION_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("验证码接收器")
    .inner_size(NOTIFICATION_WIDTH, NOTIFICATION_HEIGHT)
    .resizable(false)
    .decorations(false)
    .transparent(true)
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
            | NotificationPosition::TopCenter => {
                work_area.position.y + NOTIFICATION_MARGIN
            }
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
        .filter(|settings| validate_port(settings.port).is_ok())
        .unwrap_or(AppSettings {
            port: DEFAULT_SERVER_PORT,
            notification_enabled: default_notification_enabled(),
            notification_position: default_notification_position(),
            direct_paste_enabled: false,
            sender_devices: vec![default_sender_device()],
        })
}

fn persist_settings(state: &Arc<AppState>) -> Result<(), String> {
    let settings = AppSettings {
        port: current_port(state),
        notification_enabled: notifications_are_enabled(state),
        notification_position: current_notification_position(state),
        direct_paste_enabled: direct_paste_is_enabled(state),
        sender_devices: current_sender_devices(state),
    };

    if let Some(parent) = state.settings_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    fs::write(&state.settings_path, content).map_err(|error| error.to_string())
}

fn default_notification_enabled() -> bool {
    true
}

fn default_notification_position() -> NotificationPosition {
    NotificationPosition::BottomRight
}

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
                notification_enabled: Mutex::new(settings.notification_enabled),
                notification_position: Mutex::new(settings.notification_position),
                direct_paste_enabled: Mutex::new(settings.direct_paste_enabled),
                sender_devices: Mutex::new(sender_devices),
                receiver: Mutex::new(None),
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
                let window_for_close = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_for_close.hide();
                    }
                });
            }
            if let Err(error) = start_receiver(app.handle().clone(), state.clone()) {
                let _ = app.emit("receiver-error", error);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_messages,
            clear_messages,
            receiver_status,
            set_notification_enabled,
            set_notification_position,
            set_direct_paste_enabled,
            set_sender_devices,
            set_receiver_port,
            type_verification_code,
            start_receiver_command,
            stop_receiver
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
