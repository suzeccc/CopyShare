use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, OnceLock},
};

#[cfg(test)]
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{async_runtime::JoinHandle, AppHandle, Emitter};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{watch, Mutex},
};
use uuid::Uuid;

use crate::{
    clipboard,
    error::{AppError, AppResult},
    history,
    models::{
        ClipboardContentType, ClipboardMessage, ClipboardTextItem, HistoryDirection, HistoryItem,
        MobileSessionMode, MobileSessionPhase, MobileSessionView, SyncStatus,
    },
    network,
    notifications,
    state::AppState,
    sync,
};

pub const MOBILE_HTTP_PORT: u16 = 8766;
pub const MOBILE_TEXT_LIMIT_BYTES: usize = 100 * 1024;
#[cfg(test)]
pub const MOBILE_SESSION_TTL: Duration = Duration::from_secs(15 * 60);
pub const MOBILE_DEVICE_NAME: &str = "移动端";
const MOBILE_PC_ITEM_LIMIT: usize = 20;
const MAX_RETAINED_MOBILE_SESSIONS: usize = 10;
const DEFAULT_PC_SOURCE_DEVICE: &str = "CopyShare";

static MOBILE_RUNTIME: OnceLock<MobileRuntime> = OnceLock::new();

struct MobileRuntime {
    sessions: Arc<Mutex<MobileSessionStore>>,
    server: Mutex<Option<MobileHttpServer>>,
}

struct MobileHttpServer {
    address: SocketAddr,
    stop: watch::Sender<bool>,
    join: JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct MobileSession {
    id: String,
    token: String,
    mode: MobileSessionMode,
    phase: MobileSessionPhase,
    content_items: Vec<ClipboardTextItem>,
    submitted_items: Vec<ClipboardTextItem>,
    expires_at: Option<DateTime<Utc>>,
    token_consumed: bool,
}

#[derive(Debug)]
pub struct MobileSessionStore {
    host: String,
    port: u16,
    sessions: HashMap<String, MobileSession>,
    session_order: VecDeque<String>,
    test_clock_offset: chrono::Duration,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MobilePostBody {
    action: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MobileContentResponse {
    id: String,
    mode: MobileSessionMode,
    phase: MobileSessionPhase,
    summary: String,
    content: Option<String>,
    content_items: Vec<ClipboardTextItem>,
    submitted_items: Vec<ClipboardTextItem>,
    remaining_seconds: Option<i64>,
}

impl MobileSessionStore {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            sessions: HashMap::new(),
            session_order: VecDeque::new(),
            test_clock_offset: chrono::Duration::zero(),
        }
    }

    #[cfg(test)]
    pub fn new_for_tests(host: impl Into<String>, port: u16) -> Self {
        Self::new(host, port)
    }

    pub fn set_address(&mut self, address: SocketAddr) {
        self.host = address.ip().to_string();
        self.port = address.port();
    }

    #[cfg(test)]
    pub fn create_send_session(&mut self, content: String) -> AppResult<MobileSessionView> {
        ensure_mobile_text_size(&content)?;
        self.close_open_sessions();
        let session = self.new_session(
            MobileSessionMode::SendToMobile,
            vec![content],
            DEFAULT_PC_SOURCE_DEVICE.to_string(),
        );
        let view = self.session_view(&session);
        self.insert_session(session);
        Ok(view)
    }

    #[cfg(test)]
    pub fn create_receive_session(&mut self) -> AppResult<MobileSessionView> {
        self.close_open_sessions();
        let session = self.new_session(
            MobileSessionMode::ReceiveFromMobile,
            Vec::new(),
            DEFAULT_PC_SOURCE_DEVICE.to_string(),
        );
        let view = self.session_view(&session);
        self.insert_session(session);
        Ok(view)
    }

    #[cfg(test)]
    pub fn create_session(&mut self, contents: Vec<String>) -> AppResult<MobileSessionView> {
        self.create_session_with_source(contents, DEFAULT_PC_SOURCE_DEVICE.to_string())
    }

    pub fn create_session_with_source(
        &mut self,
        contents: Vec<String>,
        source_device: String,
    ) -> AppResult<MobileSessionView> {
        ensure_mobile_text_items_size(&contents)?;
        self.close_open_sessions();
        let session = self.new_session(MobileSessionMode::Bidirectional, contents, source_device);
        let view = self.session_view(&session);
        self.insert_session(session);
        Ok(view)
    }

    #[cfg(test)]
    pub fn append_pc_clipboard_text(&mut self, text: String) -> AppResult<()> {
        self.append_pc_clipboard_text_with_source(text, DEFAULT_PC_SOURCE_DEVICE.to_string())
    }

    pub fn append_pc_clipboard_text_with_source(
        &mut self,
        text: String,
        source_device: String,
    ) -> AppResult<()> {
        let text = text.trim().to_string();
        if text.is_empty() {
            return Ok(());
        }
        ensure_mobile_text_size(&text)?;
        let source_device = normalize_source_device(source_device, DEFAULT_PC_SOURCE_DEVICE);

        for session in self.sessions.values_mut() {
            if session_is_finished(&session.phase) {
                continue;
            }
            if !matches!(
                session.mode,
                MobileSessionMode::SendToMobile | MobileSessionMode::Bidirectional
            ) {
                continue;
            }

            upsert_mobile_content_item(
                &mut session.content_items,
                text.clone(),
                source_device.clone(),
            );
        }

        Ok(())
    }

    pub fn get_session_view(&mut self, id: &str) -> AppResult<MobileSessionView> {
        let session = self
            .sessions
            .get(id)
            .ok_or_else(|| AppError::InvalidInput("手机连接会话不存在".to_string()))?;
        Ok(self.session_view(session))
    }

    pub fn load_send_content_items(
        &mut self,
        id: &str,
        token: &str,
    ) -> AppResult<Vec<ClipboardTextItem>> {
        self.validate_token(id, token)?;
        let session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| AppError::InvalidInput("?????????".to_string()))?;
        if !matches!(session.mode, MobileSessionMode::SendToMobile | MobileSessionMode::Bidirectional) {
            return Err(AppError::InvalidInput("?????????????".to_string()));
        }
        if session.phase == MobileSessionPhase::Waiting {
            session.phase = MobileSessionPhase::Opened;
        }
        Ok(session.content_items.clone())
    }

    #[cfg(test)]
    pub fn load_send_content(&mut self, id: &str, token: &str) -> AppResult<String> {
        Ok(self
            .load_send_content_items(id, token)?
            .into_iter()
            .next()
            .map(|item| item.text)
            .unwrap_or_default())
    }

    pub fn mark_send_copied(&mut self, id: &str, token: &str) -> AppResult<MobileSessionView> {
        self.validate_token(id, token)?;
        let view_session = {
            let session = self
                .sessions
                .get_mut(id)
                .ok_or_else(|| AppError::InvalidInput("手机连接会话不存在".to_string()))?;
            if !matches!(session.mode, MobileSessionMode::SendToMobile | MobileSessionMode::Bidirectional) {
                return Err(AppError::InvalidInput("此二维码不是发送到手机会话".to_string()));
            }
            if !matches!(session.phase, MobileSessionPhase::Submitted | MobileSessionPhase::Written) {
                session.phase = MobileSessionPhase::Copied;
            }
            session.clone()
        };
        Ok(self.session_view(&view_session))
    }

    pub fn submit_mobile_content(
        &mut self,
        id: &str,
        token: &str,
        content: String,
    ) -> AppResult<MobileSessionView> {
        ensure_mobile_text_size(&content)?;
        self.validate_token(id, token)?;
        if content.trim().is_empty() {
            return Err(AppError::InvalidInput("????????".to_string()));
        }
        let view_session = {
            let session = self
                .sessions
                .get_mut(id)
                .ok_or_else(|| AppError::InvalidInput("?????????".to_string()))?;
            if !matches!(session.mode, MobileSessionMode::ReceiveFromMobile | MobileSessionMode::Bidirectional) {
                return Err(AppError::InvalidInput("??????????????".to_string()));
            }
            session
                .submitted_items
                .push(mobile_text_item(content, MOBILE_DEVICE_NAME.to_string()));
            session.phase = MobileSessionPhase::Submitted;
            session.clone()
        };
        Ok(self.session_view(&view_session))
    }

    pub fn close_session(&mut self, id: &str) -> AppResult<MobileSessionView> {
        let view_session = {
            let session = self
                .sessions
                .get_mut(id)
                .ok_or_else(|| AppError::InvalidInput("手机连接会话不存在".to_string()))?;
            session.phase = MobileSessionPhase::Closed;
            session.clone()
        };
        self.prune_finished_sessions();
        Ok(self.session_view(&view_session))
    }

    pub fn take_submitted_content_for_write(&mut self, id: &str) -> AppResult<String> {
        let session = self
            .sessions
            .get_mut(id)
            .ok_or_else(|| AppError::InvalidInput("?????????".to_string()))?;
        reject_closed_session(session)?;
        if !matches!(session.mode, MobileSessionMode::ReceiveFromMobile | MobileSessionMode::Bidirectional) {
            return Err(AppError::InvalidInput("??????????????".to_string()));
        }
        if session.phase != MobileSessionPhase::Submitted {
            return Err(AppError::InvalidInput("?????????".to_string()));
        }
        let content = session
            .submitted_items
            .last()
            .map(|item| item.text.clone())
            .ok_or_else(|| AppError::InvalidInput("?????????".to_string()))?;
        session.phase = MobileSessionPhase::Written;
        Ok(content)
    }

    pub fn mark_submitted_content_written(&mut self, id: &str) -> AppResult<MobileSessionView> {
        let view_session = {
            let session = self
                .sessions
                .get_mut(id)
                .ok_or_else(|| AppError::InvalidInput("?????????".to_string()))?;
            reject_closed_session(session)?;
            if !matches!(session.mode, MobileSessionMode::ReceiveFromMobile | MobileSessionMode::Bidirectional) {
                return Err(AppError::InvalidInput("??????????????".to_string()));
            }
            if session.submitted_items.is_empty() {
                return Err(AppError::InvalidInput("?????????".to_string()));
            }
            if session.phase != MobileSessionPhase::Written {
                if session.phase != MobileSessionPhase::Submitted {
                    return Err(AppError::InvalidInput("?????????".to_string()));
                }
                session.phase = MobileSessionPhase::Written;
            }
            if matches!(
                session.mode,
                MobileSessionMode::SendToMobile | MobileSessionMode::Bidirectional
            ) {
                if let Some(item) = session.submitted_items.last().cloned() {
                    upsert_mobile_content_item(
                        &mut session.content_items,
                        item.text,
                        item.source_device,
                    );
                }
            }
            session.clone()
        };
        Ok(self.session_view(&view_session))
    }

    #[cfg(test)]
    pub fn test_token(&self, id: &str) -> Option<String> {
        self.sessions.get(id).map(|session| session.token.clone())
    }

    #[cfg(test)]
    pub fn advance_test_clock(&mut self, duration: Duration) {
        self.test_clock_offset += chrono::Duration::from_std(duration).unwrap();
    }

    fn new_session(
        &self,
        mode: MobileSessionMode,
        contents: Vec<String>,
        source_device: String,
    ) -> MobileSession {
        let source_device = normalize_source_device(source_device, DEFAULT_PC_SOURCE_DEVICE);
        MobileSession {
            id: Uuid::new_v4().simple().to_string(),
            token: Uuid::new_v4().simple().to_string(),
            mode,
            phase: MobileSessionPhase::Waiting,
            content_items: contents
                .into_iter()
                .map(|text| mobile_text_item(text, source_device.clone()))
                .collect(),
            submitted_items: Vec::new(),
            expires_at: None,
            token_consumed: false,
        }
    }

    fn validate_token(&mut self, id: &str, token: &str) -> AppResult<()> {
        let session = self
            .sessions
            .get(id)
            .ok_or_else(|| AppError::InvalidInput("手机连接会话不存在".to_string()))?;
        reject_closed_session(session)?;
        if session.token != token {
            return Err(AppError::InvalidInput("二维码 token 无效".to_string()));
        }
        if session.token_consumed {
            return Err(AppError::InvalidInput("此二维码已失效".to_string()));
        }
        Ok(())
    }

    fn insert_session(&mut self, session: MobileSession) {
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        self.session_order.push_back(id);
        self.prune_finished_sessions();
    }

    fn prune_finished_sessions(&mut self) {
        let max_checks = self.session_order.len();
        let mut checked = 0;

        while self.sessions.len() > MAX_RETAINED_MOBILE_SESSIONS && checked < max_checks {
            let Some(id) = self.session_order.pop_front() else {
                break;
            };
            checked += 1;

            match self.sessions.get(&id) {
                Some(session) if session_is_finished(&session.phase) => {
                    self.sessions.remove(&id);
                }
                Some(_) => self.session_order.push_back(id),
                None => {}
            }
        }
    }

    fn close_open_sessions(&mut self) {
        for session in self.sessions.values_mut() {
            if !session_is_finished(&session.phase) {
                session.phase = MobileSessionPhase::Closed;
            }
        }
    }

    fn session_view(&self, session: &MobileSession) -> MobileSessionView {
        let remaining_seconds = session
            .expires_at
            .map(|expires_at| (expires_at - self.now()).num_seconds().max(0));
        MobileSessionView {
            id: session.id.clone(),
            url: self.session_url(session),
            mode: session.mode.clone(),
            phase: session.phase.clone(),
            expires_at: session.expires_at,
            remaining_seconds,
            summary: session
                .content_items
                .first()
                .map(|item| mobile_content_summary(&item.text))
                .unwrap_or_else(|| "???".to_string()),
            submitted_summary: session
                .submitted_items
                .last()
                .map(|item| mobile_content_summary(&item.text)),
            content_items: session.content_items.clone(),
            submitted_items: session.submitted_items.clone(),
        }
    }

    fn session_url(&self, session: &MobileSession) -> String {
        format!(
            "http://{}:{}/m/{}?token={}",
            self.host, self.port, session.id, session.token
        )
    }

    fn now(&self) -> DateTime<Utc> {
        Utc::now() + self.test_clock_offset
    }
}

impl Drop for MobileHttpServer {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
        self.join.abort();
    }
}

fn mobile_text_item(text: String, source_device: String) -> ClipboardTextItem {
    ClipboardTextItem {
        id: Uuid::new_v4().simple().to_string(),
        text,
        source_device,
    }
}

fn normalize_source_device(source_device: String, fallback: &str) -> String {
    let source_device = source_device.trim();
    if source_device.is_empty() {
        fallback.to_string()
    } else {
        source_device.to_string()
    }
}

fn session_is_finished(phase: &MobileSessionPhase) -> bool {
    matches!(phase, MobileSessionPhase::Expired | MobileSessionPhase::Closed)
}

fn reject_closed_session(session: &MobileSession) -> AppResult<()> {
    if session.phase == MobileSessionPhase::Closed {
        return Err(AppError::InvalidInput("手机连接会话已结束".to_string()));
    }
    if session.phase == MobileSessionPhase::Expired {
        return Err(AppError::InvalidInput("此二维码已过期".to_string()));
    }
    Ok(())
}

fn upsert_mobile_content_item(
    items: &mut Vec<ClipboardTextItem>,
    text: String,
    source_device: String,
) {
    let trimmed = text.trim().to_string();
    if let Some(index) = items
        .iter()
        .position(|item| item.text.trim() == trimmed)
    {
        let mut item = items.remove(index);
        item.text = text;
        item.source_device = source_device;
        items.insert(0, item);
    } else {
        items.insert(0, mobile_text_item(text, source_device));
        items.truncate(MOBILE_PC_ITEM_LIMIT);
    }
}

fn mobile_clipboard_message(content: String) -> ClipboardMessage {
    let content_type = ClipboardContentType::Text;
    let now = Utc::now();
    ClipboardMessage {
        message_id: Uuid::new_v4().to_string(),
        source_device_id: "mobile".to_string(),
        source_device_name: MOBILE_DEVICE_NAME.to_string(),
        content_hash: sync::content_hash(&content_type, &content),
        content_type,
        content,
        timestamp: now.timestamp(),
        origin_sequence: None,
        event_version: Some(crate::models::ClipboardEventVersion {
            physical_ms: now.timestamp_millis(),
            logical: 0,
            origin_device_id: "mobile".to_string(),
        }),
    }
}

#[cfg(test)]
fn mobile_submitted_history_item(content: String) -> crate::models::HistoryItem {
    let message = mobile_clipboard_message(content);
    mobile_submitted_history_item_from_message(&message)
}

fn mobile_submitted_history_item_from_message(message: &ClipboardMessage) -> HistoryItem {
    history::make_history_item_with_status(
        HistoryDirection::Remote,
        MOBILE_DEVICE_NAME,
        message,
        SyncStatus::Synced,
    )
}

async fn record_mobile_submitted_history(
    app: &AppHandle,
    state: &AppState,
    message: &ClipboardMessage,
) -> AppResult<()> {
    if !state.config().await.save_history {
        return Ok(());
    }

    let item = mobile_submitted_history_item_from_message(message);
    state.push_history(item.clone()).await;
    history::save_history(app, &state.history().await)?;
    let _ = app.emit("clipboard-synced", history::history_item_for_frontend(&item));
    Ok(())
}

pub fn mobile_content_summary(content: &str) -> String {
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return "空文本".to_string();
    }
    let mut summary = compact.chars().take(80).collect::<String>();
    if compact.chars().count() > 80 {
        summary.push_str("...");
    }
    summary
}

pub async fn create_session(
    app: AppHandle,
    state: AppState,
    contents: Vec<String>,
    source_device: String,
) -> AppResult<MobileSessionView> {
    let runtime = mobile_runtime();
    ensure_mobile_server(runtime, app, state).await?;
    runtime
        .sessions
        .lock()
        .await
        .create_session_with_source(contents, source_device)
}

pub async fn append_pc_clipboard_text(content: String, source_device: String) -> AppResult<()> {
    mobile_runtime()
        .sessions
        .lock()
        .await
        .append_pc_clipboard_text_with_source(content, source_device)
}

pub async fn get_session_view(id: &str) -> AppResult<MobileSessionView> {
    mobile_runtime().sessions.lock().await.get_session_view(id)
}

pub async fn close_session(id: &str) -> AppResult<MobileSessionView> {
    mobile_runtime().sessions.lock().await.close_session(id)
}

pub async fn take_submitted_content_for_write(id: &str) -> AppResult<String> {
    mobile_runtime()
        .sessions
        .lock()
        .await
        .take_submitted_content_for_write(id)
}

fn mobile_runtime() -> &'static MobileRuntime {
    MOBILE_RUNTIME.get_or_init(|| MobileRuntime {
        sessions: Arc::new(Mutex::new(MobileSessionStore::new(
            "127.0.0.1",
            MOBILE_HTTP_PORT,
        ))),
        server: Mutex::new(None),
    })
}

pub async fn mobile_server_running() -> bool {
    mobile_runtime().server.lock().await.is_some()
}

async fn ensure_mobile_server(
    runtime: &MobileRuntime,
    app: AppHandle,
    state: AppState,
) -> AppResult<SocketAddr> {
    let mut server = runtime.server.lock().await;
    if let Some(server) = server.as_ref() {
        return Ok(server.address);
    }

    let ip = select_mobile_bind_ip()?;
    let address = SocketAddr::new(ip, MOBILE_HTTP_PORT);
    let listener = TcpListener::bind(address).await?;
    let actual_address = listener.local_addr()?;
    runtime
        .sessions
        .lock()
        .await
        .set_address(actual_address);

    let sessions = runtime.sessions.clone();
    let app = app.clone();
    let state = state.clone();
    let (stop, mut stop_rx) = watch::channel(false);
    let join = tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = stop_rx.changed() => break,
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else {
                        continue;
                    };
                    let sessions = sessions.clone();
                    let app = app.clone();
                    let state = state.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = handle_mobile_connection(stream, sessions, app, state).await;
                    });
                }
            }
        }
    });

    *server = Some(MobileHttpServer {
        address: actual_address,
        stop,
        join,
    });
    Ok(actual_address)
}

fn select_mobile_bind_ip() -> AppResult<IpAddr> {
    match network::preferred_local_ip_for_peer(None) {
        Some(IpAddr::V4(ip)) if is_lan_ipv4(ip) => Ok(IpAddr::V4(ip)),
        _ => Err(AppError::InvalidInput(
            "没有找到可用于手机连接的局域网 IPv4 地址".to_string(),
        )),
    }
}

fn is_lan_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private()
}

async fn handle_mobile_connection(
    mut stream: TcpStream,
    sessions: Arc<Mutex<MobileSessionStore>>,
    app: AppHandle,
    state: AppState,
) -> AppResult<()> {
    let request = read_http_request(&mut stream).await?;
    let response = route_mobile_request(request, sessions, app, state).await;
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

async fn read_http_request(stream: &mut TcpStream) -> AppResult<HttpRequest> {
    let mut buffer = vec![0u8; 132 * 1024];
    let mut read_len = 0usize;
    let header_end;
    loop {
        let n = stream.read(&mut buffer[read_len..]).await?;
        if n == 0 {
            return Err(AppError::InvalidInput("HTTP 请求为空".to_string()));
        }
        read_len += n;
        if let Some(index) = find_header_end(&buffer[..read_len]) {
            header_end = index;
            break;
        }
        if read_len == buffer.len() {
            return Err(AppError::InvalidInput("HTTP 请求过大".to_string()));
        }
    }

    let header_text = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| AppError::InvalidInput("HTTP 请求行无效".to_string()))?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| AppError::InvalidInput("HTTP 方法无效".to_string()))?
        .to_string();
    let target = parts
        .next()
        .ok_or_else(|| AppError::InvalidInput("HTTP 路径无效".to_string()))?
        .to_string();
    let content_length = header_text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.eq_ignore_ascii_case("content-length") {
                value.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0);
    if content_length > 120 * 1024 {
        return Err(AppError::InvalidInput("HTTP 请求正文过大".to_string()));
    }

    let body_start = header_end + 4;
    while read_len < body_start + content_length {
        let n = stream.read(&mut buffer[read_len..]).await?;
        if n == 0 {
            break;
        }
        read_len += n;
    }
    let body = buffer[body_start..body_start + content_length].to_vec();

    Ok(HttpRequest {
        method,
        target,
        body,
    })
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

struct HttpRequest {
    method: String,
    target: String,
    body: Vec<u8>,
}

async fn route_mobile_request(
    request: HttpRequest,
    sessions: Arc<Mutex<MobileSessionStore>>,
    app: AppHandle,
    state: AppState,
) -> String {
    let parsed = match url::Url::parse(&format!("http://copyshare.local{}", request.target)) {
        Ok(url) => url,
        Err(_) => return http_error(400, "请求地址无效"),
    };
    let path = parsed.path();
    let token = parsed
        .query_pairs()
        .find(|(key, _)| key == "token")
        .map(|(_, value)| value.to_string())
        .unwrap_or_default();

    if request.method == "GET" && path.starts_with("/m/") {
        let id = path.trim_start_matches("/m/").trim_matches('/');
        return http_html(200, &mobile_page_html(id, &token));
    }

    if let Some(id) = path.strip_prefix("/api/mobile/session/") {
        let id = id.trim_matches('/');
        if token.is_empty() {
            return http_error(403, "二维码 token 无效");
        }
        if request.method == "GET" {
            let mut store = sessions.lock().await;
            let content_items = match store.load_send_content_items(id, &token) {
                Ok(items) => items,
                Err(error) => {
                    let view = store.get_session_view(id);
                    if let Ok(view) = view {
                        if view.mode == MobileSessionMode::ReceiveFromMobile {
                            Vec::new()
                        } else {
                            return http_error(400, &error.to_string());
                        }
                    } else {
                        return http_error(400, &error.to_string());
                    }
                }
            };
            let view = match store.get_session_view(id) {
                Ok(view) => view,
                Err(error) => return http_error(400, &error.to_string()),
            };
            let content = content_items.first().map(|item| item.text.clone());
            let response = MobileContentResponse {
                id: view.id,
                mode: view.mode,
                phase: view.phase,
                summary: view.summary,
                content,
                content_items,
                submitted_items: view.submitted_items,
                remaining_seconds: view.remaining_seconds,
            };
            return http_json(200, &response);
        }

        if request.method == "POST" {
            let body = match serde_json::from_slice::<MobilePostBody>(&request.body) {
                Ok(body) => body,
                Err(_) => return http_error(400, "请求正文无效"),
            };
            let action = body.action.unwrap_or_default();
            if action == "copied" {
                let mut store = sessions.lock().await;
                return match store.mark_send_copied(id, &token) {
                    Ok(view) => http_json(200, &view),
                    Err(error) => http_error(400, &error.to_string()),
                };
            }

            let content = body.content.unwrap_or_default();
            {
                let mut store = sessions.lock().await;
                if let Err(error) = store.submit_mobile_content(id, &token, content.clone()) {
                    return http_error(400, &error.to_string());
                }
            }
            let message = mobile_clipboard_message(content);
            if let Err(error) = clipboard::write_clipboard_text(&app, &message.content) {
                return http_error(500, &error.to_string());
            }
            state.apply_remote_clipboard(&message).await;
            notifications::notify_mobile_clipboard_received(
                &app,
                &state.config().await,
                &message,
            );
            if let Err(error) = record_mobile_submitted_history(&app, &state, &message).await {
                return http_error(500, &error.to_string());
            }
            let mut store = sessions.lock().await;
            return match store.mark_submitted_content_written(id) {
                Ok(view) => http_json(200, &view),
                Err(error) => http_error(400, &error.to_string()),
            };
        }
    }

    http_error(404, "页面不存在")
}

fn mobile_page_html(id: &str, token: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover" />
  <title>CopyShare 手机连接</title>
  <style>
    :root {{ color-scheme: dark; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    body {{ margin: 0; min-height: 100vh; background: #202020; color: #f5f5f5; display: grid; align-items: start; justify-items: center; padding: 18px; overflow-y: auto; -webkit-overflow-scrolling: touch; }}
    main {{ width: min(100%, 560px); max-height: calc(100vh - 36px); overflow-y: auto; border: 1px solid rgba(255,255,255,.14); border-radius: 24px; background: rgba(43,43,43,.94); box-shadow: 0 24px 80px rgba(0,0,0,.42); padding: 22px; }}
    h1 {{ margin: 0; font-size: 22px; }}
    h2 {{ margin: 18px 0 10px; font-size: 15px; color: #ffffff; }}
    p {{ color: #cfcfcf; line-height: 1.7; }}
    textarea, pre {{ width: 100%; box-sizing: border-box; border-radius: 18px; border: 1px solid rgba(255,255,255,.14); background: #262626; color: #fff; padding: 14px; font: 15px/1.6 ui-monospace, SFMono-Regular, Consolas, monospace; white-space: pre-wrap; word-break: break-word; user-select: text; }}
    textarea {{ min-height: 170px; }}
    pre {{ min-height: 86px; max-height: 210px; overflow: auto; margin: 10px 0 0; }}
    button {{ width: 100%; min-height: 48px; margin-top: 12px; border: 1px solid rgba(96,205,255,.44); border-radius: 999px; background: #3a3a3a; color: white; font-weight: 700; font-size: 15px; touch-action: manipulation; }}
    button.secondary {{ border-color: rgba(255,255,255,.16); background: #2f2f2f; color: #d6d6d6; }}
    .pc-list-scroll {{ max-height: min(48vh, 330px); overflow-y: auto; display: grid; gap: 10px; padding-right: 4px; overscroll-behavior: contain; -webkit-overflow-scrolling: touch; touch-action: pan-y; }}
    .pc-list-scroll::-webkit-scrollbar {{ width: 6px; }}
    .pc-list-scroll::-webkit-scrollbar-thumb {{ background: rgba(255,255,255,.18); border-radius: 999px; }}
    .item {{ border: 1px solid rgba(255,255,255,.12); border-radius: 20px; background: rgba(255,255,255,.045); padding: 12px; transition: border-color .16s ease, background-color .16s ease; }}
    .item.selected {{ border-color: rgba(138,164,184,.78); background: #263039; }}
    .item-choice {{ display: grid; grid-template-columns: 22px minmax(0,1fr); gap: 10px; align-items: start; }}
    .item-check {{ width: 18px; height: 18px; margin-top: 2px; accent-color: #8aa4b8; }}
    .item-title {{ display: flex; justify-content: space-between; gap: 10px; color: #f3f4f6; font-size: 13px; font-weight: 700; }}
    .item-title span:last-child {{ min-width: 0; max-width: 46%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #9ca3af; font-weight: 500; }}
    .item pre {{ margin: 10px 0 0; }}
    .actions {{ display: grid; grid-template-columns: minmax(0,1fr) minmax(0,1fr); gap: 10px; }}
    .empty {{ border: 1px dashed rgba(255,255,255,.18); border-radius: 18px; padding: 14px; color: #b9b9b9; }}
    .status {{ margin-top: 12px; color: #dff6ff; }}
  </style>
</head>
<body>
<main>
  <h1 id="title">CopyShare</h1>
  <p id="hint">正在读取二维码内容...</p>
  <section id="app"></section>
  <p class="status" id="status"></p>
</main>
<script>
const id = "{id}";
const token = "{token}";
const api = "/api/mobile/session/" + id + "?token=" + encodeURIComponent(token);
const app = document.getElementById("app");
const title = document.getElementById("title");
const hint = document.getElementById("hint");
const statusEl = document.getElementById("status");
let latestPcItems = [];
let latestPcItemsSignature = "";
let selectedPcIds = new Set();
let selectionTouched = false;
let pollTimer = null;
function setStatus(text) {{ statusEl.textContent = text; }}
async function load() {{
  const response = await fetch(api);
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || "二维码已失效");
  renderData(data);
}}
async function refreshFromPc() {{
  try {{
    await load();
  }} catch (error) {{
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = null;
    const message = error.message || "二维码已失效";
    if (message.includes("已结束")) {{
      renderClosed();
      return;
    }}
    setStatus(message);
  }}
}}
function schedulePolling(data) {{
  if (data.phase === "closed") {{
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = null;
    return;
  }}
  if (!pollTimer) pollTimer = setInterval(refreshFromPc, 1500);
}}
function renderData(data) {{
  if (data.phase === "closed") {{
    renderClosed();
    schedulePolling(data);
    return;
  }}
  if (data.mode === "sendToMobile") renderSend(data);
  else if (data.mode === "bidirectional") renderBoth(data);
  else renderReceive();
  schedulePolling(data);
}}
function pcItems(data) {{
  if (Array.isArray(data.contentItems) && data.contentItems.length) return data.contentItems;
  return data.content ? [{{ id: "current", text: data.content }}] : [];
}}
function itemKey(item, index) {{
  return item && item.id ? String(item.id) : "pc-" + index;
}}
function deviceLabel(item) {{
  const value = String((item && item.sourceDevice) || "").trim();
  return value || "CopyShare";
}}
function pcItemsSignature(items) {{
  return items.map(function(item, index) {{
    return itemKey(item, index) + "\\u001f" + deviceLabel(item) + "\\u001f" + String((item && item.text) || "");
  }}).join("\\u001e");
}}
function rememberScrollPosition() {{
  const pcList = document.querySelector(".pc-list-scroll");
  const main = document.querySelector("main");
  return {{
    pcTop: pcList ? pcList.scrollTop : 0,
    mainTop: main ? main.scrollTop : 0,
    windowTop: window.scrollY || document.documentElement.scrollTop || document.body.scrollTop || 0
  }};
}}
function restoreScrollPosition(position) {{
  function apply() {{
    const pcList = document.querySelector(".pc-list-scroll");
    if (pcList) {{
      const maxTop = Math.max(0, pcList.scrollHeight - pcList.clientHeight);
      pcList.scrollTop = Math.min(position.pcTop || 0, maxTop);
    }}
    const main = document.querySelector("main");
    if (main) main.scrollTop = position.mainTop || 0;
    window.scrollTo(0, position.windowTop || 0);
  }}
  apply();
  requestAnimationFrame(apply);
}}
function syncSelection(items) {{
  const available = new Set(items.map(itemKey));
  selectedPcIds = new Set(Array.from(selectedPcIds).filter(id => available.has(id)));
  if (!selectionTouched && items.length && selectedPcIds.size === 0) {{
    selectedPcIds.add(itemKey(items[0], 0));
  }}
}}
function renderPcList(items) {{
  if (!items.length) return '<p class="empty">电脑端没有可读取的文本剪贴板。</p>';
  syncSelection(items);
  const list = items.map(function(item, index) {{
    const key = itemKey(item, index);
    const checked = selectedPcIds.has(key);
    return '<article class="item' + (checked ? ' selected' : '') + '" data-pc-item="' + escapeHtml(key) + '"><div class="item-choice"><input class="item-check" type="checkbox" data-pc-check="' + escapeHtml(key) + '"' + (checked ? ' checked' : '') + ' /><div><div class="item-title"><span>电脑剪贴板 ' + (index + 1) + '</span><span>' + escapeHtml(deviceLabel(item)) + '</span></div><pre>' + escapeHtml(item.text || "") + '</pre></div></div></article>';
  }}).join("");
  const allSelected = selectedPcIds.size === items.length;
  return '<div class="pc-list-scroll">' + list + '</div><div class="actions"><button id="copySelectedPc">复制选中内容</button><button class="secondary" id="togglePcSelection">' + (allSelected ? '取消全选' : '全部选择') + '</button></div>';
}}
function bindPcList(items) {{
  document.querySelectorAll("[data-pc-check]").forEach(function(input) {{
    input.onchange = function() {{
      selectionTouched = true;
      const key = input.getAttribute("data-pc-check");
      if (input.checked) selectedPcIds.add(key);
      else selectedPcIds.delete(key);
      renderPcPanel({{ contentItems: items }}, true);
    }};
  }});
  const copyButton = document.getElementById("copySelectedPc");
  if (copyButton) copyButton.onclick = copySelectedPcItems;
  const toggleButton = document.getElementById("togglePcSelection");
  if (toggleButton) {{
    toggleButton.onclick = function() {{
      selectionTouched = true;
      if (selectedPcIds.size === items.length) selectedPcIds.clear();
      else selectedPcIds = new Set(items.map(itemKey));
      renderPcPanel({{ contentItems: items }}, true);
    }};
  }}
}}
function renderPcPanel(data, force) {{
  const items = pcItems(data);
  const panel = document.getElementById("pcList");
  if (!panel) return;
  const signature = pcItemsSignature(items);
  latestPcItems = items;
  if (!force && panel.dataset.pcSignature === signature) return;
  const scrollPosition = rememberScrollPosition();
  latestPcItemsSignature = signature;
  panel.dataset.pcSignature = signature;
  panel.innerHTML = renderPcList(items);
  bindPcList(items);
  restoreScrollPosition(scrollPosition);
}}
function selectedPcTexts(items) {{
  return items.filter(function(item, index) {{
    return selectedPcIds.has(itemKey(item, index));
  }}).map(function(item) {{
    return item.text || "";
  }}).filter(Boolean);
}}
function copySelectedPcItems() {{
  const selectedTexts = selectedPcTexts(latestPcItems);
  if (!selectedTexts.length) {{
    setStatus("请先选择要复制的电脑剪贴板内容。");
    return;
  }}
  navigator.clipboard.writeText(selectedTexts.join("\n\n")).then(async function() {{
    await fetch(api, {{ method: "POST", headers: {{ "Content-Type": "application/json" }}, body: JSON.stringify({{ action: "copied" }}) }});
    setStatus("已复制 " + selectedTexts.length + " 条电脑剪贴板内容。");
  }}).catch(function(error) {{
    setStatus(error.message || "复制失败");
  }});
}}
function renderPcShell() {{
  return '<h2>电脑剪贴板</h2><div id="pcList"></div>';
}}
function readPhoneClipboard(areaId) {{
  const textarea = document.getElementById(areaId);
  if (!textarea) return;
  if (!navigator.clipboard || !navigator.clipboard.readText) {{
    textarea.focus();
    setStatus("无法读取手机剪贴板，请长按输入框手动粘贴。");
    return;
  }}
  navigator.clipboard.readText().then(function(text) {{
    textarea.value = text || "";
    textarea.focus();
    setStatus(textarea.value.trim() ? "已粘贴手机剪贴板内容。" : "手机剪贴板为空。");
  }}).catch(function(error) {{
    textarea.focus();
    const detail = error && error.message ? " " + error.message : "";
    setStatus("无法读取手机剪贴板，请长按输入框手动粘贴。" + detail);
  }});
}}
function senderHtml(areaId) {{
  return '<textarea id="' + areaId + '" placeholder="在这里粘贴要发送到电脑的文本"></textarea><button id="paste">粘贴手机内容</button><button id="send">发送到电脑</button>';
}}
function bindSender(areaId) {{
  document.getElementById("paste").onclick = function() {{
    readPhoneClipboard(areaId);
  }};
  document.getElementById("send").onclick = async function() {{
    const field = document.getElementById(areaId);
    const content = field.value;
    const response = await fetch(api, {{ method: "POST", headers: {{ "Content-Type": "application/json" }}, body: JSON.stringify({{ content }}) }});
    const result = await response.json();
    if (!response.ok) throw new Error(result.error || "发送失败");
    field.value = "";
    setStatus("已发送，已自动写入电脑剪贴板。可以继续发送下一条。");
  }};
}}
function renderBoth(data) {{
  title.textContent = "CopyShare 手机连接";
  hint.textContent = "可复制电脑端多条剪贴板，也可连续发送多条内容到电脑。";
  if (app.dataset.mode !== "bidirectional") {{
    app.dataset.mode = "bidirectional";
    app.innerHTML = renderPcShell() + '<h2>发送到电脑</h2>' + senderHtml('phoneContent');
    bindSender('phoneContent');
  }}
  renderPcPanel(data);
}}
function renderSend(data) {{
  title.textContent = "来自 CopyShare";
  hint.textContent = "可复制电脑端多条剪贴板。二维码会在有效期结束后失效。";
  if (app.dataset.mode !== "sendToMobile") {{
    app.dataset.mode = "sendToMobile";
    app.innerHTML = renderPcShell();
  }}
  renderPcPanel(data);
}}
function renderReceive() {{
  title.textContent = "发送到电脑";
  hint.textContent = "发送成功后会自动写入电脑剪贴板，可连续发送多条。";
  if (app.dataset.mode !== "receiveFromMobile") {{
    app.dataset.mode = "receiveFromMobile";
    app.innerHTML = senderHtml('content');
    bindSender('content');
  }}
}}
function renderClosed() {{
  title.textContent = "CopyShare 手机连接";
  hint.textContent = "电脑端已结束本次连接会话";
  app.dataset.mode = "closed";
  app.innerHTML = '<p class="empty">电脑端已结束本次连接会话。请在电脑端重新生成二维码后再扫码。</p>';
  setStatus("");
}}
function preview(value) {{
  const text = String(value || "").replace(/\s+/g, " ").trim();
  return text.length > 22 ? text.slice(0, 22) + "..." : text;
}}
function escapeHtml(value) {{
  return String(value).replace(/[&<>"']/g, ch => ({{"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#39;"}}[ch]));
}}
load().catch(error => {{
  if (pollTimer) clearInterval(pollTimer);
  const message = error.message || "二维码已失效";
  hint.textContent = message.includes("已结束") ? "电脑端已结束本次连接会话" : message;
  app.innerHTML = "";
}});
</script>
</body>
</html>"#,
        id = escape_js(id),
        token = escape_js(token),
    )
}
fn http_json<T: Serialize>(status: u16, value: &T) -> String {
    let body = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    http_response(status, "application/json; charset=utf-8", &body)
}

fn http_html(status: u16, body: &str) -> String {
    http_response(status, "text/html; charset=utf-8", body)
}

fn http_error(status: u16, message: &str) -> String {
    http_json(status, &json!({ "error": message }))
}

fn http_response(status: u16, content_type: &str, body: &str) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.as_bytes().len()
    )
}

fn ensure_mobile_text_items_size(contents: &[String]) -> AppResult<()> {
    for content in contents {
        ensure_mobile_text_size(content)?;
    }
    Ok(())
}

fn ensure_mobile_text_size(content: &str) -> AppResult<()> {
    if content.as_bytes().len() > MOBILE_TEXT_LIMIT_BYTES {
        return Err(AppError::InvalidInput(
            "手机连接文本不能超过 100KB".to_string(),
        ));
    }
    Ok(())
}

fn escape_js(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        mobile::{
            mobile_content_summary, MobileSessionStore, MOBILE_SESSION_TTL, MOBILE_TEXT_LIMIT_BYTES,
        },
        models::{MobileSessionMode, MobileSessionPhase},
    };

    #[test]
    fn summary_keeps_logs_short_without_storing_full_content() {
        let summary = mobile_content_summary("  第一行\n第二行\n第三行  ");

        assert_eq!(summary, "第一行 第二行 第三行");
    }

    #[test]
    fn send_session_url_contains_id_and_token_and_stays_open_until_manual_close() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);

        let session = store
            .create_send_session("hello from pc".to_string())
            .expect("send session should be created");

        assert_eq!(session.mode, MobileSessionMode::SendToMobile);
        assert_eq!(session.phase, MobileSessionPhase::Waiting);
        assert!(session.url.starts_with("http://10.194.34.119:8766/m/"));
        assert!(session.url.contains("?token="));
        assert_eq!(session.expires_at, None);
        assert_eq!(session.remaining_seconds, None);
        assert_eq!(session.summary, "hello from pc");
    }

    #[test]
    fn send_session_token_remains_usable_after_mobile_copy_until_expiry() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let session = store.create_send_session("secret".to_string()).unwrap();
        let token = store.test_token(&session.id).unwrap();

        let content = store
            .load_send_content(&session.id, &token)
            .expect("mobile should read content");
        assert_eq!(content, "secret");

        let copied = store
            .mark_send_copied(&session.id, &token)
            .expect("copy acknowledgement should work");
        assert_eq!(copied.phase, MobileSessionPhase::Copied);
        assert_eq!(store.load_send_content(&session.id, &token).unwrap(), "secret");
        assert!(store.mark_send_copied(&session.id, &token).is_ok());
    }

    #[test]
    fn receive_session_allows_multiple_phone_submits_in_one_qr() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let session = store.create_receive_session().unwrap();
        let token = store.test_token(&session.id).unwrap();

        store
            .submit_mobile_content(&session.id, &token, "from phone one".to_string())
            .expect("first phone submit should work");
        let first_written = store
            .mark_submitted_content_written(&session.id)
            .expect("first auto clipboard write should mark session written");
        assert_eq!(first_written.phase, MobileSessionPhase::Written);
        assert_eq!(first_written.submitted_summary.as_deref(), Some("from phone one"));
        assert_eq!(first_written.submitted_items.len(), 1);

        let second = store
            .submit_mobile_content(&session.id, &token, "from phone two".to_string())
            .expect("second phone submit should work in the same QR session");
        assert_eq!(second.phase, MobileSessionPhase::Submitted);
        assert_eq!(second.submitted_items.len(), 2);

        let second_written = store.mark_submitted_content_written(&session.id).unwrap();
        assert_eq!(second_written.phase, MobileSessionPhase::Written);
        assert_eq!(second_written.submitted_summary.as_deref(), Some("from phone two"));
        assert_eq!(second_written.submitted_items[0].text, "from phone one");
        assert_eq!(second_written.submitted_items[1].text, "from phone two");
    }

    #[test]
    fn bidirectional_session_exposes_multiple_pc_items_and_multiple_phone_submits() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let session = store
            .create_session(vec!["from pc one".to_string(), "from pc two".to_string()])
            .unwrap();
        let token = store.test_token(&session.id).unwrap();

        assert_eq!(session.mode, MobileSessionMode::Bidirectional);
        assert_eq!(session.content_items.len(), 2);
        assert_eq!(session.content_items[0].text, "from pc one");
        assert_eq!(session.content_items[1].text, "from pc two");
        assert_eq!(store.load_send_content_items(&session.id, &token).unwrap().len(), 2);
        assert_eq!(
            store.mark_send_copied(&session.id, &token).unwrap().phase,
            MobileSessionPhase::Copied
        );

        store
            .submit_mobile_content(&session.id, &token, "from phone one".to_string())
            .unwrap();
        store.mark_submitted_content_written(&session.id).unwrap();
        store
            .submit_mobile_content(&session.id, &token, "from phone two".to_string())
            .unwrap();
        let written = store.mark_submitted_content_written(&session.id).unwrap();
        assert_eq!(written.phase, MobileSessionPhase::Written);
        assert_eq!(written.submitted_items.len(), 2);
        assert_eq!(written.submitted_items[1].text, "from phone two");
    }

    #[test]
    fn bidirectional_session_tags_pc_and_phone_items_with_device_names() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let session = store
            .create_session_with_source(
                vec!["from pc one".to_string()],
                "Surface Pro".to_string(),
            )
            .unwrap();
        let token = store.test_token(&session.id).unwrap();

        assert_eq!(session.content_items[0].source_device, "Surface Pro");

        store
            .append_pc_clipboard_text_with_source(
                "from pc two".to_string(),
                "Surface Pro".to_string(),
            )
            .unwrap();
        store
            .submit_mobile_content(&session.id, &token, "from phone".to_string())
            .unwrap();
        store.mark_submitted_content_written(&session.id).unwrap();

        let view = store.get_session_view(&session.id).unwrap();
        assert_eq!(view.content_items[0].text, "from phone");
        assert_eq!(view.content_items[0].source_device, super::MOBILE_DEVICE_NAME);
        assert_eq!(view.content_items[1].source_device, "Surface Pro");
        assert_eq!(view.submitted_items[0].source_device, super::MOBILE_DEVICE_NAME);
    }

    #[test]
    fn active_bidirectional_session_stays_open_after_old_ttl_and_receives_later_pc_clipboard_items_without_duplicates() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let session = store
            .create_session(vec!["from pc one".to_string()])
            .unwrap();

        store.append_pc_clipboard_text("from pc two".to_string()).unwrap();
        store.append_pc_clipboard_text("from pc one".to_string()).unwrap();

        let view = store.get_session_view(&session.id).unwrap();
        assert_eq!(view.content_items.len(), 2);
        assert_eq!(view.content_items[0].text, "from pc one");
        assert_eq!(view.content_items[1].text, "from pc two");

        store.advance_test_clock(MOBILE_SESSION_TTL + Duration::from_secs(1));
        store.append_pc_clipboard_text("after expiry".to_string()).unwrap();

        let still_open = store.get_session_view(&session.id).unwrap();
        assert_ne!(still_open.phase, MobileSessionPhase::Expired);
        assert_eq!(still_open.content_items.len(), 3);
        assert_eq!(still_open.content_items[0].text, "after expiry");
        assert_eq!(
            store.load_send_content_items(&session.id, &store.test_token(&session.id).unwrap())
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn manual_close_rejects_mobile_reads_and_submits() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let session = store
            .create_session(vec!["from pc".to_string()])
            .expect("session should be created");
        let token = store.test_token(&session.id).unwrap();

        let closed = store.close_session(&session.id).expect("session should close");

        assert_eq!(closed.phase, MobileSessionPhase::Closed);
        assert!(store.load_send_content_items(&session.id, &token).is_err());
        assert!(store
            .submit_mobile_content(&session.id, &token, "from phone".to_string())
            .is_err());
    }

    #[test]
    fn creating_new_session_closes_previous_session() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let first = store.create_session(vec!["first".to_string()]).unwrap();
        let first_token = store.test_token(&first.id).unwrap();
        let second = store.create_session(vec!["second".to_string()]).unwrap();

        assert_eq!(
            store.get_session_view(&first.id).unwrap().phase,
            MobileSessionPhase::Closed
        );
        assert!(store.load_send_content_items(&first.id, &first_token).is_err());
        assert_eq!(store.get_session_view(&second.id).unwrap().phase, MobileSessionPhase::Waiting);
    }

    #[test]
    fn repeated_mobile_sessions_do_not_accumulate_unbounded_closed_sessions() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let mut latest_id = String::new();

        for index in 0..40 {
            latest_id = store
                .create_session(vec![format!("session {index}")])
                .unwrap()
                .id;
        }

        assert!(
            store.sessions.len() <= 10,
            "expected closed mobile sessions to be pruned, got {}",
            store.sessions.len()
        );
        assert_eq!(
            store.get_session_view(&latest_id).unwrap().phase,
            MobileSessionPhase::Waiting
        );
    }

    #[test]
    fn mobile_polling_pc_items_does_not_clear_submitted_or_written_phase() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);
        let session = store
            .create_session(vec!["from pc one".to_string()])
            .unwrap();
        let token = store.test_token(&session.id).unwrap();

        store
            .submit_mobile_content(&session.id, &token, "from phone".to_string())
            .unwrap();
        store.mark_submitted_content_written(&session.id).unwrap();
        store.load_send_content_items(&session.id, &token).unwrap();

        assert_eq!(
            store.get_session_view(&session.id).unwrap().phase,
            MobileSessionPhase::Written
        );
    }

    #[test]
    fn mobile_phone_page_uses_scrollable_multi_select_list_and_polling() {
        let html = super::mobile_page_html("session-id", "token");

        assert!(html.contains("pc-list-scroll"));
        assert!(html.contains("data-pc-check"));
        assert!(html.contains("selectedPcIds"));
        assert!(html.contains("copySelectedPcItems"));
        assert!(html.contains("setInterval"));
        assert!(html.contains("data.phase === \"closed\""));
        assert!(html.contains("电脑端已结束本次连接会话"));
        assert!(!html.contains("data.remainingSeconds <= 0"));
        assert!(html.contains("join(\"\\n\\n\")"));
        assert!(html.contains("deviceLabel(item)"));
        assert!(html.contains("sourceDevice"));
        assert!(html.contains("-webkit-overflow-scrolling: touch"));
        assert!(html.contains("touch-action: pan-y"));
        assert!(html.contains("rememberScrollPosition"));
        assert!(html.contains("restoreScrollPosition"));
        assert!(html.contains("pcTop"));
        assert!(!html.contains("<label class=\"item-choice\""));
        assert!(!html.contains("data-copy-pc"));
        assert!(!html.contains("data-select-pc"));
    }

    #[test]
    fn mobile_phone_page_reports_clipboard_read_failures() {
        let html = super::mobile_page_html("session-id", "token");

        assert!(html.contains("readPhoneClipboard"));
        assert!(html.contains("navigator.clipboard.readText"));
        assert!(html.contains("textarea.focus()"));
        assert!(html.contains("setStatus(\"无法读取手机剪贴板"));
        assert!(html.contains("catch(function(error)"));
    }

    #[test]
    fn mobile_submitted_history_item_uses_mobile_device_name() {
        let item = super::mobile_submitted_history_item("from phone".to_string());

        assert_eq!(item.direction, crate::models::HistoryDirection::Remote);
        assert_eq!(item.source_device, super::MOBILE_DEVICE_NAME);
        assert_eq!(item.content_type, crate::models::ClipboardContentType::Text);
        assert_eq!(item.content, "from phone");
        assert_eq!(item.sync_status, crate::models::SyncStatus::Synced);
    }

    #[test]
    fn mobile_clipboard_messages_include_precise_event_ordering() {
        let message = super::mobile_clipboard_message("from phone".to_string());
        let version = message
            .event_version
            .expect("mobile clipboard should include an event version");

        assert_eq!(version.origin_device_id, "mobile");
        assert!(version.physical_ms >= message.timestamp.saturating_mul(1000));
    }

    #[test]
    fn sessions_reject_over_limit_text_and_do_not_expire_by_time() {
        let mut store = MobileSessionStore::new_for_tests("10.194.34.119", 8766);

        let too_large = "a".repeat(MOBILE_TEXT_LIMIT_BYTES + 1);
        assert!(store.create_send_session(too_large).is_err());

        let session = store.create_receive_session().unwrap();
        store.advance_test_clock(MOBILE_SESSION_TTL + Duration::from_secs(1));

        assert_eq!(
            store.get_session_view(&session.id).unwrap().phase,
            MobileSessionPhase::Waiting
        );
    }
}












