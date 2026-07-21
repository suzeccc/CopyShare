use std::{
    collections::{HashMap, HashSet},
    future::Future,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{
    fs::{self, File, OpenOptions},
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom},
    net::TcpStream,
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    clipboard, file_transfer_http, file_transfer_store, history,
    error::{AppError, AppResult},
    models::{
        AppConfig, ClipboardContentType, ClipboardMessage, CopyHistoryResult, DeviceInfo,
        FileCompleteFile, FileOfferFile, FileResumeGrantFile, FileResumeOffset,
        FileTransferDirection, FileTransferFile, FileTransferFileStatus, FileTransferProgressEvent,
        FileTransferStatus, FileTransferTask, HistoryDirection, HistoryItem, SelectedTransferFile,
        WireMessage, FILE_RESUME_CAPABILITY,
    },
    network,
    notifications,
    state::AppState,
};

const MAX_SINGLE_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_TRANSFER_FILE_COUNT: usize = 100;
const MAX_TRANSFER_TOTAL_SIZE: u64 = 5 * 1024 * 1024 * 1024;
const MAX_RETAINED_TRANSFER_TASKS: usize = 100;
const TRANSFER_CHUNK_SIZE: usize = 64 * 1024;
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(3),
    Duration::from_secs(10),
];
const TRANSFER_EXPIRY_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const PERSIST_CHECKPOINT_BYTES: u64 = 8 * 1024 * 1024;
const PERSIST_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(1);
const PROGRESS_EVENT_INTERVAL: Duration = Duration::from_millis(200);

static FILE_TRANSFER_MANAGER: OnceLock<Arc<FileTransferManager>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct IncomingFileOffer {
    pub transfer_id: String,
    pub sender_device_id: String,
    pub sender_device_name: String,
    pub clipboard_sync: bool,
    pub files: Vec<FileOfferFile>,
    pub total_size: u64,
    pub file_count: usize,
    pub download_host: String,
    pub download_port: u16,
}

#[derive(Debug, Clone)]
struct ManagedTransfer {
    task: FileTransferTask,
    files: HashMap<String, ManagedTransferFile>,
    download_host: Option<String>,
    download_port: Option<u16>,
    retry_count: u8,
    last_activity_at: DateTime<Utc>,
    last_persisted_bytes: u64,
    last_persisted_at: Instant,
    last_emitted_at: Option<Instant>,
}

#[derive(Debug, Clone)]
struct ManagedTransferFile {
    source_path: Option<PathBuf>,
    source_size: Option<u64>,
    source_modified_ms: Option<i64>,
    token: Option<String>,
    token_consumed: bool,
    token_offset: u64,
    token_expires_at: Option<DateTime<Utc>>,
    token_receiver_device_id: Option<String>,
    temp_path: Option<PathBuf>,
    final_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
struct FileDownloadServer {
    advertised_host: String,
    port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DownloadClaimError {
    NotFound,
    Forbidden,
    Canceled,
}

#[derive(Debug, Clone)]
struct DownloadClaim {
    transfer_id: String,
    file_id: String,
    path: PathBuf,
    size: u64,
    sha256: String,
    offset: u64,
}

#[derive(Debug, Clone)]
struct ReceiveFilePlan {
    file: FileTransferFile,
    token: Option<String>,
    token_offset: u64,
    receiver_device_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReceiveRunOutcome {
    Completed,
    NeedsGrant,
}

pub struct FileTransferManager {
    tasks: Mutex<HashMap<String, ManagedTransfer>>,
    store_root: Mutex<Option<PathBuf>>,
    active_receive_workers: Mutex<HashSet<String>>,
}

pub fn manager() -> Arc<FileTransferManager> {
    FILE_TRANSFER_MANAGER
        .get_or_init(FileTransferManager::new)
        .clone()
}

impl FileTransferManager {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tasks: Mutex::new(HashMap::new()),
            store_root: Mutex::new(None),
            active_receive_workers: Mutex::new(HashSet::new()),
        })
    }

    async fn set_store_root(&self, root: PathBuf) {
        *self.store_root.lock().await = Some(root);
    }

    async fn persist_transfer(&self, transfer_id: &str) -> AppResult<()> {
        let root = self.store_root.lock().await.clone();
        let Some(root) = root else {
            return Ok(());
        };
        let snapshot = self
            .tasks
            .lock()
            .await
            .get(transfer_id)
            .map(transfer_snapshot)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        file_transfer_store::save(&root, &snapshot)
    }

    async fn remove_persisted_transfer(
        &self,
        direction: &FileTransferDirection,
        transfer_id: &str,
    ) -> AppResult<()> {
        let root = self.store_root.lock().await.clone();
        if let Some(root) = root {
            file_transfer_store::remove(&root, direction, transfer_id)?;
        }
        Ok(())
    }

    async fn restore_from_store(&self) -> AppResult<Vec<FileTransferTask>> {
        let root = self.store_root.lock().await.clone();
        let Some(root) = root else {
            return Ok(Vec::new());
        };
        let expired = file_transfer_store::prune_expired(&root, Utc::now())?;
        let mut expired_tasks = Vec::with_capacity(expired.len());
        for snapshot in expired {
            for file in &snapshot.files {
                if let Some(path) = file.temp_path.as_deref() {
                    let _ = std::fs::remove_file(path);
                }
            }
            let mut task = snapshot.task;
            task.status = FileTransferStatus::Failed;
            task.error = Some("file transfer task expired".to_string());
            task.completed_at = Some(Utc::now());
            expired_tasks.push(task);
        }
        let mut restored = HashMap::new();
        for mut snapshot in file_transfer_store::load_all(&root)? {
            match file_transfer_store::reconcile_receiver_progress(&mut snapshot) {
                Ok(()) => {
                    if matches!(
                        snapshot.task.status,
                        FileTransferStatus::Accepted
                            | FileTransferStatus::Transferring
                            | FileTransferStatus::Retrying
                    ) {
                        snapshot.task.status = FileTransferStatus::WaitingForPeer;
                    }
                }
                Err(error) => {
                    snapshot.task.status = FileTransferStatus::Paused;
                    snapshot.task.error = Some(error.to_string());
                    snapshot.task.completed_at = None;
                }
            }
            let transfer_id = snapshot.task.transfer_id.clone();
            restored.insert(transfer_id, managed_transfer_from_snapshot(snapshot));
        }
        self.tasks.lock().await.extend(restored);
        Ok(expired_tasks)
    }

    async fn prune_expired_transfers(&self) -> AppResult<Vec<FileTransferTask>> {
        let root = self.store_root.lock().await.clone();
        let Some(root) = root else {
            return Ok(Vec::new());
        };
        let expired = file_transfer_store::prune_expired(&root, Utc::now())?;
        let mut expired_tasks = Vec::with_capacity(expired.len());
        let mut tasks = self.tasks.lock().await;
        for snapshot in expired {
            for file in &snapshot.files {
                if let Some(path) = file.temp_path.as_deref() {
                    let _ = std::fs::remove_file(path);
                }
            }
            tasks.remove(&snapshot.task.transfer_id);
            let mut task = snapshot.task;
            task.status = FileTransferStatus::Failed;
            task.error = Some("file transfer task expired".to_string());
            task.completed_at = Some(Utc::now());
            expired_tasks.push(task);
        }
        Ok(expired_tasks)
    }

    pub async fn tasks(&self) -> Vec<FileTransferTask> {
        let mut tasks = self
            .tasks
            .lock()
            .await
            .values()
            .map(|entry| entry.task.clone())
            .collect::<Vec<_>>();
        tasks.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        tasks
    }

    pub async fn task(&self, transfer_id: &str) -> Option<FileTransferTask> {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| entry.task.clone())
    }

    pub async fn selected_file_from_path(&self, path: PathBuf) -> AppResult<SelectedTransferFile> {
        selected_file_from_path(path).await
    }

    pub async fn selected_files_from_paths(
        &self,
        paths: Vec<PathBuf>,
    ) -> AppResult<Vec<SelectedTransferFile>> {
        selected_files_from_paths(paths).await
    }

    pub async fn create_send_offer(
        self: &Arc<Self>,
        app: &AppHandle,
        state: &AppState,
        target_device_id: String,
        file_path: PathBuf,
    ) -> AppResult<FileTransferTask> {
        self.create_send_offer_files(app, state, target_device_id, vec![file_path])
            .await
    }

    pub async fn create_send_offer_files(
        self: &Arc<Self>,
        app: &AppHandle,
        state: &AppState,
        target_device_id: String,
        file_paths: Vec<PathBuf>,
    ) -> AppResult<FileTransferTask> {
        self.create_send_offer_files_with_mode(app, state, target_device_id, file_paths, false)
            .await
    }

    pub async fn create_clipboard_send_offer_files(
        self: &Arc<Self>,
        app: &AppHandle,
        state: &AppState,
        target_device_id: String,
        file_paths: Vec<PathBuf>,
    ) -> AppResult<FileTransferTask> {
        self.create_send_offer_files_with_mode(app, state, target_device_id, file_paths, true)
            .await
    }

    async fn create_send_offer_files_with_mode(
        self: &Arc<Self>,
        app: &AppHandle,
        state: &AppState,
        target_device_id: String,
        file_paths: Vec<PathBuf>,
        clipboard_sync: bool,
    ) -> AppResult<FileTransferTask> {
        let devices = state.devices().await;
        let peer = trusted_transfer_peer(&devices, &target_device_id)?;
        let config = state.config().await;
        let selected_files = selected_files_from_paths_with_limit(
            file_paths,
            Some((config.max_send_file_size_mib, "send")),
        )
        .await?;
        let sizes = selected_files
            .iter()
            .map(|file| file.size)
            .collect::<Vec<_>>();
        validate_transfer_limits(selected_files.len(), &sizes)?;

        let status = state.status().await;
        let peer_hint = peer.ip.parse::<IpAddr>().ok();
        let server =
            advertised_download_endpoint(status.local_ip.clone(), status.port, peer_hint)?;
        let transfer_id = Uuid::new_v4().to_string();
        let total_size = sizes.iter().sum::<u64>();
        let mut task_files = Vec::with_capacity(selected_files.len());
        let mut managed_files = HashMap::new();
        let mut offer_files = Vec::with_capacity(selected_files.len());

        for selected in selected_files {
            let file_id = Uuid::new_v4().to_string();
            let token = Uuid::new_v4().simple().to_string();
            let source_metadata = fs::metadata(&selected.path).await?;
            let task_file = new_transfer_file(
                file_id.clone(),
                selected.name.clone(),
                selected.size,
                selected.sha256.clone(),
                selected.thumbnail.clone(),
            );
            managed_files.insert(
                file_id.clone(),
                ManagedTransferFile {
                    source_path: Some(PathBuf::from(&selected.path)),
                    source_size: Some(selected.size),
                    source_modified_ms: source_modified_ms(&source_metadata),
                    token: Some(token.clone()),
                    token_consumed: false,
                    token_offset: 0,
                    token_expires_at: None,
                    token_receiver_device_id: None,
                    temp_path: None,
                    final_path: None,
                },
            );
            offer_files.push(FileOfferFile {
                file_id,
                file_name: selected.name,
                file_size: selected.size,
                sha256: selected.sha256,
                thumbnail: selected.thumbnail,
                token,
            });
            task_files.push(task_file);
        }

        let task = FileTransferTask {
            transfer_id: transfer_id.clone(),
            direction: FileTransferDirection::Send,
            peer_device_id: peer.id.clone(),
            peer_device_name: peer.name.clone(),
            clipboard_sync,
            files: task_files,
            total_size,
            transferred_bytes: 0,
            status: FileTransferStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        };

        self.tasks.lock().await.insert(
            transfer_id.clone(),
            ManagedTransfer {
                task: task.clone(),
                files: managed_files,
                download_host: Some(server.advertised_host.clone()),
                download_port: Some(server.port),
                retry_count: 0,
                last_activity_at: Utc::now(),
                last_persisted_bytes: 0,
                last_persisted_at: Instant::now(),
                last_emitted_at: None,
            },
        );
        self.persist_transfer(&transfer_id).await?;
        emit_task_updated(app, &task);

        let sent = state
            .send_trusted_to_device(
                &state.config().await,
                &peer.id,
                WireMessage::FileOffer {
                    transfer_id: transfer_id.clone(),
                    sender_device_id: status.device_id,
                    sender_device_name: status.device_name,
                    clipboard_sync,
                    files: offer_files,
                    total_size,
                    file_count: task.files.len(),
                    download_host: server.advertised_host,
                    download_port: server.port,
                },
            )
            .await;

        if !sent {
            let failed = self
                .mark_failed(&transfer_id, "target device is not connected or trusted")
                .await?;
            emit_task_failed(app, &failed);
            return Err(AppError::InvalidInput(
                "target device is not connected or mutually trusted".to_string(),
            ));
        }

        Ok(task)
    }

    pub async fn serve_download_connection(
        self: Arc<Self>,
        app: AppHandle,
        stream: TcpStream,
    ) -> AppResult<()> {
        self.serve_download_connection_inner(Some(&app), stream)
            .await
    }

    async fn serve_download_connection_inner(
        self: Arc<Self>,
        app: Option<&AppHandle>,
        mut stream: TcpStream,
    ) -> AppResult<()> {
        let request = match file_transfer_http::read_head(&mut stream).await {
            Ok(request) => request,
            Err(_) => {
                write_http_response(&mut stream, 400, "Bad Request", b"bad request").await?;
                return Ok(());
            }
        };
        let Some(target) = request_target(&request.first_line) else {
            write_http_response(&mut stream, 400, "Bad Request", b"bad request").await?;
            return Ok(());
        };
        let Some((transfer_id, file_id, token, receiver_device_id)) =
            parse_download_query(&target)
        else {
            write_http_response(&mut stream, 403, "Forbidden", b"forbidden").await?;
            return Ok(());
        };
        let requested_range = match file_transfer_http::parse_open_range(request.header("range")) {
            Ok(range) => range,
            Err(_) => {
                write_http_response(&mut stream, 400, "Bad Request", b"bad range").await?;
                return Ok(());
            }
        };
        let offset = requested_range.unwrap_or(0);

        let claim = match self
            .claim_download(
                &transfer_id,
                &file_id,
                &token,
                offset,
                receiver_device_id.as_deref(),
            )
            .await
        {
            Ok(claim) => claim,
            Err(DownloadClaimError::Canceled) => {
                write_http_response(&mut stream, 409, "Conflict", b"canceled").await?;
                return Ok(());
            }
            Err(DownloadClaimError::Forbidden | DownloadClaimError::NotFound) => {
                write_http_response(&mut stream, 403, "Forbidden", b"forbidden").await?;
                return Ok(());
            }
        };
        self.persist_transfer(&transfer_id).await?;

        if claim.offset >= claim.size && claim.size > 0 {
            stream
                .write_all(file_transfer_http::range_not_satisfiable_header(claim.size).as_bytes())
                .await?;
            return Ok(());
        }
        let mut file = File::open(&claim.path).await?;
        if claim.offset > 0 {
            file.seek(SeekFrom::Start(claim.offset)).await?;
        }
        let header = if requested_range.is_some() && claim.size > 0 {
            file_transfer_http::partial_content_header(claim.size, claim.offset, &claim.sha256)
                .map_err(|error| AppError::InvalidInput(error.to_string()))?
        } else {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nETag: \"sha256:{}\"\r\nConnection: close\r\n\r\n",
                claim.size, claim.sha256
            )
        };
        stream.write_all(header.as_bytes()).await?;

        let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];
        let mut transferred = claim.offset;
        loop {
            if self.is_canceled(&claim.transfer_id).await {
                return Ok(());
            }
            let read = file.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            stream.write_all(&buffer[..read]).await?;
            transferred += read as u64;
            self.update_progress_with_optional_events(
                app,
                &claim.transfer_id,
                &claim.file_id,
                transferred,
                false,
            )
            .await?;
        }
        self.update_progress_with_optional_events(
            app,
            &claim.transfer_id,
            &claim.file_id,
            transferred,
            true,
        )
        .await?;
        Ok(())
    }

    async fn claim_download(
        &self,
        transfer_id: &str,
        file_id: &str,
        token: &str,
        offset: u64,
        receiver_device_id: Option<&str>,
    ) -> Result<DownloadClaim, DownloadClaimError> {
        let mut tasks = self.tasks.lock().await;
        let entry = tasks
            .get_mut(transfer_id)
            .ok_or(DownloadClaimError::NotFound)?;
        if entry.task.status == FileTransferStatus::Canceled {
            return Err(DownloadClaimError::Canceled);
        }
        if entry.task.direction != FileTransferDirection::Send {
            return Err(DownloadClaimError::Forbidden);
        }

        let (size, sha256) = entry
            .task
            .files
            .iter()
            .find(|file| file.id == file_id)
            .map(|file| (file.size, file.sha256.clone()))
            .ok_or(DownloadClaimError::NotFound)?;
        let managed_file = entry
            .files
            .get_mut(file_id)
            .ok_or(DownloadClaimError::NotFound)?;
        if managed_file.token.as_deref() != Some(token)
            || managed_file.token_consumed
            || managed_file.token_offset != offset
            || managed_file
                .token_expires_at
                .is_some_and(|expires_at| expires_at <= Utc::now())
            || managed_file.token_receiver_device_id.as_deref() != receiver_device_id
        {
            return Err(DownloadClaimError::Forbidden);
        }
        let path = managed_file
            .source_path
            .clone()
            .ok_or(DownloadClaimError::NotFound)?;
        let metadata = std::fs::metadata(&path).map_err(|_| DownloadClaimError::NotFound)?;
        if !metadata.is_file()
            || metadata.len() != managed_file.source_size.unwrap_or(size)
            || managed_file.source_modified_ms.is_some_and(|expected| {
                source_modified_ms(&metadata).is_some_and(|actual| actual != expected)
            })
        {
            return Err(DownloadClaimError::Forbidden);
        }
        managed_file.token_consumed = true;
        entry.task.status = FileTransferStatus::Transferring;
        if let Some(file) = entry.task.files.iter_mut().find(|file| file.id == file_id) {
            file.status = FileTransferFileStatus::Transferring;
        }
        Ok(DownloadClaim {
            transfer_id: transfer_id.to_string(),
            file_id: file_id.to_string(),
            path,
            size,
            sha256,
            offset,
        })
    }

    async fn issue_resume_token(
        &self,
        transfer_id: &str,
        file_id: &str,
        receiver_device_id: &str,
        offset: u64,
        expires_at: DateTime<Utc>,
    ) -> AppResult<String> {
        let mut tasks = self.tasks.lock().await;
        let entry = tasks
            .get_mut(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        if entry.task.direction != FileTransferDirection::Send
            || entry.task.peer_device_id != receiver_device_id
        {
            return Err(AppError::InvalidInput(
                "resume receiver does not match transfer peer".to_string(),
            ));
        }
        let size = entry
            .task
            .files
            .iter()
            .find(|file| file.id == file_id)
            .map(|file| file.size)
            .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
        if offset > size {
            return Err(AppError::InvalidInput(
                "resume offset exceeds file size".to_string(),
            ));
        }
        let managed = entry
            .files
            .get_mut(file_id)
            .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
        let token = Uuid::new_v4().simple().to_string();
        managed.token = Some(token.clone());
        managed.token_consumed = false;
        managed.token_offset = offset;
        managed.token_expires_at = Some(expires_at);
        managed.token_receiver_device_id = Some(receiver_device_id.to_string());
        Ok(token)
    }

    async fn create_resume_grants(
        &self,
        transfer_id: &str,
        receiver_device_id: &str,
        requested_files: Vec<FileResumeOffset>,
    ) -> AppResult<Vec<FileResumeGrantFile>> {
        let mut validated = Vec::with_capacity(requested_files.len());
        {
            let tasks = self.tasks.lock().await;
            let entry = tasks
                .get(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            if entry.task.direction != FileTransferDirection::Send
                || entry.task.peer_device_id != receiver_device_id
            {
                return Err(AppError::InvalidInput(
                    "resume receiver does not match transfer peer".to_string(),
                ));
            }
            for requested in requested_files {
                let task_file = entry
                    .task
                    .files
                    .iter()
                    .find(|file| file.id == requested.file_id)
                    .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
                if requested.offset > task_file.size {
                    return Err(AppError::InvalidInput(
                        "resume offset exceeds file size".to_string(),
                    ));
                }
                let managed = entry
                    .files
                    .get(&requested.file_id)
                    .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
                let path = managed
                    .source_path
                    .as_ref()
                    .ok_or_else(|| AppError::InvalidInput("source file is missing".to_string()))?;
                let metadata = std::fs::metadata(path)?;
                if !metadata.is_file()
                    || metadata.len() != managed.source_size.unwrap_or(task_file.size)
                    || managed.source_modified_ms.is_some_and(|expected| {
                        source_modified_ms(&metadata).is_some_and(|actual| actual != expected)
                    })
                {
                    return Err(AppError::InvalidInput("source file has changed".to_string()));
                }
                validated.push(requested);
            }
        }

        let expires_at = Utc::now() + chrono::Duration::seconds(60);
        let mut grants = Vec::with_capacity(validated.len());
        for requested in validated {
            let token = self
                .issue_resume_token(
                    transfer_id,
                    &requested.file_id,
                    receiver_device_id,
                    requested.offset,
                    expires_at,
                )
                .await?;
            grants.push(FileResumeGrantFile {
                file_id: requested.file_id,
                offset: requested.offset,
                token,
            });
        }
        Ok(grants)
    }

    async fn handle_resume_request(
        &self,
        state: &AppState,
        transfer_id: &str,
        receiver_device_id: &str,
        requested_files: Vec<FileResumeOffset>,
    ) -> AppResult<()> {
        if !state
            .peer_supports(receiver_device_id, FILE_RESUME_CAPABILITY)
            .await
        {
            return Err(AppError::InvalidInput(
                "peer does not support resumable file transfers".to_string(),
            ));
        }
        let peer = trusted_transfer_peer(&state.devices().await, receiver_device_id)?;
        let status = state.status().await;
        let endpoint = advertised_download_endpoint(
            status.local_ip.clone(),
            status.port,
            peer.ip.parse::<IpAddr>().ok(),
        )?;
        let (download_host, download_port) = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            if entry.task.direction != FileTransferDirection::Send
                || entry.task.peer_device_id != receiver_device_id
            {
                return Err(AppError::InvalidInput(
                    "resume receiver does not match transfer peer".to_string(),
                ));
            }
            entry.download_host = Some(endpoint.advertised_host.clone());
            entry.download_port = Some(endpoint.port);
            (endpoint.advertised_host, endpoint.port)
        };
        let files = self
            .create_resume_grants(transfer_id, receiver_device_id, requested_files)
            .await?;
        {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry.last_activity_at = Utc::now();
        }
        self.persist_transfer(transfer_id).await?;

        let sent = state
            .send_trusted_to_device(
                &state.config().await,
                receiver_device_id,
                WireMessage::FileResumeGrant {
                    transfer_id: transfer_id.to_string(),
                    sender_device_id: status.device_id,
                    download_host,
                    download_port,
                    files,
                },
            )
            .await;
        if !sent {
            return Err(AppError::InvalidInput(
                "receiver is not connected or trusted".to_string(),
            ));
        }
        Ok(())
    }

    async fn update_progress(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        file_id: &str,
        file_transferred_bytes: u64,
    ) -> AppResult<FileTransferTask> {
        self.update_progress_with_optional_events(
            Some(app),
            transfer_id,
            file_id,
            file_transferred_bytes,
            false,
        )
        .await
        .map(|(task, _)| task)
    }

    async fn update_progress_with_optional_events(
        &self,
        app: Option<&AppHandle>,
        transfer_id: &str,
        file_id: &str,
        file_transferred_bytes: u64,
        force: bool,
    ) -> AppResult<(FileTransferTask, bool)> {
        let (task, persist, emit) = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            if !matches!(
                entry.task.status,
                FileTransferStatus::Canceled
                    | FileTransferStatus::Completed
                    | FileTransferStatus::Failed
                    | FileTransferStatus::Rejected
            ) {
                let file = entry
                    .task
                    .files
                    .iter_mut()
                    .find(|file| file.id == file_id)
                    .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
                file.transferred_bytes = file_transferred_bytes.min(file.size);
                if file.status != FileTransferFileStatus::Completed {
                    file.status = FileTransferFileStatus::Transferring;
                }
                entry.task.transferred_bytes = entry
                    .task
                    .files
                    .iter()
                    .map(|file| file.transferred_bytes.min(file.size))
                    .sum::<u64>()
                    .min(entry.task.total_size);
                entry.task.status = FileTransferStatus::Transferring;
                entry.last_activity_at = Utc::now();
            }
            let now = Instant::now();
            let persist = force
                || should_persist_progress(
                    entry.last_persisted_bytes,
                    entry.last_persisted_at,
                    entry.task.transferred_bytes,
                    now,
                );
            if persist {
                entry.last_persisted_bytes = entry.task.transferred_bytes;
                entry.last_persisted_at = now;
            }
            let emit = force || should_emit_progress(entry.last_emitted_at, now);
            if emit {
                entry.last_emitted_at = Some(now);
            }
            (entry.task.clone(), persist, emit)
        };
        if persist {
            self.persist_transfer(transfer_id).await?;
        }
        if emit {
            if let Some(app) = app {
            emit_progress(app, &task, file_id);
            emit_task_updated(app, &task);
            }
        }
        Ok((task, emit))
    }

    async fn is_canceled(&self, transfer_id: &str) -> bool {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| entry.task.status == FileTransferStatus::Canceled)
            .unwrap_or(true)
    }

    async fn receive_should_wait_for_peer(&self, transfer_id: &str) -> bool {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| entry.task.status == FileTransferStatus::WaitingForPeer)
            .unwrap_or(true)
    }

    pub async fn accept_receive(
        self: Arc<Self>,
        app: AppHandle,
        state: AppState,
        transfer_id: String,
    ) -> AppResult<FileTransferTask> {
        let task = self.mark_status(&transfer_id, FileTransferStatus::Accepted).await?;
        let status = state.status().await;
        let sent = state
            .send_trusted_to_device(
                &state.config().await,
                &task.peer_device_id,
                WireMessage::FileAccept {
                    transfer_id: transfer_id.clone(),
                    receiver_device_id: status.device_id,
                    receiver_device_name: status.device_name,
                },
            )
            .await;
        if !sent {
            let waiting = self
                .mark_waiting_for_peer(&transfer_id, "sender device is offline")
                .await?;
            self.emit_receive_state(&app, &state, &waiting).await;
            return Ok(waiting);
        }
        emit_task_updated(&app, &task);
        self.clone()
            .start_receive_worker(app, state, transfer_id)
            .await;

        Ok(task)
    }

    async fn start_receive_worker(
        self: Arc<Self>,
        app: AppHandle,
        state: AppState,
        transfer_id: String,
    ) {
        if !self
            .active_receive_workers
            .lock()
            .await
            .insert(transfer_id.clone())
        {
            return;
        }
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            let result = manager
                .clone()
                .download_receive(app.clone(), state.clone(), transfer_id.clone())
                .await;
            manager
                .active_receive_workers
                .lock()
                .await
                .remove(&transfer_id);
            manager
                .handle_receive_worker_result(&app, &state, &transfer_id, result)
                .await;
        });
    }

    async fn download_receive(
        self: Arc<Self>,
        app: AppHandle,
        state: AppState,
        transfer_id: String,
    ) -> AppResult<ReceiveRunOutcome> {
        let config = state.config().await;
        let save_dir = transfer_save_dir(&config)?;
        fs::create_dir_all(&save_dir).await?;

        loop {
            if self.is_canceled(&transfer_id).await {
                self.cleanup_temp(&transfer_id).await;
                return Ok(ReceiveRunOutcome::Completed);
            }
            let Some((host, port, peer_device_id, plan)) =
                self.next_receive_plan(&transfer_id).await?
            else {
                break;
            };
            let (temp_path, _) = self
                .ensure_receive_paths(&transfer_id, &plan.file.id, &save_dir, &plan.file.name)
                .await?;
            let actual_offset = fs::metadata(&temp_path)
                .await
                .map(|metadata| metadata.len())
                .or_else(|error| {
                    if error.kind() == std::io::ErrorKind::NotFound {
                        Ok(0)
                    } else {
                        Err(error)
                    }
                })?;
            if actual_offset < plan.file.size
                && (plan.token.is_none()
                    || plan.token_offset != actual_offset
                    || host.is_none()
                    || port.is_none())
            {
                return Ok(ReceiveRunOutcome::NeedsGrant);
            }
            let (host, port) = if actual_offset == plan.file.size {
                (host.unwrap_or_default(), port.unwrap_or_default())
            } else {
                (
                    host.ok_or_else(|| {
                        AppError::InvalidInput("missing download host".to_string())
                    })?,
                    port.ok_or_else(|| {
                        AppError::InvalidInput("missing download port".to_string())
                    })?,
                )
            };
            match self
                .download_one_receive(
                    &app,
                    &state,
                    &transfer_id,
                    &peer_device_id,
                    &host,
                    port,
                    &save_dir,
                    &plan,
                )
                .await
            {
                Ok(_) => {}
                Err(_error) if self.is_canceled(&transfer_id).await => {
                    self.cleanup_temp(&transfer_id).await;
                    return Ok(ReceiveRunOutcome::Completed);
                }
                Err(AppError::InvalidInput(message))
                    if message == "resume authorization expired" =>
                {
                    return Ok(ReceiveRunOutcome::NeedsGrant);
                }
                Err(AppError::InvalidInput(message))
                    if message == "restart download from zero" =>
                {
                    self.reset_current_partial(&transfer_id, &plan.file.id).await?;
                    return Ok(ReceiveRunOutcome::NeedsGrant);
                }
                Err(error) => return Err(error),
            }
        }

        let completed = self.mark_completed(&transfer_id).await?;
        if completed.clipboard_sync {
            if let Err(error) = apply_completed_clipboard_file_sync(&app, &state, &completed).await {
                tracing::warn!("clipboard file sync apply failed: {error}");
            }
        }
        let completed_files = completed
            .files
            .iter()
            .map(|file| FileCompleteFile {
                file_id: file.id.clone(),
                sha256: file.sha256.clone(),
            })
            .collect();
        let _ = state
            .send_trusted_to_device(
                &state.config().await,
                &completed.peer_device_id,
                WireMessage::FileComplete {
                    transfer_id: transfer_id.clone(),
                    device_id: state.status().await.device_id,
                    files: completed_files,
                },
            )
            .await;
        emit_task_completed(&app, &completed);
        if let Err(error) = maybe_open_folder_after_save(&state.config().await, &save_dir) {
            tracing::warn!("auto open transfer folder failed: {error}");
        }
        Ok(ReceiveRunOutcome::Completed)
    }

    async fn handle_receive_worker_result(
        &self,
        app: &AppHandle,
        state: &AppState,
        transfer_id: &str,
        result: AppResult<ReceiveRunOutcome>,
    ) {
        match result {
            Ok(ReceiveRunOutcome::Completed) => {}
            Ok(ReceiveRunOutcome::NeedsGrant) => {
                if let Err(error) = self.request_resume_grant(app, state, transfer_id).await {
                    if let Ok(task) = self.mark_paused(transfer_id, &error.to_string()).await {
                        self.emit_receive_state(app, state, &task).await;
                    }
                }
            }
            Err(_) if self.is_canceled(transfer_id).await => {
                self.cleanup_temp(transfer_id).await;
            }
            Err(error) => {
                let message = error.to_string();
                match error {
                    AppError::ConnectionTimeout(_) | AppError::WebSocket(_) => {
                        if trusted_transfer_peer(
                            &state.devices().await,
                            &self.peer_device_id(transfer_id).await,
                        )
                        .is_err()
                        {
                            if let Ok(task) =
                                self.mark_waiting_for_peer(transfer_id, &message).await
                            {
                                self.emit_receive_state(app, state, &task).await;
                            }
                            return;
                        }
                        match self.schedule_transient_retry(transfer_id, &message).await {
                            Ok(Some(delay)) => {
                                if let Some(task) = self.task(transfer_id).await {
                                    self.emit_receive_state(app, state, &task).await;
                                }
                                tokio::time::sleep(delay).await;
                                if let Err(error) =
                                    self.request_resume_grant(app, state, transfer_id).await
                                {
                                    if let Ok(task) =
                                        self.mark_paused(transfer_id, &error.to_string()).await
                                    {
                                        self.emit_receive_state(app, state, &task).await;
                                    }
                                }
                            }
                            Ok(None) => {
                                if let Some(task) = self.task(transfer_id).await {
                                    self.emit_receive_state(app, state, &task).await;
                                }
                            }
                            Err(error) => {
                                tracing::warn!("file transfer retry state failed: {error}");
                            }
                        }
                    }
                    AppError::Io(_) => {
                        if let Ok(task) = self.mark_paused(transfer_id, &message).await {
                            self.emit_receive_state(app, state, &task).await;
                        }
                    }
                    _ => {
                        if let Ok(task) = self.mark_failed(transfer_id, &message).await {
                            self.cleanup_temp(transfer_id).await;
                            if task.clipboard_sync {
                                let _ = update_clipboard_file_history_status(app, state, &task, None)
                                    .await;
                            }
                            let _ = state
                                .send_trusted_to_device(
                                    &state.config().await,
                                    &task.peer_device_id,
                                    WireMessage::FileError {
                                        transfer_id: transfer_id.to_string(),
                                        file_id: None,
                                        device_id: state.status().await.device_id,
                                        message,
                                    },
                                )
                                .await;
                            emit_task_failed(app, &task);
                        }
                    }
                }
            }
        }
    }

    async fn request_resume_grant(
        &self,
        app: &AppHandle,
        state: &AppState,
        transfer_id: &str,
    ) -> AppResult<()> {
        let (peer_device_id, file_id, offset) = {
            let tasks = self.tasks.lock().await;
            let entry = tasks
                .get(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            if entry.task.direction != FileTransferDirection::Receive
                || is_terminal_transfer_status(&entry.task.status)
            {
                return Ok(());
            }
            let file = entry
                .task
                .files
                .iter()
                .find(|file| file.status != FileTransferFileStatus::Completed)
                .ok_or_else(|| AppError::InvalidInput("file transfer is already complete".to_string()))?;
            let managed = entry
                .files
                .get(&file.id)
                .ok_or_else(|| AppError::InvalidInput("missing managed file".to_string()))?;
            let offset = managed
                .temp_path
                .as_deref()
                .filter(|path| path.exists())
                .map(std::fs::metadata)
                .transpose()?
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            if offset > file.size {
                return Err(AppError::InvalidInput(
                    "partial file is larger than expected".to_string(),
                ));
            }
            (entry.task.peer_device_id.clone(), file.id.clone(), offset)
        };

        if trusted_transfer_peer(&state.devices().await, &peer_device_id).is_err() {
            let task = self
                .mark_waiting_for_peer(transfer_id, "sender device is offline")
                .await?;
            self.emit_receive_state(app, state, &task).await;
            return Ok(());
        }
        if !state
            .peer_supports(&peer_device_id, FILE_RESUME_CAPABILITY)
            .await
        {
            let task = self
                .mark_paused(
                    transfer_id,
                    "sender does not support resumable file transfers",
                )
                .await?;
            self.emit_receive_state(app, state, &task).await;
            return Ok(());
        }

        let task = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            let managed = entry
                .files
                .get_mut(&file_id)
                .ok_or_else(|| AppError::InvalidInput("missing managed file".to_string()))?;
            managed.token = None;
            managed.token_consumed = false;
            managed.token_offset = offset;
            managed.token_receiver_device_id = None;
            entry.task.status = FileTransferStatus::Retrying;
            entry.task.completed_at = None;
            entry.last_activity_at = Utc::now();
            entry.task.clone()
        };
        self.persist_transfer(transfer_id).await?;
        self.emit_receive_state(app, state, &task).await;

        let status = state.status().await;
        let sent = state
            .send_trusted_to_device(
                &state.config().await,
                &peer_device_id,
                WireMessage::FileResumeRequest {
                    transfer_id: transfer_id.to_string(),
                    receiver_device_id: status.device_id,
                    files: vec![FileResumeOffset { file_id, offset }],
                },
            )
            .await;
        if !sent {
            let task = self
                .mark_waiting_for_peer(transfer_id, "sender device is offline")
                .await?;
            self.emit_receive_state(app, state, &task).await;
        }
        Ok(())
    }

    async fn emit_receive_state(
        &self,
        app: &AppHandle,
        state: &AppState,
        task: &FileTransferTask,
    ) {
        emit_task_updated(app, task);
        if task.clipboard_sync {
            let _ = update_clipboard_file_history_status(app, state, task, None).await;
        }
    }

    async fn peer_device_id(&self, transfer_id: &str) -> String {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| entry.task.peer_device_id.clone())
            .unwrap_or_default()
    }

    async fn mark_waiting_for_peer(
        &self,
        transfer_id: &str,
        message: &str,
    ) -> AppResult<FileTransferTask> {
        self.mark_resumable_status(
            transfer_id,
            FileTransferStatus::WaitingForPeer,
            message,
        )
        .await
    }

    async fn mark_peer_disconnected(
        &self,
        peer_device_id: &str,
    ) -> AppResult<Vec<FileTransferTask>> {
        let changed = {
            let mut tasks = self.tasks.lock().await;
            tasks
                .values_mut()
                .filter(|entry| {
                    entry.task.peer_device_id == peer_device_id
                        && matches!(
                            entry.task.status,
                            FileTransferStatus::Accepted
                                | FileTransferStatus::Transferring
                                | FileTransferStatus::Retrying
                        )
                })
                .map(|entry| {
                    entry.task.status = FileTransferStatus::WaitingForPeer;
                    entry.task.error = Some("peer device is offline".to_string());
                    entry.task.completed_at = None;
                    entry.last_activity_at = Utc::now();
                    entry.task.clone()
                })
                .collect::<Vec<_>>()
        };
        for task in &changed {
            self.persist_transfer(&task.transfer_id).await?;
        }
        Ok(changed)
    }

    async fn waiting_receive_transfer_ids(&self, peer_device_id: &str) -> Vec<String> {
        self.tasks
            .lock()
            .await
            .values()
            .filter(|entry| {
                entry.task.direction == FileTransferDirection::Receive
                    && entry.task.peer_device_id == peer_device_id
                    && entry.task.status == FileTransferStatus::WaitingForPeer
            })
            .map(|entry| entry.task.transfer_id.clone())
            .collect()
    }

    async fn completed_receive_tasks_for_peer(
        &self,
        peer_device_id: &str,
    ) -> Vec<FileTransferTask> {
        self.tasks
            .lock()
            .await
            .values()
            .filter(|entry| {
                entry.task.direction == FileTransferDirection::Receive
                    && entry.task.peer_device_id == peer_device_id
                    && entry.task.status == FileTransferStatus::Completed
            })
            .map(|entry| entry.task.clone())
            .collect()
    }

    async fn mark_paused(
        &self,
        transfer_id: &str,
        message: &str,
    ) -> AppResult<FileTransferTask> {
        self.mark_resumable_status(transfer_id, FileTransferStatus::Paused, message)
            .await
    }

    async fn mark_resumable_status(
        &self,
        transfer_id: &str,
        status: FileTransferStatus,
        message: &str,
    ) -> AppResult<FileTransferTask> {
        let task = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            if is_terminal_transfer_status(&entry.task.status) {
                return Ok(entry.task.clone());
            }
            entry.task.status = status;
            entry.task.error = Some(message.to_string());
            entry.task.completed_at = None;
            entry.last_activity_at = Utc::now();
            entry.task.clone()
        };
        self.persist_transfer(transfer_id).await?;
        Ok(task)
    }

    async fn reset_current_partial(
        &self,
        transfer_id: &str,
        file_id: &str,
    ) -> AppResult<()> {
        let temp_path = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            let managed = entry
                .files
                .get_mut(file_id)
                .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
            managed.token = None;
            managed.token_consumed = false;
            managed.token_offset = 0;
            managed.token_receiver_device_id = None;
            let temp_path = managed.temp_path.clone();
            if let Some(file) = entry.task.files.iter_mut().find(|file| file.id == file_id) {
                file.transferred_bytes = 0;
                file.status = FileTransferFileStatus::Pending;
                file.error = None;
            }
            entry.task.transferred_bytes = entry
                .task
                .files
                .iter()
                .map(|file| file.transferred_bytes.min(file.size))
                .sum::<u64>()
                .min(entry.task.total_size);
            entry.last_activity_at = Utc::now();
            temp_path
        };
        if let Some(temp_path) = temp_path {
            match fs::remove_file(temp_path).await {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        self.persist_transfer(transfer_id).await
    }

    async fn apply_resume_grant(
        &self,
        transfer_id: &str,
        sender_device_id: &str,
        download_host: String,
        download_port: u16,
        receiver_device_id: &str,
        granted_files: Vec<FileResumeGrantFile>,
    ) -> AppResult<FileTransferTask> {
        if granted_files.is_empty() {
            return Err(AppError::InvalidInput(
                "resume grant does not contain a file".to_string(),
            ));
        }
        let task = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            if entry.task.direction != FileTransferDirection::Receive
                || entry.task.peer_device_id != sender_device_id
            {
                return Err(AppError::InvalidInput(
                    "resume sender does not match transfer peer".to_string(),
                ));
            }
            let current_file_id = entry
                .task
                .files
                .iter()
                .find(|file| file.status != FileTransferFileStatus::Completed)
                .map(|file| file.id.clone())
                .ok_or_else(|| AppError::InvalidInput("file transfer is already complete".to_string()))?;
            if granted_files.len() != 1 || granted_files[0].file_id != current_file_id {
                return Err(AppError::InvalidInput(
                    "resume grant does not match the current file".to_string(),
                ));
            }
            let grant = &granted_files[0];
            let task_file = entry
                .task
                .files
                .iter()
                .find(|file| file.id == grant.file_id)
                .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
            let managed = entry
                .files
                .get_mut(&grant.file_id)
                .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
            let actual_offset = managed
                .temp_path
                .as_deref()
                .filter(|path| path.exists())
                .map(std::fs::metadata)
                .transpose()?
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            if actual_offset > task_file.size || grant.offset != actual_offset {
                return Err(AppError::InvalidInput(
                    "resume grant offset does not match partial file".to_string(),
                ));
            }
            managed.token = Some(grant.token.clone());
            managed.token_consumed = false;
            managed.token_offset = grant.offset;
            managed.token_expires_at = None;
            managed.token_receiver_device_id = Some(receiver_device_id.to_string());
            entry.download_host = Some(download_host);
            entry.download_port = Some(download_port);
            entry.task.status = FileTransferStatus::Retrying;
            entry.task.completed_at = None;
            entry.task.error = None;
            entry.last_activity_at = Utc::now();
            entry.task.clone()
        };
        self.persist_transfer(transfer_id).await?;
        Ok(task)
    }

    async fn next_receive_plan(
        &self,
        transfer_id: &str,
    ) -> AppResult<Option<(Option<String>, Option<u16>, String, ReceiveFilePlan)>> {
        let tasks = self.tasks.lock().await;
        let entry = tasks
            .get(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        let Some(file) = entry
            .task
            .files
            .iter()
            .find(|file| file.status != FileTransferFileStatus::Completed)
        else {
            return Ok(None);
        };
        let managed = entry
            .files
            .get(&file.id)
            .ok_or_else(|| AppError::InvalidInput("missing managed file".to_string()))?;
        Ok(Some((
            entry.download_host.clone(),
            entry.download_port,
            entry.task.peer_device_id.clone(),
            ReceiveFilePlan {
                file: file.clone(),
                token: managed.token.clone(),
                token_offset: managed.token_offset,
                receiver_device_id: managed.token_receiver_device_id.clone(),
            },
        )))
    }

    async fn download_one_receive(
        &self,
        app: &AppHandle,
        state: &AppState,
        transfer_id: &str,
        peer_device_id: &str,
        host: &str,
        port: u16,
        save_dir: &Path,
        plan: &ReceiveFilePlan,
    ) -> AppResult<String> {
        let (temp_path, final_path) = self
            .ensure_receive_paths(transfer_id, &plan.file.id, save_dir, &plan.file.name)
            .await?;
        self.prepare_receive_file(
            app,
            transfer_id,
            &plan.file.id,
            temp_path.clone(),
            final_path.clone(),
        )
        .await?;

        let (offset, hasher) = hash_partial_file(&temp_path, plan.file.size).await?;
        if offset != plan.token_offset {
            return Err(AppError::InvalidInput(
                "resume authorization expired".to_string(),
            ));
        }
        if offset == plan.file.size {
            let actual_hash = format!("{:x}", hasher.finalize());
            if actual_hash != plan.file.sha256 {
                let _ = fs::remove_file(&temp_path).await;
                return Err(AppError::InvalidInput("file sha256 mismatch".to_string()));
            }
            fs::rename(&temp_path, &final_path).await?;
            self.mark_file_completed(
                app,
                transfer_id,
                &plan.file.id,
                Some(final_path.to_string_lossy().to_string()),
            )
            .await?;
            return Ok(actual_hash);
        }

        let token = plan
            .token
            .as_deref()
            .ok_or_else(|| AppError::InvalidInput("resume authorization expired".to_string()))?;
        {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            let managed = entry
                .files
                .get_mut(&plan.file.id)
                .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
            if managed.token.as_deref() != Some(token) || managed.token_offset != offset {
                return Err(AppError::InvalidInput(
                    "resume authorization expired".to_string(),
                ));
            }
            managed.token = None;
            managed.token_consumed = true;
        }

        let request_target = download_request_target(
            transfer_id,
            &plan.file.id,
            token,
            plan.receiver_device_id.as_deref(),
        );
        let actual_hash = receive_http_to_part(
            host,
            port,
            &request_target,
            offset,
            plan.receiver_device_id.is_some(),
            plan.file.size,
            &plan.file.sha256,
            &temp_path,
            hasher,
            |transferred| async move {
                if self.is_canceled(transfer_id).await {
                    return Err(AppError::InvalidInput("transfer canceled".to_string()));
                }
                if self.receive_should_wait_for_peer(transfer_id).await {
                    return Err(AppError::ConnectionTimeout(
                        "sender device disconnected".to_string(),
                    ));
                }
                self.update_receive_progress(
                    app,
                    state,
                    transfer_id,
                    peer_device_id,
                    &plan.file.id,
                    transferred,
                )
                .await
            },
        )
        .await?;

        fs::rename(&temp_path, &final_path).await?;
        self.mark_file_completed(
            app,
            transfer_id,
            &plan.file.id,
            Some(final_path.to_string_lossy().to_string()),
        )
        .await?;
        Ok(actual_hash)
    }

    async fn ensure_receive_paths(
        &self,
        transfer_id: &str,
        file_id: &str,
        save_dir: &Path,
        file_name: &str,
    ) -> AppResult<(PathBuf, PathBuf)> {
        let paths = {
            let mut tasks = self.tasks.lock().await;
            let reserved_paths = tasks
                .values()
                .flat_map(|entry| entry.files.values())
                .filter_map(|file| file.final_path.clone())
                .collect::<HashSet<_>>();
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            let managed = entry
                .files
                .get_mut(file_id)
                .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
            match (&managed.temp_path, &managed.final_path) {
                (Some(temp), Some(final_path)) => (temp.clone(), final_path.clone()),
                _ => {
                    let final_path =
                        unique_save_path_with_reserved(save_dir, file_name, &reserved_paths);
                    let temp_path = part_path_for(&final_path);
                    managed.temp_path = Some(temp_path.clone());
                    managed.final_path = Some(final_path.clone());
                    entry.last_activity_at = Utc::now();
                    (temp_path, final_path)
                }
            }
        };
        self.persist_transfer(transfer_id).await?;
        Ok(paths)
    }

    async fn prepare_receive_file(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        file_id: &str,
        temp_path: PathBuf,
        final_path: PathBuf,
    ) -> AppResult<FileTransferTask> {
        let task = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            let managed_file = entry
                .files
                .get_mut(file_id)
                .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
            managed_file.temp_path = Some(temp_path);
            managed_file.final_path = Some(final_path);
            entry.task.status = FileTransferStatus::Transferring;
            if let Some(file) = entry.task.files.iter_mut().find(|file| file.id == file_id) {
                file.status = FileTransferFileStatus::Transferring;
                file.error = None;
            }
            entry.task.clone()
        };
        self.persist_transfer(transfer_id).await?;
        emit_task_updated(app, &task);
        Ok(task)
    }

    async fn update_receive_progress(
        &self,
        app: &AppHandle,
        state: &AppState,
        transfer_id: &str,
        peer_device_id: &str,
        file_id: &str,
        transferred: u64,
    ) -> AppResult<()> {
        let (task, should_notify) = self
            .update_progress_with_optional_events(
                Some(app),
                transfer_id,
                file_id,
                transferred,
                false,
            )
            .await?;
        if !should_notify {
            return Ok(());
        }
        let file = task
            .files
            .iter()
            .find(|file| file.id == file_id)
            .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
        let _ = state
            .send_trusted_to_device(
                &state.config().await,
                peer_device_id,
                WireMessage::FileProgress {
                    transfer_id: transfer_id.to_string(),
                    device_id: state.status().await.device_id,
                    file_id: file_id.to_string(),
                    file_transferred_bytes: file.transferred_bytes,
                    file_size: file.size,
                    total_transferred_bytes: task.transferred_bytes,
                    total_size: task.total_size,
                },
            )
            .await;
        Ok(())
    }

    pub async fn reject_receive(
        &self,
        app: &AppHandle,
        state: &AppState,
        transfer_id: &str,
    ) -> AppResult<FileTransferTask> {
        let task = self.mark_status(transfer_id, FileTransferStatus::Rejected).await?;
        let _ = state
            .send_trusted_to_device(
                &state.config().await,
                &task.peer_device_id,
                WireMessage::FileReject {
                    transfer_id: transfer_id.to_string(),
                    receiver_device_id: state.status().await.device_id,
                    reason: Some("receiver rejected file transfer".to_string()),
                },
            )
            .await;
        emit_task_updated(app, &task);
        Ok(task)
    }

    pub async fn cancel_and_notify(
        &self,
        app: &AppHandle,
        state: &AppState,
        transfer_id: &str,
    ) -> AppResult<FileTransferTask> {
        let task = self.cancel_local(transfer_id).await?;
        self.cleanup_temp(transfer_id).await;
        let _ = state
            .send_trusted_to_device(
                &state.config().await,
                &task.peer_device_id,
                WireMessage::FileCancel {
                    transfer_id: transfer_id.to_string(),
                    device_id: state.status().await.device_id,
                },
            )
            .await;
        emit_task_updated(app, &task);
        Ok(task)
    }

    pub async fn cancel_local(&self, transfer_id: &str) -> AppResult<FileTransferTask> {
        self.mark_status(transfer_id, FileTransferStatus::Canceled).await
    }

    pub async fn handle_offer(
        &self,
        app: &AppHandle,
        offer: IncomingFileOffer,
        max_receive_file_size_mib: u32,
    ) -> AppResult<FileTransferTask> {
        if offer.file_count != offer.files.len() {
            return Err(AppError::InvalidInput("file count mismatch".to_string()));
        }
        let sizes = offer.files.iter().map(|file| file.file_size).collect::<Vec<_>>();
        validate_transfer_limits(offer.file_count, &sizes)?;
        validate_configured_single_file_limit(
            &sizes,
            max_receive_file_size_mib,
            "receive",
        )?;
        let calculated_total = sizes.iter().sum::<u64>();
        if calculated_total != offer.total_size {
            return Err(AppError::InvalidInput("total size mismatch".to_string()));
        }

        let mut task_files = Vec::with_capacity(offer.files.len());
        let mut managed_files = HashMap::new();
        for offered_file in offer.files {
            let file_name = sanitize_file_name(&offered_file.file_name);
            let thumbnail = offered_file.thumbnail.clone();
            task_files.push(new_transfer_file(
                offered_file.file_id.clone(),
                file_name,
                offered_file.file_size,
                offered_file.sha256,
                thumbnail,
            ));
            managed_files.insert(
                offered_file.file_id,
                ManagedTransferFile {
                    source_path: None,
                    source_size: None,
                    source_modified_ms: None,
                    token: Some(offered_file.token),
                    token_consumed: false,
                    token_offset: 0,
                    token_expires_at: None,
                    token_receiver_device_id: None,
                    temp_path: None,
                    final_path: None,
                },
            );
        }

        let task = FileTransferTask {
            transfer_id: offer.transfer_id.clone(),
            direction: FileTransferDirection::Receive,
            peer_device_id: offer.sender_device_id,
            peer_device_name: offer.sender_device_name,
            clipboard_sync: offer.clipboard_sync,
            files: task_files,
            total_size: offer.total_size,
            transferred_bytes: 0,
            status: FileTransferStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        };
        self.tasks.lock().await.insert(
            offer.transfer_id,
            ManagedTransfer {
                task: task.clone(),
                files: managed_files,
                download_host: Some(offer.download_host),
                download_port: Some(offer.download_port),
                retry_count: 0,
                last_activity_at: Utc::now(),
                last_persisted_bytes: 0,
                last_persisted_at: Instant::now(),
                last_emitted_at: None,
            },
        );
        self.persist_transfer(&task.transfer_id).await?;
        if !task.clipboard_sync {
            notifications::notify_file_transfer_offer(app, &task);
        }
        let _ = app.emit("file-transfer-offer", task.clone());
        emit_task_updated(app, &task);
        Ok(task)
    }

    pub async fn handle_accept(
        &self,
        app: &AppHandle,
        transfer_id: &str,
    ) -> AppResult<FileTransferTask> {
        let task = self
            .mark_status(transfer_id, FileTransferStatus::Accepted)
            .await?;
        emit_task_updated(app, &task);
        Ok(task)
    }

    pub async fn handle_reject(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        reason: Option<String>,
    ) -> AppResult<FileTransferTask> {
        let mut task = self.mark_status(transfer_id, FileTransferStatus::Rejected).await?;
        if let Some(reason) = reason {
            task = self.mark_error_text(transfer_id, reason).await?;
        }
        emit_task_updated(app, &task);
        Ok(task)
    }

    pub async fn handle_progress(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        file_id: &str,
        file_transferred_bytes: u64,
    ) -> AppResult<FileTransferTask> {
        self.update_progress(app, transfer_id, file_id, file_transferred_bytes)
            .await
    }

    pub async fn handle_complete(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        completed_files: &[FileCompleteFile],
    ) -> AppResult<FileTransferTask> {
        let expected = {
            let tasks = self.tasks.lock().await;
            tasks
                .get(transfer_id)
                .map(|entry| {
                    entry
                        .task
                        .files
                        .iter()
                        .map(|file| (file.id.clone(), file.sha256.clone()))
                        .collect::<HashMap<_, _>>()
                })
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?
        };
        let received = completed_files
            .iter()
            .map(|file| (file.file_id.clone(), file.sha256.clone()))
            .collect::<HashMap<_, _>>();
        let hashes_match = expected.len() == received.len()
            && expected
                .iter()
                .all(|(file_id, sha256)| received.get(file_id) == Some(sha256));

        let task = if hashes_match {
            self.mark_completed(transfer_id).await?
        } else {
            self.mark_failed(transfer_id, "peer file sha256 mismatch")
                .await?
        };
        if task.status == FileTransferStatus::Completed {
            emit_task_completed(app, &task);
        } else {
            emit_task_failed(app, &task);
        }
        Ok(task)
    }

    pub async fn handle_cancel(
        &self,
        app: &AppHandle,
        transfer_id: &str,
    ) -> AppResult<FileTransferTask> {
        let task = self.cancel_local(transfer_id).await?;
        self.cleanup_temp(transfer_id).await;
        emit_task_updated(app, &task);
        Ok(task)
    }

    pub async fn handle_error(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        file_id: Option<&str>,
        message: &str,
    ) -> AppResult<FileTransferTask> {
        let task = if let Some(file_id) = file_id {
            self.mark_file_failed(transfer_id, file_id, message).await?
        } else {
            self.mark_failed(transfer_id, message).await?
        };
        if task.direction == FileTransferDirection::Receive {
            self.cleanup_temp(transfer_id).await;
        }
        emit_task_failed(app, &task);
        Ok(task)
    }

    async fn mark_status(
        &self,
        transfer_id: &str,
        status: FileTransferStatus,
    ) -> AppResult<FileTransferTask> {
        let (task, direction) = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry.task.status = status;
            entry.last_activity_at = Utc::now();
            match entry.task.status {
                FileTransferStatus::Canceled => {
                    for file in &mut entry.task.files {
                        if file.status != FileTransferFileStatus::Completed {
                            file.status = FileTransferFileStatus::Canceled;
                        }
                    }
                }
                FileTransferStatus::Rejected => {
                    for file in &mut entry.task.files {
                        if file.status == FileTransferFileStatus::Pending {
                            file.status = FileTransferFileStatus::Canceled;
                        }
                    }
                }
                FileTransferStatus::Transferring => {
                    for file in &mut entry.task.files {
                        if file.status == FileTransferFileStatus::Pending {
                            file.status = FileTransferFileStatus::Transferring;
                            break;
                        }
                    }
                }
                _ => {}
            }
            if is_terminal_transfer_status(&entry.task.status) {
                entry.task.completed_at = Some(Utc::now());
            }
            let task = entry.task.clone();
            let direction = task.direction.clone();
            prune_terminal_transfer_tasks(&mut tasks);
            (task, direction)
        };
        if is_terminal_transfer_status(&task.status) {
            self.remove_persisted_transfer(&direction, transfer_id).await?;
        } else {
            self.persist_transfer(transfer_id).await?;
        }
        Ok(task)
    }

    async fn mark_file_completed(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        file_id: &str,
        saved_path: Option<String>,
    ) -> AppResult<FileTransferTask> {
        let task = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            let file = entry
                .task
                .files
                .iter_mut()
                .find(|file| file.id == file_id)
                .ok_or_else(|| AppError::InvalidInput("file not found in transfer".to_string()))?;
            file.status = FileTransferFileStatus::Completed;
            file.transferred_bytes = file.size;
            file.saved_path = saved_path;
            file.error = None;
            entry.task.transferred_bytes = entry
                .task
                .files
                .iter()
                .map(|file| file.transferred_bytes.min(file.size))
                .sum::<u64>()
                .min(entry.task.total_size);
            if entry.task.status != FileTransferStatus::Canceled {
                entry.task.status = FileTransferStatus::Transferring;
            }
            entry.last_activity_at = Utc::now();
            entry.task.clone()
        };
        self.persist_transfer(transfer_id).await?;
        emit_progress(app, &task, file_id);
        emit_task_updated(app, &task);
        Ok(task)
    }

    async fn mark_completed(&self, transfer_id: &str) -> AppResult<FileTransferTask> {
        let (task, direction) = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry.task.status = FileTransferStatus::Completed;
            entry.task.transferred_bytes = entry.task.total_size;
            entry.task.completed_at = Some(Utc::now());
            entry.task.error = None;
            entry.last_activity_at = Utc::now();
            for file in &mut entry.task.files {
                file.status = FileTransferFileStatus::Completed;
                file.transferred_bytes = file.size;
                file.error = None;
            }
            let task = entry.task.clone();
            let direction = task.direction.clone();
            prune_terminal_transfer_tasks(&mut tasks);
            (task, direction)
        };
        self.remove_persisted_transfer(&direction, transfer_id).await?;
        Ok(task)
    }

    async fn mark_error_text(
        &self,
        transfer_id: &str,
        message: String,
    ) -> AppResult<FileTransferTask> {
        let task = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry.task.error = Some(message);
            entry.last_activity_at = Utc::now();
            entry.task.clone()
        };
        self.persist_transfer(transfer_id).await?;
        Ok(task)
    }

    async fn schedule_transient_retry(
        &self,
        transfer_id: &str,
        message: &str,
    ) -> AppResult<Option<Duration>> {
        let delay = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry.task.error = Some(message.to_string());
            entry.task.completed_at = None;
            entry.last_activity_at = Utc::now();
            if let Some(delay) = RETRY_DELAYS.get(entry.retry_count as usize).copied() {
                entry.retry_count += 1;
                entry.task.status = FileTransferStatus::Retrying;
                Some(delay)
            } else {
                entry.task.status = FileTransferStatus::Paused;
                None
            }
        };
        self.persist_transfer(transfer_id).await?;
        Ok(delay)
    }

    async fn reset_manual_retry(&self, transfer_id: &str) -> AppResult<FileTransferTask> {
        let task = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            if entry.task.direction != FileTransferDirection::Receive
                || !matches!(
                    entry.task.status,
                    FileTransferStatus::Paused | FileTransferStatus::WaitingForPeer
                )
            {
                return Err(AppError::InvalidInput(
                    "file transfer cannot be resumed in its current state".to_string(),
                ));
            }
            entry.retry_count = 0;
            entry.task.status = FileTransferStatus::Retrying;
            entry.task.error = None;
            entry.task.completed_at = None;
            entry.last_activity_at = Utc::now();
            entry.task.clone()
        };
        self.persist_transfer(transfer_id).await?;
        Ok(task)
    }

    async fn reset_oversized_partial_if_needed(&self, transfer_id: &str) -> AppResult<()> {
        let oversized_file_id = {
            let tasks = self.tasks.lock().await;
            let entry = tasks
                .get(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry
                .task
                .files
                .iter()
                .find(|file| file.status != FileTransferFileStatus::Completed)
                .and_then(|file| {
                    entry.files.get(&file.id).and_then(|managed| {
                        managed
                            .temp_path
                            .as_deref()
                            .filter(|path| path.exists())
                            .and_then(|path| std::fs::metadata(path).ok())
                            .filter(|metadata| metadata.len() > file.size)
                            .map(|_| file.id.clone())
                    })
                })
        };
        if let Some(file_id) = oversized_file_id {
            self.reset_current_partial(transfer_id, &file_id).await?;
        }
        Ok(())
    }

    async fn mark_failed(&self, transfer_id: &str, message: &str) -> AppResult<FileTransferTask> {
        let (task, direction) = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry.task.status = FileTransferStatus::Failed;
            entry.task.error = Some(message.to_string());
            entry.task.completed_at = Some(Utc::now());
            entry.last_activity_at = Utc::now();
            for file in &mut entry.task.files {
                if file.status != FileTransferFileStatus::Completed {
                    file.status = FileTransferFileStatus::Failed;
                    file.error = Some(message.to_string());
                }
            }
            let task = entry.task.clone();
            let direction = task.direction.clone();
            prune_terminal_transfer_tasks(&mut tasks);
            (task, direction)
        };
        self.remove_persisted_transfer(&direction, transfer_id).await?;
        Ok(task)
    }

    async fn mark_file_failed(
        &self,
        transfer_id: &str,
        file_id: &str,
        message: &str,
    ) -> AppResult<FileTransferTask> {
        let (task, direction) = {
            let mut tasks = self.tasks.lock().await;
            let entry = tasks
                .get_mut(transfer_id)
                .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
            entry.task.status = FileTransferStatus::Failed;
            entry.task.error = Some(message.to_string());
            entry.task.completed_at = Some(Utc::now());
            entry.last_activity_at = Utc::now();
            if let Some(file) = entry.task.files.iter_mut().find(|file| file.id == file_id) {
                file.status = FileTransferFileStatus::Failed;
                file.error = Some(message.to_string());
            }
            let task = entry.task.clone();
            let direction = task.direction.clone();
            prune_terminal_transfer_tasks(&mut tasks);
            (task, direction)
        };
        self.remove_persisted_transfer(&direction, transfer_id).await?;
        Ok(task)
    }

    async fn cleanup_temp(&self, transfer_id: &str) {
        let temp_paths = self
            .tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| {
                entry
                    .files
                    .values()
                    .filter_map(|file| file.temp_path.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for temp_path in temp_paths {
            let _ = fs::remove_file(temp_path).await;
        }
    }

    #[cfg(test)]
    async fn insert_test_send_task(
        &self,
        peer_device_id: &str,
        source_path: PathBuf,
        file_name: &str,
        token: &str,
    ) -> String {
        self.insert_test_send_task_with_files(
            peer_device_id,
            vec![("file-1", source_path, file_name, token)],
        )
        .await
    }

    #[cfg(test)]
    async fn insert_test_send_task_with_files(
        &self,
        peer_device_id: &str,
        files: Vec<(&str, PathBuf, &str, &str)>,
    ) -> String {
        let transfer_id = Uuid::new_v4().to_string();
        let mut task_files = Vec::new();
        let mut managed_files = HashMap::new();
        for (file_id, source_path, file_name, token) in files {
            let metadata = std::fs::metadata(&source_path).unwrap();
            let size = metadata.len();
            task_files.push(new_transfer_file(
                file_id.to_string(),
                file_name.to_string(),
                size,
                "hash".to_string(),
                None,
            ));
            managed_files.insert(
                file_id.to_string(),
                ManagedTransferFile {
                    source_path: Some(source_path),
                    source_size: Some(size),
                    source_modified_ms: source_modified_ms(&metadata),
                    token: Some(token.to_string()),
                    token_consumed: false,
                    token_offset: 0,
                    token_expires_at: None,
                    token_receiver_device_id: None,
                    temp_path: None,
                    final_path: None,
                },
            );
        }
        let total_size = task_files.iter().map(|file| file.size).sum();
        let task = FileTransferTask {
            transfer_id: transfer_id.clone(),
            direction: FileTransferDirection::Send,
            peer_device_id: peer_device_id.to_string(),
            peer_device_name: peer_device_id.to_string(),
            clipboard_sync: false,
            files: task_files,
            total_size,
            transferred_bytes: 0,
            status: FileTransferStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        };
        self.tasks.lock().await.insert(
            transfer_id.clone(),
            ManagedTransfer {
                task,
                files: managed_files,
                download_host: None,
                download_port: None,
                retry_count: 0,
                last_activity_at: Utc::now(),
                last_persisted_bytes: 0,
                last_persisted_at: Instant::now(),
                last_emitted_at: None,
            },
        );
        transfer_id
    }

    #[cfg(test)]
    async fn insert_test_receive_task(
        &self,
        peer_device_id: &str,
        file_name: &str,
        size: u64,
        sha256: &str,
        temp_path: PathBuf,
    ) -> String {
        let transfer_id = Uuid::new_v4().to_string();
        let file = new_transfer_file(
            "file-1".to_string(),
            file_name.to_string(),
            size,
            sha256.to_string(),
            None,
        );
        let task = FileTransferTask {
            transfer_id: transfer_id.clone(),
            direction: FileTransferDirection::Receive,
            peer_device_id: peer_device_id.to_string(),
            peer_device_name: peer_device_id.to_string(),
            clipboard_sync: false,
            files: vec![file],
            total_size: size,
            transferred_bytes: 0,
            status: FileTransferStatus::Transferring,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        };
        self.tasks.lock().await.insert(
            transfer_id.clone(),
            ManagedTransfer {
                task,
                files: HashMap::from([(
                    "file-1".to_string(),
                    ManagedTransferFile {
                        source_path: None,
                        source_size: None,
                        source_modified_ms: None,
                        token: Some("token".to_string()),
                        token_consumed: false,
                        token_offset: 0,
                        token_expires_at: None,
                        token_receiver_device_id: None,
                        temp_path: Some(temp_path),
                        final_path: None,
                    },
                )]),
                download_host: None,
                download_port: None,
                retry_count: 0,
                last_activity_at: Utc::now(),
                last_persisted_bytes: 0,
                last_persisted_at: Instant::now(),
                last_emitted_at: None,
            },
        );
        transfer_id
    }

    #[cfg(test)]
    async fn claim_download_for_test(
        &self,
        transfer_id: &str,
        file_id: &str,
        token: &str,
    ) -> Result<DownloadClaim, DownloadClaimError> {
        self.claim_download(transfer_id, file_id, token, 0, None)
            .await
    }

    #[cfg(test)]
    async fn claim_resume_download_for_test(
        &self,
        transfer_id: &str,
        file_id: &str,
        token: &str,
        offset: u64,
        receiver_device_id: Option<&str>,
    ) -> Result<DownloadClaim, DownloadClaimError> {
        self.claim_download(
            transfer_id,
            file_id,
            token,
            offset,
            receiver_device_id,
        )
        .await
    }

    #[cfg(test)]
    async fn issue_resume_token_for_test(
        &self,
        transfer_id: &str,
        file_id: &str,
        receiver_device_id: &str,
        offset: u64,
        expires_at: DateTime<Utc>,
    ) -> AppResult<String> {
        self.issue_resume_token(
            transfer_id,
            file_id,
            receiver_device_id,
            offset,
            expires_at,
        )
        .await
    }

    #[cfg(test)]
    async fn create_resume_grants_for_test(
        &self,
        transfer_id: &str,
        receiver_device_id: &str,
        files: Vec<FileResumeOffset>,
    ) -> AppResult<Vec<FileResumeGrantFile>> {
        self.create_resume_grants(transfer_id, receiver_device_id, files)
            .await
    }

    #[cfg(test)]
    async fn set_store_root_for_test(&self, root: PathBuf) {
        self.set_store_root(root).await;
    }

    #[cfg(test)]
    async fn persist_transfer_for_test(&self, transfer_id: &str) -> AppResult<()> {
        self.persist_transfer(transfer_id).await
    }

    #[cfg(test)]
    async fn restore_from_store_for_test(&self) -> AppResult<Vec<FileTransferTask>> {
        self.restore_from_store().await
    }

    #[cfg(test)]
    async fn has_download_token_for_test(&self, transfer_id: &str, file_id: &str) -> bool {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .and_then(|entry| entry.files.get(file_id))
            .and_then(|file| file.token.as_ref())
            .is_some()
    }

    #[cfg(test)]
    async fn finish_receive_for_test(
        &self,
        transfer_id: &str,
        temp_path: PathBuf,
        final_path: PathBuf,
        expected_hash: String,
    ) -> AppResult<FileTransferTask> {
        let actual = sha256_file(&temp_path).await?;
        if actual != expected_hash {
            let task = self.mark_failed(transfer_id, "file sha256 mismatch").await?;
            let _ = fs::remove_file(temp_path).await;
            return Err(AppError::InvalidInput(
                task.error
                    .unwrap_or_else(|| "file sha256 mismatch".to_string()),
            ));
        }
        fs::rename(&temp_path, &final_path).await?;
        self.mark_completed(transfer_id).await
    }

    #[cfg(test)]
    async fn task_status_for_test(&self, transfer_id: &str) -> Option<FileTransferStatus> {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| entry.task.status.clone())
    }
}

pub async fn initialize(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let root = app.path().app_data_dir()?.join("file-transfers");
    let manager = manager();
    manager.set_store_root(root).await;
    let expired_tasks = manager.restore_from_store().await?;

    for task in expired_tasks {
        emit_task_failed(app, &task);
        if task.clipboard_sync {
            update_clipboard_file_history_status(app, state, &task, None).await?;
        }
    }

    for task in manager.tasks().await {
        emit_task_updated(app, &task);
        if task.clipboard_sync {
            let has_history = state.history().await.iter().any(|item| {
                item.file_transfer_id.as_deref() == Some(task.transfer_id.as_str())
            });
            if has_history {
                update_clipboard_file_history_status(app, state, &task, None).await?;
            } else {
                push_pending_clipboard_file_history(app, state, &task).await?;
            }
        }
        if task.direction == FileTransferDirection::Receive
            && task.status != FileTransferStatus::Pending
            && !is_terminal_transfer_status(&task.status)
            && task
                .files
                .iter()
                .all(|file| file.status == FileTransferFileStatus::Completed)
        {
            manager
                .clone()
                .start_receive_worker(
                    app.clone(),
                    state.clone(),
                    task.transfer_id.clone(),
                )
                .await;
        }
    }

    let app_for_cleanup = app.clone();
    let state_for_cleanup = state.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(TRANSFER_EXPIRY_INTERVAL);
        interval.tick().await;
        loop {
            interval.tick().await;
            match manager.prune_expired_transfers().await {
                Ok(tasks) => {
                    for task in tasks {
                        emit_task_failed(&app_for_cleanup, &task);
                        if task.clipboard_sync {
                            let _ = update_clipboard_file_history_status(
                                &app_for_cleanup,
                                &state_for_cleanup,
                                &task,
                                None,
                            )
                            .await;
                        }
                    }
                }
                Err(error) => tracing::warn!("file transfer expiry cleanup failed: {error}"),
            }
        }
    });
    Ok(())
}

pub async fn select_file_for_transfer(app: AppHandle) -> AppResult<Option<SelectedTransferFile>> {
    use tauri_plugin_dialog::DialogExt;

    let Some(path) = app.dialog().file().blocking_pick_file() else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|error| AppError::InvalidInput(error.to_string()))?;
    manager().selected_file_from_path(path).await.map(Some)
}

pub async fn select_files_for_transfer(app: AppHandle) -> AppResult<Vec<SelectedTransferFile>> {
    use tauri_plugin_dialog::DialogExt;

    let Some(paths) = app.dialog().file().blocking_pick_files() else {
        return Ok(Vec::new());
    };
    let paths = paths
        .into_iter()
        .map(|path| {
            path.into_path()
                .map_err(|error| AppError::InvalidInput(error.to_string()))
        })
        .collect::<AppResult<Vec<_>>>()?;
    manager().selected_files_from_paths(paths).await
}

pub async fn send_file_to_device(
    app: AppHandle,
    state: AppState,
    device_id: String,
    file_path: String,
) -> AppResult<FileTransferTask> {
    manager()
        .create_send_offer(&app, &state, device_id, PathBuf::from(file_path))
        .await
}

pub async fn send_files_to_device(
    app: AppHandle,
    state: AppState,
    device_id: String,
    file_paths: Vec<String>,
) -> AppResult<FileTransferTask> {
    manager()
        .create_send_offer_files(
            &app,
            &state,
            device_id,
            file_paths.into_iter().map(PathBuf::from).collect(),
        )
        .await
}

pub async fn send_clipboard_files_to_trusted_devices(
    app: AppHandle,
    state: AppState,
    file_paths: Vec<PathBuf>,
) -> AppResult<usize> {
    let devices = state
        .devices()
        .await
        .into_iter()
        .filter(|device| device.connected && device.trusted && device.remote_trusted)
        .collect::<Vec<_>>();
    let mut sent_count = 0usize;
    let mut first_error = None;

    for device in devices {
        match manager()
            .create_clipboard_send_offer_files(
                &app,
                &state,
                device.id.clone(),
                file_paths.clone(),
            )
            .await
        {
            Ok(_) => sent_count += 1,
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }

    if sent_count == 0 {
        if let Some(error) = first_error {
            return Err(error);
        }
    }
    Ok(sent_count)
}

pub async fn accept_file_transfer(
    app: AppHandle,
    state: AppState,
    transfer_id: String,
) -> AppResult<FileTransferTask> {
    manager().accept_receive(app, state, transfer_id).await
}

pub async fn reject_file_transfer(
    app: AppHandle,
    state: AppState,
    transfer_id: String,
) -> AppResult<FileTransferTask> {
    manager().reject_receive(&app, &state, &transfer_id).await
}

pub async fn cancel_file_transfer(
    app: AppHandle,
    state: AppState,
    transfer_id: String,
) -> AppResult<FileTransferTask> {
    manager().cancel_and_notify(&app, &state, &transfer_id).await
}

pub async fn resume_file_transfer(
    app: AppHandle,
    state: AppState,
    transfer_id: String,
) -> AppResult<FileTransferTask> {
    let manager = manager();
    manager
        .reset_oversized_partial_if_needed(&transfer_id)
        .await?;
    let task = manager.reset_manual_retry(&transfer_id).await?;
    manager.emit_receive_state(&app, &state, &task).await;
    manager
        .request_resume_grant(&app, &state, &transfer_id)
        .await?;
    manager
        .task(&transfer_id)
        .await
        .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))
}

pub async fn get_file_transfers() -> AppResult<Vec<FileTransferTask>> {
    Ok(manager().tasks().await)
}

async fn apply_completed_clipboard_file_sync(
    app: &AppHandle,
    state: &AppState,
    task: &FileTransferTask,
) -> AppResult<()> {
    let Some((content, message)) = apply_completed_clipboard_file_sync_core(
        state,
        task,
        |paths| clipboard::write_clipboard_files(app, paths),
    )
    .await?
    else {
        return Ok(());
    };

    let config = state.config().await;
    if config.save_history {
        let item = if let Some(item) = state
            .update_file_transfer_history(
                &task.transfer_id,
                FileTransferStatus::Completed,
                Some(content.clone()),
            )
            .await
        {
            item
        } else {
            let mut item = history::make_history_item(
                HistoryDirection::Remote,
                task.peer_device_name.clone(),
                &message,
            );
            item.file_transfer_id = Some(task.transfer_id.clone());
            item.file_transfer_status = Some(FileTransferStatus::Completed);
            state.push_history(item.clone()).await;
            item
        };
        history::save_history(app, &state.history().await)?;
        let _ = app.emit("clipboard-synced", history::history_item_for_frontend(&item));
    }
    Ok(())
}

async fn apply_completed_clipboard_file_sync_core<F>(
    state: &AppState,
    task: &FileTransferTask,
    write: F,
) -> AppResult<Option<(String, ClipboardMessage)>>
where
    F: FnOnce(&[PathBuf]) -> AppResult<()>,
{
    if !task.clipboard_sync
        || task.status != FileTransferStatus::Completed
        || task.files.is_empty()
        || task.files.iter().any(|file| {
            file.status != FileTransferFileStatus::Completed
                || file.transferred_bytes != file.size
        })
    {
        return Ok(None);
    }
    let paths = task
        .files
        .iter()
        .map(|file| {
            file.saved_path
                .as_deref()
                .map(PathBuf::from)
                .ok_or_else(|| {
                    AppError::InvalidInput("downloaded file paths are missing".to_string())
                })
        })
        .collect::<AppResult<Vec<_>>>()?;
    let content = clipboard::file_paths_to_clipboard_content(&paths)?;
    let now = Utc::now();
    let message = ClipboardMessage {
        message_id: task.transfer_id.clone(),
        source_device_id: task.peer_device_id.clone(),
        source_device_name: task.peer_device_name.clone(),
        content_type: ClipboardContentType::FileList,
        content_hash: clipboard_file_list_hash(&content),
        content: content.clone(),
        timestamp: now.timestamp(),
        origin_sequence: None,
        event_version: Some(crate::models::ClipboardEventVersion {
            physical_ms: now.timestamp_millis(),
            logical: 0,
            origin_device_id: task.peer_device_id.clone(),
        }),
    };
    if !state.should_apply_remote_clipboard(&message).await {
        return Ok(None);
    }
    write(&paths)?;
    state.mark_remote_clipboard_applied(&message).await;
    Ok(Some((content, message)))
}

async fn push_pending_clipboard_file_history(
    app: &AppHandle,
    state: &AppState,
    task: &FileTransferTask,
) -> AppResult<()> {
    let config = state.config().await;
    if !config.save_history {
        return Ok(());
    }

    let content = pending_clipboard_file_content(task)?;
    let message = ClipboardMessage {
        message_id: task.transfer_id.clone(),
        source_device_id: task.peer_device_id.clone(),
        source_device_name: task.peer_device_name.clone(),
        content_type: ClipboardContentType::FileList,
        content_hash: clipboard_file_list_hash(&content),
        content,
        timestamp: Utc::now().timestamp(),
        origin_sequence: None,
        event_version: None,
    };
    let mut item = history::make_history_item(
        HistoryDirection::Remote,
        task.peer_device_name.clone(),
        &message,
    );
    item.file_transfer_id = Some(task.transfer_id.clone());
    item.file_transfer_status = Some(task.status.clone());
    state.push_history(item.clone()).await;
    history::save_history(app, &state.history().await)?;
    let _ = app.emit("clipboard-synced", history::history_item_for_frontend(&item));
    Ok(())
}

fn pending_clipboard_file_content(task: &FileTransferTask) -> AppResult<String> {
    let entries = task
        .files
        .iter()
        .map(|file| clipboard::ClipboardFileEntry {
            path: String::new(),
            name: file.name.clone(),
            size: file.size,
            thumbnail: file.thumbnail.clone(),
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&entries).map_err(Into::into)
}

async fn update_clipboard_file_history_status(
    app: &AppHandle,
    state: &AppState,
    task: &FileTransferTask,
    content: Option<String>,
) -> AppResult<()> {
    if let Some(item) = state
        .update_file_transfer_history(&task.transfer_id, task.status.clone(), content)
        .await
    {
        history::save_history(app, &state.history().await)?;
        let _ = app.emit("clipboard-synced", history::history_item_for_frontend(&item));
    }
    Ok(())
}

pub async fn copy_clipboard_file_history_item(
    app: AppHandle,
    state: AppState,
    item: &HistoryItem,
) -> AppResult<CopyHistoryResult> {
    let transfer_id = item
        .file_transfer_id
        .as_deref()
        .ok_or_else(|| AppError::InvalidInput("file transfer id is missing".to_string()))?;
    let task = manager()
        .task(transfer_id)
        .await
        .ok_or_else(|| AppError::InvalidInput("file sync session expired".to_string()))?;

    match task.status {
        FileTransferStatus::Pending => {
            let task = manager()
                .accept_receive(app.clone(), state.clone(), transfer_id.to_string())
                .await?;
            update_clipboard_file_history_status(&app, &state, &task, None).await?;
            Ok(CopyHistoryResult::DownloadStarted)
        }
        FileTransferStatus::Accepted
        | FileTransferStatus::Transferring
        | FileTransferStatus::Retrying => {
            Ok(CopyHistoryResult::Downloading)
        }
        FileTransferStatus::WaitingForPeer | FileTransferStatus::Paused => {
            let manager = manager();
            manager
                .reset_oversized_partial_if_needed(transfer_id)
                .await?;
            let task = manager.reset_manual_retry(transfer_id).await?;
            manager.emit_receive_state(&app, &state, &task).await;
            manager
                .request_resume_grant(&app, &state, transfer_id)
                .await?;
            Ok(CopyHistoryResult::Downloading)
        }
        FileTransferStatus::Completed => {
            let paths = task
                .files
                .iter()
                .filter_map(|file| file.saved_path.as_deref())
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            if paths.is_empty() {
                return Err(AppError::InvalidInput(
                    "downloaded file paths are missing".to_string(),
                ));
            }
            clipboard::write_clipboard_files(&app, &paths)?;
            Ok(CopyHistoryResult::Copied)
        }
        FileTransferStatus::Failed
        | FileTransferStatus::Canceled
        | FileTransferStatus::Rejected => Err(AppError::InvalidInput(
            "file sync is not available anymore".to_string(),
        )),
    }
}

fn clipboard_file_list_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"fileList");
    hasher.update([0]);
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn open_transfer_folder(config: &AppConfig) -> AppResult<()> {
    let folder = transfer_save_dir(config)?;
    std::fs::create_dir_all(&folder)?;
    open_folder_with_system_file_manager(&folder)
}

pub fn source_file_path_from_history_item(item: &HistoryItem) -> AppResult<PathBuf> {
    if item.direction != HistoryDirection::Local
        || item.content_type != ClipboardContentType::FileList
    {
        return Err(AppError::InvalidInput(
            "history item is not a local file item".to_string(),
        ));
    }
    file_path_from_history_item(item)
}

pub fn file_path_from_history_item(item: &HistoryItem) -> AppResult<PathBuf> {
    if item.content_type != ClipboardContentType::FileList {
        return Err(AppError::InvalidInput(
            "history item is not a file item".to_string(),
        ));
    }

    let path = clipboard::clipboard_content_to_file_paths(&item.content)?
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| AppError::InvalidInput("source file path is missing".to_string()))?;
    Ok(path)
}

pub fn open_history_file_location(item: &HistoryItem) -> AppResult<()> {
    reveal_path_with_system_file_manager(&source_file_path_from_history_item(item)?)
}

fn maybe_open_folder_after_save(config: &AppConfig, save_dir: &Path) -> AppResult<()> {
    if !config.auto_open_folder_after_save {
        return Ok(());
    }

    open_folder_with_system_file_manager(save_dir)
}

pub fn current_transfer_save_dir(config: &AppConfig) -> AppResult<String> {
    Ok(transfer_save_dir(config)?.to_string_lossy().to_string())
}

pub async fn handle_file_offer(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    offer: IncomingFileOffer,
) {
    if !file_control_sender_is_trusted(state, connection_id, &offer.sender_device_id).await {
        return;
    }
    let config = state.config().await;
    if offer.clipboard_sync && !config.sync_files {
        return;
    }
    let clipboard_sync = offer.clipboard_sync;
    let transfer_id = offer.transfer_id.clone();
    let sender_device_id = offer.sender_device_id.clone();
    let manager = manager();
    match manager
        .handle_offer(app, offer, config.max_receive_file_size_mib)
        .await
    {
        Ok(task) => {
            if clipboard_sync {
                let _ = push_pending_clipboard_file_history(app, state, &task).await;
            }
        }
        Err(error) => {
            let reason = error.to_string();
            tracing::warn!(transfer_id, %reason, "file offer rejected");
            let _ = state
                .send_trusted_to_device(
                    &config,
                    &sender_device_id,
                    WireMessage::FileReject {
                        transfer_id,
                        receiver_device_id: state.status().await.device_id,
                        reason: Some(reason),
                    },
                )
                .await;
        }
    }
}

pub async fn handle_file_accept(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    receiver_device_id: String,
) {
    if !state
        .peer_connection_matches_device(connection_id, &receiver_device_id)
        .await
        || !file_control_sender_is_trusted(state, connection_id, &receiver_device_id).await
    {
        return;
    }
    let _ = manager().handle_accept(app, &transfer_id).await;
}

pub async fn handle_file_reject(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    receiver_device_id: String,
    reason: Option<String>,
) {
    if !file_resume_control_sender_is_trusted(state, connection_id, &receiver_device_id).await {
        return;
    }
    let _ = manager().handle_reject(app, &transfer_id, reason).await;
}

pub async fn handle_file_progress(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    device_id: String,
    file_id: String,
    file_transferred_bytes: u64,
) {
    if !file_control_sender_is_trusted(state, connection_id, &device_id).await {
        return;
    }
    let _ = manager()
        .handle_progress(app, &transfer_id, &file_id, file_transferred_bytes)
        .await;
}

pub async fn handle_file_complete(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    device_id: String,
    files: Vec<FileCompleteFile>,
) {
    if !file_control_sender_is_trusted(state, connection_id, &device_id).await {
        return;
    }
    let _ = manager().handle_complete(app, &transfer_id, &files).await;
}

pub async fn handle_file_cancel(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    device_id: String,
) {
    if !file_control_sender_is_trusted(state, connection_id, &device_id).await {
        return;
    }
    let _ = manager().handle_cancel(app, &transfer_id).await;
}

pub async fn handle_file_error(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    file_id: Option<String>,
    device_id: String,
    message: String,
) {
    if !file_control_sender_is_trusted(state, connection_id, &device_id).await {
        return;
    }
    let _ = manager()
        .handle_error(app, &transfer_id, file_id.as_deref(), &message)
        .await;
}

pub async fn handle_file_resume_request(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    receiver_device_id: String,
    files: Vec<FileResumeOffset>,
) {
    if !file_control_sender_is_trusted(state, connection_id, &receiver_device_id).await {
        return;
    }
    let manager = manager();
    if let Err(error) = manager
        .handle_resume_request(state, &transfer_id, &receiver_device_id, files)
        .await
    {
        tracing::warn!("file resume request rejected: {error}");
        let message = error.to_string();
        if message.contains("source file has changed") {
            if let Ok(task) = manager.mark_failed(&transfer_id, &message).await {
                emit_task_failed(app, &task);
            }
        }
        let _ = state
            .send_trusted_to_device(
                &state.config().await,
                &receiver_device_id,
                WireMessage::FileError {
                    transfer_id,
                    file_id: None,
                    device_id: state.status().await.device_id,
                    message,
                },
            )
            .await;
    }
}

pub async fn handle_file_resume_grant(
    app: &AppHandle,
    state: &AppState,
    connection_id: &str,
    transfer_id: String,
    sender_device_id: String,
    download_host: String,
    download_port: u16,
    files: Vec<FileResumeGrantFile>,
) {
    if !file_resume_control_sender_is_trusted(state, connection_id, &sender_device_id).await
        || !state
            .peer_supports(&sender_device_id, FILE_RESUME_CAPABILITY)
            .await
    {
        return;
    }
    let receiver_device_id = state.status().await.device_id;
    let manager = manager();
    match manager
        .apply_resume_grant(
            &transfer_id,
            &sender_device_id,
            download_host,
            download_port,
            &receiver_device_id,
            files,
        )
        .await
    {
        Ok(task) => {
            manager.emit_receive_state(app, state, &task).await;
            manager
                .start_receive_worker(app.clone(), state.clone(), transfer_id)
                .await;
        }
        Err(error) => tracing::warn!("file resume grant rejected: {error}"),
    }
}

pub async fn handle_file_peer_disconnected(
    app: &AppHandle,
    state: &AppState,
    peer_device_id: &str,
) {
    let manager = manager();
    match manager.mark_peer_disconnected(peer_device_id).await {
        Ok(tasks) => {
            for task in tasks {
                manager.emit_receive_state(app, state, &task).await;
            }
        }
        Err(error) => tracing::warn!("file transfer disconnect state failed: {error}"),
    }
}

pub async fn handle_file_peer_available(
    app: &AppHandle,
    state: &AppState,
    peer_device_id: &str,
) {
    let manager = manager();
    for task in manager
        .completed_receive_tasks_for_peer(peer_device_id)
        .await
    {
        let files = task
            .files
            .iter()
            .map(|file| FileCompleteFile {
                file_id: file.id.clone(),
                sha256: file.sha256.clone(),
            })
            .collect();
        let _ = state
            .send_trusted_to_device(
                &state.config().await,
                peer_device_id,
                WireMessage::FileComplete {
                    transfer_id: task.transfer_id,
                    device_id: state.status().await.device_id,
                    files,
                },
            )
            .await;
    }
    for transfer_id in manager.waiting_receive_transfer_ids(peer_device_id).await {
        if matches!(manager.next_receive_plan(&transfer_id).await, Ok(None)) {
            manager
                .clone()
                .start_receive_worker(app.clone(), state.clone(), transfer_id)
                .await;
            continue;
        }
        if let Err(error) = manager
            .request_resume_grant(app, state, &transfer_id)
            .await
        {
            tracing::warn!("file transfer reconnect resume failed: {error}");
        }
    }
}

async fn file_control_sender_is_trusted(
    state: &AppState,
    connection_id: &str,
    device_id: &str,
) -> bool {
    if !state
        .clipboard_sender_is_trusted(&state.config().await, connection_id, device_id)
        .await
    {
        return false;
    }
    trusted_transfer_peer(&state.devices().await, device_id).is_ok()
}

async fn file_resume_control_sender_is_trusted(
    state: &AppState,
    connection_id: &str,
    device_id: &str,
) -> bool {
    state
        .peer_connection_matches_device(connection_id, device_id)
        .await
        && file_control_sender_is_trusted(state, connection_id, device_id).await
}

async fn selected_files_from_paths(paths: Vec<PathBuf>) -> AppResult<Vec<SelectedTransferFile>> {
    selected_files_from_paths_with_limit(paths, None).await
}

async fn selected_files_from_paths_with_limit(
    paths: Vec<PathBuf>,
    configured_limit: Option<(u32, &'static str)>,
) -> AppResult<Vec<SelectedTransferFile>> {
    if paths.is_empty() {
        return Err(AppError::InvalidInput("select at least one file".to_string()));
    }
    if paths.len() > MAX_TRANSFER_FILE_COUNT {
        return Err(AppError::InvalidInput(format!(
            "cannot send more than {MAX_TRANSFER_FILE_COUNT} files"
        )));
    }

    let mut selected = Vec::with_capacity(paths.len());
    let mut sizes = Vec::with_capacity(paths.len());
    for path in paths {
        let metadata = fs::metadata(&path).await?;
        if !metadata.is_file() {
            return Err(AppError::InvalidInput("only regular files can be sent".to_string()));
        }
        sizes.push(metadata.len());
        validate_transfer_limits(sizes.len(), &sizes)?;
        if let Some((limit_mib, direction)) = configured_limit {
            validate_configured_single_file_limit(&sizes, limit_mib, direction)?;
        }
        let name = sanitize_file_name(
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default(),
        );
        let sha256 = sha256_file(&path).await?;
        selected.push(SelectedTransferFile {
            path: path.to_string_lossy().to_string(),
            name,
            size: metadata.len(),
            sha256,
            thumbnail: history::video_thumbnail_base64_for_path(&path, 240).ok(),
        });
    }
    validate_transfer_limits(selected.len(), &sizes)?;
    Ok(selected)
}

fn source_modified_ms(metadata: &std::fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}

async fn selected_file_from_path(path: PathBuf) -> AppResult<SelectedTransferFile> {
    selected_files_from_paths(vec![path])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| AppError::InvalidInput("select a file".to_string()))
}

fn validate_transfer_limits(file_count: usize, sizes: &[u64]) -> AppResult<()> {
    if file_count == 0 {
        return Err(AppError::InvalidInput("select at least one file".to_string()));
    }
    if file_count > MAX_TRANSFER_FILE_COUNT {
        return Err(AppError::InvalidInput(format!(
            "cannot send more than {MAX_TRANSFER_FILE_COUNT} files"
        )));
    }
    if sizes.iter().any(|size| *size > MAX_SINGLE_FILE_SIZE) {
        return Err(AppError::InvalidInput(
            "single file size cannot exceed 2 GiB".to_string(),
        ));
    }
    let total = sizes.iter().try_fold(0_u64, |total, size| {
        total.checked_add(*size).ok_or_else(|| {
            AppError::InvalidInput("transfer total size is too large".to_string())
        })
    })?;
    if total > MAX_TRANSFER_TOTAL_SIZE {
        return Err(AppError::InvalidInput(
            "transfer total size cannot exceed 5 GiB".to_string(),
        ));
    }
    Ok(())
}

fn validate_configured_single_file_limit(
    sizes: &[u64],
    limit_mib: u32,
    direction: &str,
) -> AppResult<()> {
    const MIB: u64 = 1024 * 1024;
    let limit_bytes = u64::from(limit_mib).saturating_mul(MIB);
    if sizes.iter().any(|size| *size > limit_bytes) {
        return Err(AppError::InvalidInput(format!(
            "single file size exceeds configured {direction} limit of {limit_mib} MiB"
        )));
    }
    Ok(())
}

fn should_persist_progress(
    last_bytes: u64,
    last_at: Instant,
    transferred_bytes: u64,
    now: Instant,
) -> bool {
    transferred_bytes.saturating_sub(last_bytes) >= PERSIST_CHECKPOINT_BYTES
        || now.duration_since(last_at) >= PERSIST_CHECKPOINT_INTERVAL
}

fn should_emit_progress(last_at: Option<Instant>, now: Instant) -> bool {
    last_at
        .map(|last_at| now.duration_since(last_at) >= PROGRESS_EVENT_INTERVAL)
        .unwrap_or(true)
}

fn transfer_snapshot(entry: &ManagedTransfer) -> file_transfer_store::TransferSnapshot {
    let (download_host, download_port) = if entry.task.direction == FileTransferDirection::Send {
        (entry.download_host.clone(), entry.download_port)
    } else {
        (None, None)
    };
    let mut files = entry
        .files
        .iter()
        .map(|(file_id, file)| file_transfer_store::PersistedTransferFile {
            file_id: file_id.clone(),
            source_path: file.source_path.clone(),
            source_size: file.source_size,
            source_modified_ms: file.source_modified_ms,
            temp_path: file.temp_path.clone(),
            final_path: file.final_path.clone(),
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.file_id.cmp(&right.file_id));
    file_transfer_store::TransferSnapshot {
        version: file_transfer_store::TRANSFER_SNAPSHOT_VERSION,
        task: entry.task.clone(),
        files,
        download_host,
        download_port,
        retry_count: entry.retry_count,
        last_activity_at: entry.last_activity_at,
    }
}

fn managed_transfer_from_snapshot(
    snapshot: file_transfer_store::TransferSnapshot,
) -> ManagedTransfer {
    let task = snapshot.task;
    let restore_download_endpoint = task.direction == FileTransferDirection::Send;
    let last_persisted_bytes = task.transferred_bytes;
    let files = snapshot
        .files
        .into_iter()
        .map(|file| {
            (
                file.file_id,
                ManagedTransferFile {
                    source_path: file.source_path,
                    source_size: file.source_size,
                    source_modified_ms: file.source_modified_ms,
                    token: None,
                    token_consumed: false,
                    token_offset: 0,
                    token_expires_at: None,
                    token_receiver_device_id: None,
                    temp_path: file.temp_path,
                    final_path: file.final_path,
                },
            )
        })
        .collect();
    ManagedTransfer {
        task,
        files,
        download_host: restore_download_endpoint
            .then_some(snapshot.download_host)
            .flatten(),
        download_port: restore_download_endpoint
            .then_some(snapshot.download_port)
            .flatten(),
        retry_count: snapshot.retry_count,
        last_activity_at: snapshot.last_activity_at,
        last_persisted_bytes,
        last_persisted_at: Instant::now(),
        last_emitted_at: None,
    }
}

fn prune_terminal_transfer_tasks(tasks: &mut HashMap<String, ManagedTransfer>) {
    let terminal_count = tasks
        .values()
        .filter(|entry| is_terminal_transfer_status(&entry.task.status))
        .count();
    if terminal_count <= MAX_RETAINED_TRANSFER_TASKS {
        return;
    }

    let mut removable = tasks
        .iter()
        .filter(|(_, entry)| is_terminal_transfer_status(&entry.task.status))
        .map(|(transfer_id, entry)| {
            (
                transfer_id.clone(),
                entry.task.completed_at.unwrap_or(entry.task.created_at),
            )
        })
        .collect::<Vec<_>>();
    removable.sort_by_key(|(_, completed_at)| *completed_at);

    for (transfer_id, _) in removable
        .into_iter()
        .take(terminal_count - MAX_RETAINED_TRANSFER_TASKS)
    {
        tasks.remove(&transfer_id);
    }
}

fn is_terminal_transfer_status(status: &FileTransferStatus) -> bool {
    matches!(
        status,
        FileTransferStatus::Completed
            | FileTransferStatus::Failed
            | FileTransferStatus::Canceled
            | FileTransferStatus::Rejected
    )
}

async fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

async fn hash_partial_file(path: &Path, expected_size: u64) -> AppResult<(u64, Sha256)> {
    if !path.exists() {
        return Ok((0, Sha256::new()));
    }
    let metadata = fs::metadata(path).await?;
    if metadata.len() > expected_size {
        return Err(AppError::InvalidInput(
            "partial file is larger than expected".to_string(),
        ));
    }
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];
    loop {
        let read = file.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok((metadata.len(), hasher))
}

async fn receive_http_to_part<F, Fut>(
    host: &str,
    port: u16,
    request_target: &str,
    offset: u64,
    force_range: bool,
    expected_size: u64,
    expected_sha256: &str,
    temp_path: &Path,
    mut hasher: Sha256,
    mut on_progress: F,
) -> AppResult<String>
where
    F: FnMut(u64) -> Fut,
    Fut: Future<Output = AppResult<()>>,
{
    let mut stream = TcpStream::connect((host, port)).await.map_err(|error| {
        AppError::ConnectionTimeout(format!("file transfer connection failed: {error}"))
    })?;
    let range = if offset > 0 || force_range {
        format!("Range: bytes={offset}-\r\n")
    } else {
        String::new()
    };
    let request = format!(
        "GET {request_target} HTTP/1.1\r\nHost: {host}:{port}\r\n{range}Connection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.map_err(|error| {
        AppError::ConnectionTimeout(format!("file transfer request failed: {error}"))
    })?;

    let mut head = file_transfer_http::read_head(&mut stream)
        .await
        .map_err(|error| {
            AppError::ConnectionTimeout(format!(
                "file transfer response header failed: {error}"
            ))
        })?;
    let response_status = http_status_code(&head.first_line)
        .ok_or_else(|| AppError::InvalidInput("invalid HTTP download response".to_string()))?;
    if matches!(response_status, 401 | 403) {
        return Err(AppError::InvalidInput(
            "resume authorization expired".to_string(),
        ));
    }
    if (500..=599).contains(&response_status) {
        return Err(AppError::ConnectionTimeout(format!(
            "file transfer server returned HTTP {response_status}"
        )));
    }
    if offset > 0 && response_status == 200 {
        return Err(AppError::InvalidInput(
            "restart download from zero".to_string(),
        ));
    }
    file_transfer_http::validate_download_head(
        &head,
        expected_size,
        offset,
        expected_sha256,
    )
    .map_err(|error| AppError::InvalidInput(error.to_string()))?;

    let mut output = if offset == 0 {
        File::create(temp_path).await?
    } else {
        OpenOptions::new().append(true).open(temp_path).await?
    };
    let mut transferred = offset;
    let body = std::mem::take(&mut head.body_prefix);
    if !body.is_empty() {
        if transferred.saturating_add(body.len() as u64) > expected_size {
            return Err(AppError::InvalidInput("file size mismatch".to_string()));
        }
        output.write_all(&body).await?;
        hasher.update(&body);
        transferred += body.len() as u64;
        on_progress(transferred).await?;
    }

    let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];
    loop {
        let read = stream.read(&mut buffer).await.map_err(|error| {
            AppError::ConnectionTimeout(format!("file transfer response failed: {error}"))
        })?;
        if read == 0 {
            break;
        }
        if transferred.saturating_add(read as u64) > expected_size {
            return Err(AppError::InvalidInput("file size mismatch".to_string()));
        }
        output.write_all(&buffer[..read]).await?;
        hasher.update(&buffer[..read]);
        transferred += read as u64;
        on_progress(transferred).await?;
    }
    output.flush().await?;

    if transferred != expected_size {
        return Err(AppError::ConnectionTimeout(
            "file transfer ended before the expected size".to_string(),
        ));
    }

    let actual_hash = format!("{:x}", hasher.finalize());
    if actual_hash != expected_sha256 {
        let _ = fs::remove_file(temp_path).await;
        return Err(AppError::InvalidInput("file sha256 mismatch".to_string()));
    }
    Ok(actual_hash)
}

fn trusted_transfer_peer(devices: &[DeviceInfo], device_id: &str) -> AppResult<DeviceInfo> {
    let device = devices
        .iter()
        .find(|device| transfer_peer_matches_key(device, device_id))
        .cloned()
        .ok_or_else(|| AppError::UnknownDevice(device_id.to_string()))?;
    if device.connected && device.trusted && device.remote_trusted {
        Ok(device)
    } else {
        Err(AppError::InvalidInput(
            "file transfer requires a connected mutually trusted device".to_string(),
        ))
    }
}

fn transfer_peer_matches_key(device: &DeviceInfo, key: &str) -> bool {
    if device.id == key {
        return true;
    }

    let Ok(device_endpoint) = network::endpoint_from_connection_id(&device.ip, device.port) else {
        return false;
    };
    let Ok(key_endpoint) = network::normalize_peer_endpoint(key, device.port) else {
        return false;
    };

    device_endpoint == key_endpoint
}

fn advertised_download_endpoint(
    local_ip: Option<String>,
    port: u16,
    peer_hint: Option<IpAddr>,
) -> AppResult<FileDownloadServer> {
    let advertised_host = local_ip
        .filter(|host| !host.trim().is_empty())
        .or_else(|| network::preferred_local_ip_for_peer(peer_hint).map(|ip| ip.to_string()))
        .ok_or_else(|| {
            AppError::InvalidInput("cannot determine local LAN address for file transfer".to_string())
        })?;

    Ok(FileDownloadServer {
        advertised_host,
        port,
    })
}

fn new_transfer_file(
    id: String,
    name: String,
    size: u64,
    sha256: String,
    thumbnail: Option<String>,
) -> FileTransferFile {
    FileTransferFile {
        id,
        name,
        size,
        sha256,
        thumbnail,
        saved_path: None,
        transferred_bytes: 0,
        status: FileTransferFileStatus::Pending,
        error: None,
    }
}

fn sanitize_file_name(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let base = normalized
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim();
    let mut cleaned = base
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(character, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
        .collect::<String>();
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", ".");
    }
    let cleaned = cleaned.trim_matches(|character| character == '.' || character == ' ');
    if cleaned.is_empty() {
        "download".to_string()
    } else {
        cleaned.to_string()
    }
}

fn default_save_dir() -> AppResult<PathBuf> {
    let downloads = dirs::download_dir()
        .ok_or_else(|| AppError::InvalidInput("cannot locate downloads folder".to_string()))?;
    Ok(downloads.join("Copy-Sharer"))
}

fn transfer_save_dir(config: &AppConfig) -> AppResult<PathBuf> {
    config
        .file_save_dir
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(default_save_dir)
}

#[cfg(test)]
fn unique_save_path(save_dir: &Path, file_name: &str) -> PathBuf {
    unique_save_path_with_reserved(save_dir, file_name, &HashSet::new())
}

fn unique_save_path_with_reserved(
    save_dir: &Path,
    file_name: &str,
    reserved_paths: &HashSet<PathBuf>,
) -> PathBuf {
    let sanitized = sanitize_file_name(file_name);
    let candidate = save_dir.join(&sanitized);
    if !candidate.exists()
        && !part_path_for(&candidate).exists()
        && !reserved_paths.contains(&candidate)
    {
        return candidate;
    }

    let path = Path::new(&sanitized);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1.. {
        let name = match extension {
            Some(extension) if !extension.is_empty() => {
                format!("{stem} ({index}).{extension}")
            }
            _ => format!("{stem} ({index})"),
        };
        let candidate = save_dir.join(name);
        if !candidate.exists()
            && !part_path_for(&candidate).exists()
            && !reserved_paths.contains(&candidate)
        {
            return candidate;
        }
    }
    unreachable!()
}

fn part_path_for(final_path: &Path) -> PathBuf {
    final_path.with_extension(format!(
        "{}part",
        final_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!("{value}."))
            .unwrap_or_default()
    ))
}

fn download_request_target(
    transfer_id: &str,
    file_id: &str,
    token: &str,
    receiver_device_id: Option<&str>,
) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer
        .append_pair("transfer_id", transfer_id)
        .append_pair("file_id", file_id)
        .append_pair("token", token);
    if let Some(receiver_device_id) = receiver_device_id {
        serializer.append_pair("receiver_device_id", receiver_device_id);
    }
    format!("/file-transfer?{}", serializer.finish())
}

fn parse_download_query(target: &str) -> Option<(String, String, String, Option<String>)> {
    let url = url::Url::parse(&format!("http://copyshare{target}")).ok()?;
    if url.path() != "/file-transfer" {
        return None;
    }
    let mut transfer_id = None;
    let mut file_id = None;
    let mut token = None;
    let mut receiver_device_id = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "transfer_id" => transfer_id = Some(value.to_string()),
            "file_id" => file_id = Some(value.to_string()),
            "token" => token = Some(value.to_string()),
            "receiver_device_id" => receiver_device_id = Some(value.to_string()),
            _ => {}
        }
    }
    Some((transfer_id?, file_id?, token?, receiver_device_id))
}

fn request_target(first_line: &str) -> Option<String> {
    let mut parts = first_line.split_whitespace();
    match (parts.next(), parts.next(), parts.next()) {
        (Some("GET"), Some(target), Some(version)) if version.starts_with("HTTP/") => {
            Some(target.to_string())
        }
        _ => None,
    }
}

fn http_status_code(first_line: &str) -> Option<u16> {
    let mut parts = first_line.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some(version), Some(status)) if version.starts_with("HTTP/") => status.parse().ok(),
        _ => None,
    }
}

async fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    body: &[u8],
) -> AppResult<()> {
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    Ok(())
}

fn emit_progress(app: &AppHandle, task: &FileTransferTask, file_id: &str) {
    if let Some(file) = task.files.iter().find(|file| file.id == file_id) {
        let _ = app.emit(
            "file-transfer-progress",
            FileTransferProgressEvent {
                transfer_id: task.transfer_id.clone(),
                file_id: file.id.clone(),
                file_transferred_bytes: file.transferred_bytes,
                file_size: file.size,
                total_transferred_bytes: task.transferred_bytes,
                total_size: task.total_size,
                status: task.status.clone(),
            },
        );
    }
}

fn emit_task_updated(app: &AppHandle, task: &FileTransferTask) {
    let _ = app.emit("file-transfer-updated", task.clone());
}

fn emit_task_completed(app: &AppHandle, task: &FileTransferTask) {
    notifications::notify_file_transfer_completed(app, task);
    let _ = app.emit("file-transfer-completed", task.clone());
    emit_task_updated(app, task);
}

fn emit_task_failed(app: &AppHandle, task: &FileTransferTask) {
    notifications::notify_file_transfer_failed(app, task);
    let _ = app.emit("file-transfer-failed", task.clone());
    emit_task_updated(app, task);
}

#[cfg(target_os = "windows")]
fn open_folder_with_system_file_manager(path: &Path) -> AppResult<()> {
    std::process::Command::new("explorer").arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn reveal_path_with_system_file_manager(path: &Path) -> AppResult<()> {
    std::process::Command::new("explorer")
        .arg("/select,")
        .arg(path)
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_folder_with_system_file_manager(path: &Path) -> AppResult<()> {
    std::process::Command::new("open").arg(path).spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn reveal_path_with_system_file_manager(path: &Path) -> AppResult<()> {
    std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_folder_with_system_file_manager(path: &Path) -> AppResult<()> {
    std::process::Command::new("xdg-open").arg(path).spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn reveal_path_with_system_file_manager(path: &Path) -> AppResult<()> {
    let folder = path.parent().ok_or_else(|| {
        AppError::InvalidInput("source file parent folder is missing".to_string())
    })?;
    std::process::Command::new("xdg-open").arg(folder).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
        time::Duration,
    };

    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc;
    use uuid::Uuid;

    use crate::{
        file_transfer_http, file_transfer_store,
        models::{
            ClipboardContentType, DeviceInfo, DeviceStatus, FileResumeGrantFile, FileResumeOffset,
            FileTransferFileStatus, FileTransferStatus, HistoryDirection, HistoryItem, SyncStatus,
            WireMessage, FILE_RESUME_CAPABILITY,
        },
        state::AppState,
    };

    use super::{
        advertised_download_endpoint, apply_completed_clipboard_file_sync_core,
        current_transfer_save_dir, download_request_target, file_resume_control_sender_is_trusted,
        hash_partial_file, new_transfer_file, part_path_for, pending_clipboard_file_content,
        receive_http_to_part, sanitize_file_name,
        source_file_path_from_history_item, transfer_save_dir, trusted_transfer_peer,
        transfer_snapshot, unique_save_path, validate_configured_single_file_limit,
        validate_transfer_limits, DownloadClaimError, FileTransferManager, ManagedTransferFile,
        MAX_SINGLE_FILE_SIZE, MAX_TRANSFER_FILE_COUNT, MAX_TRANSFER_TOTAL_SIZE,
    };

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "copyshare-file-transfer-test-{}-{name}",
            Uuid::new_v4()
        ))
    }

    fn trusted_device() -> DeviceInfo {
        DeviceInfo {
            id: "device-b".to_string(),
            name: "Laptop B".to_string(),
            ip: "10.0.0.2".to_string(),
            port: 8765,
            connected: true,
            trusted: true,
            remote_trusted: true,
            has_connected_before: true,
            last_seen_at: Some(Utc::now()),
            status: DeviceStatus::Online,
        }
    }

    fn local_file_history(content: String) -> HistoryItem {
        let content_hash = crate::sync::content_hash(&ClipboardContentType::FileList, &content);
        HistoryItem {
            id: "local-file".to_string(),
            direction: HistoryDirection::Local,
            source_device: "CopyShare".to_string(),
            summary: "file.txt 5 B".to_string(),
            content,
            content_hash,
            content_type: ClipboardContentType::FileList,
            sync_status: SyncStatus::Synced,
            file_transfer_id: None,
            file_transfer_status: None,
            is_pinned: false,
            pinned_at: None,
            success: true,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn sender_history_resolves_first_existing_source_file() {
        let first = temp_path("first-source.txt");
        let second = temp_path("second-source.txt");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();
        let content = crate::clipboard::file_paths_to_clipboard_content(&[
            first.clone(),
            second.clone(),
        ])
        .unwrap();

        let resolved = source_file_path_from_history_item(&local_file_history(content)).unwrap();

        assert_eq!(resolved, first);
        let _ = fs::remove_file(first);
        let _ = fs::remove_file(second);
    }

    #[test]
    fn sender_history_rejects_missing_source_file() {
        let missing = temp_path("missing-source.txt");
        let content = serde_json::json!([{
            "path": missing.to_string_lossy(),
            "name": "missing-source.txt",
            "size": 0
        }])
        .to_string();

        assert!(source_file_path_from_history_item(&local_file_history(content)).is_err());
    }

    #[test]
    fn sanitize_file_name_blocks_path_traversal() {
        assert_eq!(sanitize_file_name("../evil.txt"), "evil.txt");
        assert_eq!(sanitize_file_name("C:\\temp\\secret.txt"), "secret.txt");
        assert_eq!(sanitize_file_name("/tmp/report.pdf"), "report.pdf");
        assert_eq!(sanitize_file_name(""), "download");
        assert!(!sanitize_file_name("..\\..\\evil.txt").contains(".."));
    }

    #[tokio::test]
    async fn token_validation_fails_for_wrong_token() {
        let manager = FileTransferManager::new();
        let source = temp_path("source.txt");
        fs::write(&source, b"hello").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "hello.txt", "good-token")
            .await;

        let result = manager
            .claim_download_for_test(&transfer_id, "file-1", "bad-token")
            .await;

        assert!(matches!(result, Err(DownloadClaimError::Forbidden)));
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn token_validation_consumes_successful_token_once() {
        let manager = FileTransferManager::new();
        let source = temp_path("source.txt");
        fs::write(&source, b"hello").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "hello.txt", "one-use-token")
            .await;

        assert!(manager
            .claim_download_for_test(&transfer_id, "file-1", "one-use-token")
            .await
            .is_ok());
        assert!(matches!(
            manager
                .claim_download_for_test(&transfer_id, "file-1", "one-use-token")
                .await,
            Err(DownloadClaimError::Forbidden)
        ));
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn initial_download_token_rejects_a_changed_source_file() {
        let manager = FileTransferManager::new();
        let source = temp_path("changed-initial-source.txt");
        fs::write(&source, b"hello").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "hello.txt", "token")
            .await;
        fs::write(&source, b"changed-size").unwrap();

        let claim = manager
            .claim_download_for_test(&transfer_id, "file-1", "token")
            .await;

        assert_eq!(claim.unwrap_err(), DownloadClaimError::Forbidden);
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn resume_token_is_bound_to_offset_receiver_expiry_and_single_use() {
        let manager = FileTransferManager::new();
        let source = temp_path("resume-token-source.txt");
        fs::write(&source, b"resume-token").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "source.txt", "initial")
            .await;
        let token = manager
            .issue_resume_token_for_test(
                &transfer_id,
                "file-1",
                "device-b",
                3,
                Utc::now() + chrono::Duration::seconds(60),
            )
            .await
            .unwrap();

        assert!(manager
            .claim_resume_download_for_test(&transfer_id, "file-1", &token, 3, Some("device-b"))
            .await
            .is_ok());
        assert!(matches!(
            manager
                .claim_resume_download_for_test(&transfer_id, "file-1", &token, 3, Some("device-b"))
                .await,
            Err(DownloadClaimError::Forbidden)
        ));

        let wrong_offset = manager
            .issue_resume_token_for_test(
                &transfer_id,
                "file-1",
                "device-b",
                4,
                Utc::now() + chrono::Duration::seconds(60),
            )
            .await
            .unwrap();
        assert!(matches!(
            manager
                .claim_resume_download_for_test(
                    &transfer_id,
                    "file-1",
                    &wrong_offset,
                    3,
                    Some("device-b")
                )
                .await,
            Err(DownloadClaimError::Forbidden)
        ));

        let expired = manager
            .issue_resume_token_for_test(
                &transfer_id,
                "file-1",
                "device-b",
                3,
                Utc::now() - chrono::Duration::seconds(1),
            )
            .await
            .unwrap();
        assert!(matches!(
            manager
                .claim_resume_download_for_test(
                    &transfer_id,
                    "file-1",
                    &expired,
                    3,
                    Some("device-b")
                )
                .await,
            Err(DownloadClaimError::Forbidden)
        ));
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn resume_grant_rejects_a_changed_source_file() {
        let manager = FileTransferManager::new();
        let source = temp_path("resume-grant-source.txt");
        fs::write(&source, b"original").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "source.txt", "initial")
            .await;

        let grants = manager
            .create_resume_grants_for_test(
                &transfer_id,
                "device-b",
                vec![crate::models::FileResumeOffset {
                    file_id: "file-1".to_string(),
                    offset: 4,
                }],
            )
            .await
            .unwrap();
        assert_eq!(grants[0].file_id, "file-1");
        assert_eq!(grants[0].offset, 4);

        fs::write(&source, b"changed-size").unwrap();
        assert!(manager
            .create_resume_grants_for_test(
                &transfer_id,
                "device-b",
                vec![crate::models::FileResumeOffset {
                    file_id: "file-1".to_string(),
                    offset: 4,
                }],
            )
            .await
            .is_err());
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn resume_request_sends_a_fresh_offset_bound_grant() {
        let manager = FileTransferManager::new();
        let source = temp_path("resume-request-source.txt");
        fs::write(&source, b"hello world").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "hello.txt", "initial")
            .await;
        {
            let mut tasks = manager.tasks.lock().await;
            let transfer = tasks.get_mut(&transfer_id).unwrap();
            transfer.download_host = Some("192.168.1.10".to_string());
            transfer.download_port = Some(8765);
        }

        let state = AppState::new();
        let mut config = state.config().await;
        config.trusted_devices.push("device-b".to_string());
        state.set_config(config).await;
        state.set_local_ip(Some("192.168.1.10".to_string())).await;
        state.replace_devices(vec![trusted_device()]).await;
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(std::future::pending());
        state
            .register_peer("connection-b".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "connection-b",
                "device-b".to_string(),
                Some("ws://192.168.1.20:8765/".to_string()),
            )
            .await;
        state
            .set_peer_capabilities(
                "connection-b",
                vec![FILE_RESUME_CAPABILITY.to_string()],
            )
            .await;

        manager
            .handle_resume_request(
                &state,
                &transfer_id,
                "device-b",
                vec![FileResumeOffset {
                    file_id: "file-1".to_string(),
                    offset: 6,
                }],
            )
            .await
            .unwrap();

        match receiver.recv().await.unwrap() {
            WireMessage::FileResumeGrant {
                transfer_id: granted_transfer_id,
                sender_device_id,
                download_host,
                download_port,
                files,
            } => {
                assert_eq!(granted_transfer_id, transfer_id);
                assert_eq!(sender_device_id, state.status().await.device_id);
                assert_eq!(download_host, "192.168.1.10");
                assert_eq!(download_port, 8765);
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].file_id, "file-1");
                assert_eq!(files[0].offset, 6);
                assert_ne!(files[0].token, "initial");
            }
            message => panic!("unexpected message: {message:?}"),
        }
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn sender_restart_refreshes_the_resume_grant_download_endpoint() {
        let root = temp_path("sender-restart-store");
        let source = root.join("source.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"hello world").unwrap();
        let original = FileTransferManager::new();
        original.set_store_root_for_test(root.clone()).await;
        let transfer_id = original
            .insert_test_send_task("device-b", source, "hello.txt", "stale-token")
            .await;
        {
            let mut tasks = original.tasks.lock().await;
            let transfer = tasks.get_mut(&transfer_id).unwrap();
            transfer.download_host = Some("192.168.1.99".to_string());
            transfer.download_port = Some(9999);
        }
        original
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        let restored = FileTransferManager::new();
        restored.set_store_root_for_test(root.clone()).await;
        restored.restore_from_store_for_test().await.unwrap();
        assert!(!restored.has_download_token_for_test(&transfer_id, "file-1").await);

        let state = AppState::new();
        let mut config = state.config().await;
        config.trusted_devices.push("device-b".to_string());
        state.set_config(config).await;
        state.set_local_ip(Some("192.168.1.10".to_string())).await;
        state.replace_devices(vec![trusted_device()]).await;
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(std::future::pending());
        state
            .register_peer("connection-b".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("connection-b", "device-b".to_string(), None)
            .await;
        state
            .set_peer_capabilities(
                "connection-b",
                vec![FILE_RESUME_CAPABILITY.to_string()],
            )
            .await;

        restored
            .handle_resume_request(
                &state,
                &transfer_id,
                "device-b",
                vec![FileResumeOffset {
                    file_id: "file-1".to_string(),
                    offset: 6,
                }],
            )
            .await
            .unwrap();

        match receiver.recv().await.unwrap() {
            WireMessage::FileResumeGrant {
                download_host,
                download_port,
                ..
            } => {
                assert_eq!(download_host, "192.168.1.10");
                assert_eq!(download_port, state.config().await.port);
            }
            message => panic!("unexpected message: {message:?}"),
        }
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn both_restarts_resume_the_same_transfer_over_http_range() {
        let sender_root = temp_path("both-restart-sender");
        let receiver_root = temp_path("both-restart-receiver");
        let source = sender_root.join("source.txt");
        let part = receiver_root.join("received.txt.part");
        let final_path = receiver_root.join("received.txt");
        let content = b"hello world";
        let expected_hash = format!("{:x}", Sha256::digest(content));
        fs::create_dir_all(&sender_root).unwrap();
        fs::create_dir_all(&receiver_root).unwrap();
        fs::write(&source, content).unwrap();
        fs::write(&part, b"hello ").unwrap();

        let original_sender = FileTransferManager::new();
        original_sender
            .set_store_root_for_test(sender_root.clone())
            .await;
        let transfer_id = original_sender
            .insert_test_send_task(
                "receiver-device",
                source,
                "received.txt",
                "stale-token",
            )
            .await;
        {
            let mut tasks = original_sender.tasks.lock().await;
            tasks.get_mut(&transfer_id).unwrap().task.files[0].sha256 =
                expected_hash.clone();
        }
        original_sender
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        let original_receiver = FileTransferManager::new();
        original_receiver
            .set_store_root_for_test(receiver_root.clone())
            .await;
        let temporary_receiver_id = original_receiver
            .insert_test_receive_task(
                "sender-device",
                "received.txt",
                content.len() as u64,
                &expected_hash,
                part.clone(),
            )
            .await;
        {
            let mut tasks = original_receiver.tasks.lock().await;
            let mut transfer = tasks.remove(&temporary_receiver_id).unwrap();
            transfer.task.transfer_id = transfer_id.clone();
            tasks.insert(transfer_id.clone(), transfer);
        }
        original_receiver
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        let restored_sender = FileTransferManager::new();
        restored_sender
            .set_store_root_for_test(sender_root.clone())
            .await;
        restored_sender.restore_from_store_for_test().await.unwrap();
        let restored_receiver = FileTransferManager::new();
        restored_receiver
            .set_store_root_for_test(receiver_root.clone())
            .await;
        restored_receiver
            .restore_from_store_for_test()
            .await
            .unwrap();

        assert_eq!(
            restored_receiver.task(&transfer_id).await.unwrap().transferred_bytes,
            6
        );
        assert!(!restored_sender
            .has_download_token_for_test(&transfer_id, "file-1")
            .await);
        assert!(!restored_receiver
            .has_download_token_for_test(&transfer_id, "file-1")
            .await);

        let grant = restored_sender
            .create_resume_grants_for_test(
                &transfer_id,
                "receiver-device",
                vec![FileResumeOffset {
                    file_id: "file-1".to_string(),
                    offset: 6,
                }],
            )
            .await
            .unwrap()
            .remove(0);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        restored_receiver
            .apply_resume_grant(
                &transfer_id,
                "sender-device",
                "127.0.0.1".to_string(),
                address.port(),
                "receiver-device",
                vec![grant],
            )
            .await
            .unwrap();
        let (_, _, _, plan) = restored_receiver
            .next_receive_plan(&transfer_id)
            .await
            .unwrap()
            .unwrap();

        let server_manager = restored_sender.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            server_manager
                .serve_download_connection_inner(None, stream)
                .await
                .unwrap();
        });
        let (offset, hasher) = hash_partial_file(&part, content.len() as u64).await.unwrap();
        let target = download_request_target(
            &transfer_id,
            &plan.file.id,
            plan.token.as_deref().unwrap(),
            plan.receiver_device_id.as_deref(),
        );
        let receiver_for_progress = restored_receiver.clone();
        let transfer_for_progress = transfer_id.clone();
        let file_for_progress = plan.file.id.clone();
        let actual_hash = receive_http_to_part(
            "127.0.0.1",
            address.port(),
            &target,
            offset,
            true,
            plan.file.size,
            &plan.file.sha256,
            &part,
            hasher,
            move |transferred| {
                let receiver = receiver_for_progress.clone();
                let transfer_id = transfer_for_progress.clone();
                let file_id = file_for_progress.clone();
                async move {
                    receiver
                        .update_progress_with_optional_events(
                            None,
                            &transfer_id,
                            &file_id,
                            transferred,
                            false,
                        )
                        .await
                        .map(|_| ())
                }
            },
        )
        .await
        .unwrap();
        server.await.unwrap();

        assert_eq!(actual_hash, expected_hash);
        assert_eq!(fs::read(&part).unwrap(), content);
        assert_eq!(
            restored_sender.task(&transfer_id).await.unwrap().transferred_bytes,
            content.len() as u64
        );
        assert_eq!(
            restored_receiver.task(&transfer_id).await.unwrap().transferred_bytes,
            content.len() as u64
        );
        let completed = restored_receiver
            .finish_receive_for_test(
                &transfer_id,
                part.clone(),
                final_path.clone(),
                expected_hash,
            )
            .await
            .unwrap();
        assert_eq!(completed.status, FileTransferStatus::Completed);
        assert_eq!(fs::read(&final_path).unwrap(), content);
        assert!(file_transfer_store::load_all(&receiver_root)
            .unwrap()
            .is_empty());
        let _ = fs::remove_dir_all(sender_root);
        let _ = fs::remove_dir_all(receiver_root);
    }

    #[tokio::test]
    async fn resume_control_requires_the_connection_to_match_the_claimed_device() {
        let state = AppState::new();
        let mut config = state.config().await;
        config.trusted_devices.push("device-b".to_string());
        state.set_config(config).await;
        state.replace_devices(vec![trusted_device()]).await;

        let (matching_sender, _matching_receiver) = mpsc::unbounded_channel();
        let matching_join = tauri::async_runtime::spawn(std::future::pending());
        state
            .register_peer("connection-b".to_string(), matching_sender, matching_join)
            .await;
        state
            .attach_peer_device("connection-b", "device-b".to_string(), None)
            .await;

        let (other_sender, _other_receiver) = mpsc::unbounded_channel();
        let other_join = tauri::async_runtime::spawn(std::future::pending());
        state
            .register_peer("connection-other".to_string(), other_sender, other_join)
            .await;
        state
            .attach_peer_device("connection-other", "device-other".to_string(), None)
            .await;

        assert!(file_resume_control_sender_is_trusted(
            &state,
            "connection-b",
            "device-b"
        )
        .await);
        assert!(!file_resume_control_sender_is_trusted(
            &state,
            "connection-other",
            "device-b"
        )
        .await);
    }

    #[tokio::test]
    async fn range_download_serves_only_the_requested_suffix_and_persists_progress() {
        let root = temp_path("range-download-store");
        let source = root.join("source.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"hello world").unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_send_task("device-b", source, "hello.txt", "initial")
            .await;
        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();
        let token = manager
            .issue_resume_token_for_test(
                &transfer_id,
                "file-1",
                "device-b",
                6,
                Utc::now() + chrono::Duration::seconds(60),
            )
            .await
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_manager = manager.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            server_manager
                .serve_download_connection_inner(None, stream)
                .await
                .unwrap();
        });

        let mut client = TcpStream::connect(address).await.unwrap();
        let target = download_request_target(
            &transfer_id,
            "file-1",
            &token,
            Some("device-b"),
        );
        client
            .write_all(
                format!(
                    "GET {target} HTTP/1.1\r\nHost: {address}\r\nRange: bytes=6-\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut response = Vec::new();
        client.read_to_end(&mut response).await.unwrap();
        server.await.unwrap();

        let header_end = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let header = String::from_utf8_lossy(&response[..header_end]);
        assert!(header.starts_with("HTTP/1.1 206 Partial Content\r\n"));
        assert!(header.contains("Content-Length: 5\r\n"));
        assert!(header.contains("Content-Range: bytes 6-10/11\r\n"));
        assert!(header.contains("ETag: \"sha256:hash\"\r\n"));
        assert_eq!(&response[header_end + 4..], b"world");
        let snapshots = file_transfer_store::load_all(&root).unwrap();
        assert_eq!(snapshots[0].task.status, FileTransferStatus::Transferring);
        assert_eq!(snapshots[0].task.transferred_bytes, 11);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn receiver_appends_an_http_range_response_to_the_existing_partial_file() {
        let content = b"hello world";
        let expected_hash = format!("{:x}", Sha256::digest(content));
        let part = temp_path("http-range-append.part");
        fs::write(&part, b"hello ").unwrap();
        let (offset, hasher) = hash_partial_file(&part, content.len() as u64).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_hash = expected_hash.clone();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = file_transfer_http::read_head(&mut stream).await.unwrap();
            assert_eq!(
                request.headers.get("range").map(String::as_str),
                Some("bytes=6-")
            );
            let response = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Length: 5\r\nContent-Range: bytes 6-10/11\r\nETag: \"sha256:{server_hash}\"\r\nConnection: close\r\n\r\nworld"
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let actual_hash = receive_http_to_part(
            "127.0.0.1",
            address.port(),
            "/file-download?transferId=t&fileId=f&token=token",
            offset,
            false,
            content.len() as u64,
            &expected_hash,
            &part,
            hasher,
            |_| async { Ok(()) },
        )
        .await
        .unwrap();

        server.await.unwrap();
        assert_eq!(actual_hash, expected_hash);
        assert_eq!(fs::read(&part).unwrap(), content);
        let _ = fs::remove_file(part);
    }

    #[tokio::test]
    async fn receiver_restarts_safely_from_zero_when_a_range_request_gets_http_200() {
        let content = b"hello world";
        let expected_hash = format!("{:x}", Sha256::digest(content));
        let part = temp_path("http-range-ignored.part");
        fs::write(&part, b"hello ").unwrap();
        let (offset, hasher) = hash_partial_file(&part, content.len() as u64).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server_hash = expected_hash.clone();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = file_transfer_http::read_head(&mut stream).await.unwrap();
            assert_eq!(
                request.headers.get("range").map(String::as_str),
                Some("bytes=6-")
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: 11\r\nETag: \"sha256:{server_hash}\"\r\nConnection: close\r\n\r\nhello world"
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        let result = receive_http_to_part(
            "127.0.0.1",
            address.port(),
            "/file-download?transferId=t&fileId=f&token=token",
            offset,
            false,
            content.len() as u64,
            &expected_hash,
            &part,
            hasher,
            |_| async { Ok(()) },
        )
        .await;

        server.await.unwrap();
        assert!(matches!(
            result,
            Err(crate::error::AppError::InvalidInput(message))
                if message == "restart download from zero"
        ));
        assert_eq!(fs::read(&part).unwrap(), b"hello ");

        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task(
                "device-a",
                "video.mp4",
                content.len() as u64,
                &expected_hash,
                part.clone(),
            )
            .await;
        manager
            .reset_current_partial(&transfer_id, "file-1")
            .await
            .unwrap();
        assert!(!part.exists());
        let task = manager.task(&transfer_id).await.unwrap();
        assert_eq!(task.files[0].transferred_bytes, 0);
        assert_eq!(task.files[0].status, FileTransferFileStatus::Pending);
    }

    #[tokio::test]
    async fn interrupted_http_downloads_use_three_retries_then_pause_with_partial_data() {
        let content = b"hello world";
        let expected_hash = format!("{:x}", Sha256::digest(content));
        let part = temp_path("http-interrupted-retries.part");
        fs::write(&part, b"hello ").unwrap();
        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task(
                "device-a",
                "video.mp4",
                content.len() as u64,
                &expected_hash,
                part.clone(),
            )
            .await;
        let expected_delays = [
            Some(Duration::from_secs(1)),
            Some(Duration::from_secs(3)),
            Some(Duration::from_secs(10)),
            None,
        ];

        for expected_delay in expected_delays {
            let (offset, hasher) = hash_partial_file(&part, content.len() as u64).await.unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let server_hash = expected_hash.clone();
            let server = tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let _ = file_transfer_http::read_head(&mut stream).await.unwrap();
                let response = format!(
                    "HTTP/1.1 206 Partial Content\r\nContent-Length: 5\r\nContent-Range: bytes 6-10/11\r\nETag: \"sha256:{server_hash}\"\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            });

            let result = receive_http_to_part(
                "127.0.0.1",
                address.port(),
                "/file-download?transferId=t&fileId=f&token=token",
                offset,
                false,
                content.len() as u64,
                &expected_hash,
                &part,
                hasher,
                |_| async { Ok(()) },
            )
            .await;
            server.await.unwrap();
            assert!(matches!(
                result,
                Err(crate::error::AppError::ConnectionTimeout(_))
            ));
            assert_eq!(
                manager
                    .schedule_transient_retry(&transfer_id, "connection closed")
                    .await
                    .unwrap(),
                expected_delay
            );
        }

        let task = manager.task(&transfer_id).await.unwrap();
        assert_eq!(task.status, FileTransferStatus::Paused);
        assert_eq!(fs::read(&part).unwrap(), b"hello ");
        let _ = fs::remove_file(part);
    }

    #[tokio::test]
    async fn multi_file_tokens_are_independent_and_single_use() {
        let manager = FileTransferManager::new();
        let first = temp_path("first.txt");
        let second = temp_path("second.txt");
        fs::write(&first, b"first").unwrap();
        fs::write(&second, b"second").unwrap();
        let transfer_id = manager
            .insert_test_send_task_with_files(
                "device-b",
                vec![
                    ("file-1", first.clone(), "first.txt", "token-1"),
                    ("file-2", second.clone(), "second.txt", "token-2"),
                ],
            )
            .await;

        assert!(matches!(
            manager
                .claim_download_for_test(&transfer_id, "file-1", "token-2")
                .await,
            Err(DownloadClaimError::Forbidden)
        ));
        assert!(manager
            .claim_download_for_test(&transfer_id, "file-1", "token-1")
            .await
            .is_ok());
        assert!(manager
            .claim_download_for_test(&transfer_id, "file-2", "token-2")
            .await
            .is_ok());
        assert!(matches!(
            manager
                .claim_download_for_test(&transfer_id, "file-2", "token-2")
                .await,
            Err(DownloadClaimError::Forbidden)
        ));

        let _ = fs::remove_file(first);
        let _ = fs::remove_file(second);
    }

    #[test]
    fn multi_file_limits_reject_too_many_files_and_oversize_total() {
        assert!(validate_transfer_limits(MAX_TRANSFER_FILE_COUNT + 1, &[1]).is_err());
        assert!(validate_transfer_limits(1, &[MAX_SINGLE_FILE_SIZE + 1]).is_err());
        assert!(validate_transfer_limits(
            3,
            &[
                MAX_TRANSFER_TOTAL_SIZE / 2,
                MAX_TRANSFER_TOTAL_SIZE / 2,
                1,
            ],
        )
        .is_err());

        const GIB: u64 = 1024 * 1024 * 1024;
        assert!(validate_transfer_limits(1, &[2 * GIB]).is_ok());
        assert!(validate_transfer_limits(1, &[2 * GIB + 1]).is_err());
        assert!(validate_transfer_limits(3, &[2 * GIB, 2 * GIB, GIB]).is_ok());
        assert!(validate_transfer_limits(3, &[2 * GIB, 2 * GIB, GIB + 1]).is_err());
    }

    #[test]
    fn configured_file_size_limit_accepts_boundary_and_rejects_larger_files() {
        const MIB: u64 = 1024 * 1024;

        assert!(validate_configured_single_file_limit(&[500 * MIB], 500, "send").is_ok());
        assert!(
            validate_configured_single_file_limit(&[500 * MIB + 1], 500, "send").is_err()
        );
        assert!(validate_configured_single_file_limit(&[2048 * MIB], 2048, "receive").is_ok());
    }

    #[test]
    fn duplicate_file_names_get_unique_save_paths() {
        let dir = temp_path("save-dir");
        fs::create_dir_all(&dir).unwrap();
        let first = unique_save_path(&dir, "file.txt");
        fs::write(&first, b"first").unwrap();
        let second = unique_save_path(&dir, "file.txt");

        assert_eq!(first.file_name().unwrap().to_str().unwrap(), "file.txt");
        assert_eq!(second.file_name().unwrap().to_str().unwrap(), "file (1).txt");

        let _ = fs::remove_file(first);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn partial_file_reserves_its_final_download_name() {
        let dir = temp_path("part-reserved-save-dir");
        fs::create_dir_all(&dir).unwrap();
        let first = unique_save_path(&dir, "file.txt");
        let part = part_path_for(&first);
        fs::write(&part, b"partial").unwrap();

        let second = unique_save_path(&dir, "file.txt");

        assert_eq!(second.file_name().unwrap().to_str().unwrap(), "file (1).txt");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn untrusted_device_cannot_create_file_offer() {
        let mut device = trusted_device();
        device.remote_trusted = false;

        let result = trusted_transfer_peer(&[device], "device-b");

        assert!(result.is_err());
    }

    #[test]
    fn trusted_device_can_be_selected_by_endpoint_alias() {
        let device = trusted_device();

        let by_ws_endpoint = trusted_transfer_peer(&[device.clone()], "ws://10.0.0.2:8765/");
        let by_host_port = trusted_transfer_peer(&[device], "10.0.0.2:8765");

        assert!(by_ws_endpoint.is_ok());
        assert!(by_host_port.is_ok());
    }

    #[test]
    fn file_offer_advertises_existing_sync_listener_port() {
        let endpoint = advertised_download_endpoint(
            Some("192.168.1.10".to_string()),
            8765,
            Some("192.168.1.20".parse().unwrap()),
        )
        .unwrap();

        assert_eq!(endpoint.advertised_host, "192.168.1.10");
        assert_eq!(endpoint.port, 8765);
    }

    #[test]
    fn transfer_save_dir_uses_custom_config_or_default() {
        let custom = temp_path("receive-dir");
        let config = crate::models::AppConfig {
            file_save_dir: Some(custom.to_string_lossy().to_string()),
            ..crate::models::AppConfig::default()
        };

        assert_eq!(transfer_save_dir(&config).unwrap(), custom);
        assert!(current_transfer_save_dir(&crate::models::AppConfig::default())
            .unwrap()
            .ends_with("Copy-Sharer"));
    }

    #[tokio::test]
    async fn clipboard_pending_history_content_uses_metadata_without_local_paths() {
        let manager = FileTransferManager::new();
        let source = temp_path("source.txt");
        fs::write(&source, b"hello").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "hello.txt", "token")
            .await;
        let task = manager.task(&transfer_id).await.expect("task should exist");

        let content = pending_clipboard_file_content(&task).expect("pending content");
        let entries: Vec<crate::clipboard::ClipboardFileEntry> =
            serde_json::from_str(&content).expect("metadata should parse");

        assert_eq!(entries[0].name, "hello.txt");
        assert_eq!(entries[0].path, "");
        assert_eq!(entries[0].size, 5);
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn clipboard_sync_writes_only_once_after_every_file_is_complete() {
        let first_path = temp_path("clipboard-first.txt");
        let second_path = temp_path("clipboard-second.txt");
        fs::write(&first_path, b"first").unwrap();
        fs::write(&second_path, b"second").unwrap();
        let mut first = new_transfer_file(
            "file-1".to_string(),
            "first.txt".to_string(),
            5,
            "hash-1".to_string(),
            None,
        );
        first.status = FileTransferFileStatus::Completed;
        first.transferred_bytes = first.size;
        first.saved_path = Some(first_path.to_string_lossy().to_string());
        let second = new_transfer_file(
            "file-2".to_string(),
            "second.txt".to_string(),
            6,
            "hash-2".to_string(),
            None,
        );
        let mut task = crate::models::FileTransferTask {
            transfer_id: "clipboard-transfer".to_string(),
            direction: crate::models::FileTransferDirection::Receive,
            peer_device_id: "sender-device".to_string(),
            peer_device_name: "Sender".to_string(),
            clipboard_sync: true,
            files: vec![first, second],
            total_size: 11,
            transferred_bytes: 5,
            status: FileTransferStatus::Transferring,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        };
        let state = AppState::new();
        let writes = AtomicUsize::new(0);

        let incomplete = apply_completed_clipboard_file_sync_core(&state, &task, |_| {
            writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap();
        assert!(incomplete.is_none());
        assert_eq!(writes.load(Ordering::SeqCst), 0);

        task.status = FileTransferStatus::Completed;
        task.transferred_bytes = task.total_size;
        task.files[1].status = FileTransferFileStatus::Completed;
        task.files[1].transferred_bytes = task.files[1].size;
        task.files[1].saved_path = Some(second_path.to_string_lossy().to_string());
        let applied = apply_completed_clipboard_file_sync_core(&state, &task, |_| {
            writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap();
        assert!(applied.is_some());
        assert!(applied.as_ref().unwrap().1.event_version.is_some());
        assert_eq!(writes.load(Ordering::SeqCst), 1);

        let duplicate = apply_completed_clipboard_file_sync_core(&state, &task, |_| {
            writes.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
        .await
        .unwrap();
        assert!(duplicate.is_none());
        assert_eq!(writes.load(Ordering::SeqCst), 1);
        let _ = fs::remove_file(first_path);
        let _ = fs::remove_file(second_path);
    }

    #[tokio::test]
    async fn hash_mismatch_marks_failed_and_removes_temp_file() {
        let manager = FileTransferManager::new();
        let temp = temp_path("received.part");
        let final_path = temp_path("received.txt");
        fs::write(&temp, b"actual").unwrap();
        let expected = format!("{:x}", Sha256::digest(b"expected"));
        let transfer_id = manager
            .insert_test_receive_task("device-a", "received.txt", 6, &expected, temp.clone())
            .await;

        let result = manager
            .finish_receive_for_test(&transfer_id, temp.clone(), final_path.clone(), expected)
            .await;

        assert!(result.is_err());
        assert!(!temp.exists());
        assert!(!final_path.exists());
        assert_eq!(
            manager.task_status_for_test(&transfer_id).await,
            Some(FileTransferStatus::Failed)
        );
    }

    #[tokio::test]
    async fn resume_receive_task_restores_from_manifest_and_part_length() {
        let root = temp_path("resume-store-root");
        let part = root.join("video.mp4.part");
        fs::create_dir_all(&root).unwrap();
        fs::write(&part, b"partial").unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_receive_task("device-a", "video.mp4", 20, "hash", part)
            .await;
        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        let restored = FileTransferManager::new();
        restored.set_store_root_for_test(root.clone()).await;
        restored.restore_from_store_for_test().await.unwrap();
        let task = restored.task(&transfer_id).await.unwrap();

        assert_eq!(task.status, FileTransferStatus::WaitingForPeer);
        assert_eq!(task.files[0].transferred_bytes, 7);
        assert_eq!(task.transferred_bytes, 7);
        assert!(!restored.has_download_token_for_test(&transfer_id, "file-1").await);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn receiver_snapshot_omits_the_ephemeral_download_endpoint() {
        let part = temp_path("receiver-endpoint.part");
        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task("device-a", "video.mp4", 20, "hash", part.clone())
            .await;
        let snapshot = {
            let mut tasks = manager.tasks.lock().await;
            let transfer = tasks.get_mut(&transfer_id).unwrap();
            transfer.download_host = Some("192.168.1.10".to_string());
            transfer.download_port = Some(8765);
            transfer_snapshot(transfer)
        };

        assert_eq!(snapshot.download_host, None);
        assert_eq!(snapshot.download_port, None);
        let _ = fs::remove_file(part);
    }

    #[tokio::test]
    async fn pending_receive_offer_stays_pending_after_restart() {
        let root = temp_path("pending-offer-store");
        let part = root.join("video.mp4.part");
        fs::create_dir_all(&root).unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_receive_task("device-a", "video.mp4", 20, "hash", part)
            .await;
        {
            let mut tasks = manager.tasks.lock().await;
            tasks.get_mut(&transfer_id).unwrap().task.status = FileTransferStatus::Pending;
        }
        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        let restored = FileTransferManager::new();
        restored.set_store_root_for_test(root.clone()).await;
        restored.restore_from_store_for_test().await.unwrap();

        assert_eq!(
            restored.task(&transfer_id).await.unwrap().status,
            FileTransferStatus::Pending
        );
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn oversized_partial_file_restores_as_paused_instead_of_aborting_startup() {
        let root = temp_path("oversized-part-store");
        let part = root.join("video.mp4.part");
        fs::create_dir_all(&root).unwrap();
        fs::write(&part, vec![0_u8; 21]).unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_receive_task("device-a", "video.mp4", 20, "hash", part)
            .await;
        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        let restored = FileTransferManager::new();
        restored.set_store_root_for_test(root.clone()).await;
        restored.restore_from_store_for_test().await.unwrap();

        let task = restored.task(&transfer_id).await.unwrap();
        assert_eq!(task.status, FileTransferStatus::Paused);
        assert!(task.error.unwrap().contains("partial file is larger"));
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn startup_returns_expired_tasks_as_failed_for_history_reconciliation() {
        let root = temp_path("expired-startup-store");
        let part = root.join("video.mp4.part");
        fs::create_dir_all(&root).unwrap();
        fs::write(&part, b"partial").unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_receive_task("device-a", "video.mp4", 20, "hash", part.clone())
            .await;
        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();
        let mut snapshot = file_transfer_store::load_all(&root).unwrap().remove(0);
        snapshot.last_activity_at = Utc::now() - chrono::Duration::days(8);
        file_transfer_store::save(&root, &snapshot).unwrap();

        let restored = FileTransferManager::new();
        restored.set_store_root_for_test(root.clone()).await;
        let expired = restored.restore_from_store_for_test().await.unwrap();

        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].transfer_id, transfer_id);
        assert_eq!(expired[0].status, FileTransferStatus::Failed);
        assert_eq!(
            expired[0].error.as_deref(),
            Some("file transfer task expired")
        );
        assert!(expired[0].completed_at.is_some());
        assert!(restored.task(&transfer_id).await.is_none());
        assert!(!part.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn resume_hasher_is_seeded_with_existing_partial_bytes() {
        let part = temp_path("resume-hasher.part");
        fs::write(&part, b"hello ").unwrap();

        let (offset, mut hasher) = hash_partial_file(&part, 11).await.unwrap();
        hasher.update(b"world");

        assert_eq!(offset, 6);
        assert_eq!(
            format!("{:x}", hasher.finalize()),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        let _ = fs::remove_file(part);
    }

    #[tokio::test]
    async fn resume_grant_replaces_receiver_token_at_the_actual_partial_offset() {
        let partial = temp_path("grant-offset.part");
        fs::write(&partial, b"partial").unwrap();
        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task(
                "device-a",
                "video.mp4",
                20,
                "hash",
                partial.clone(),
            )
            .await;

        manager
            .apply_resume_grant(
                &transfer_id,
                "device-a",
                "192.168.1.10".to_string(),
                8765,
                "device-b",
                vec![FileResumeGrantFile {
                    file_id: "file-1".to_string(),
                    offset: 7,
                    token: "fresh-token".to_string(),
                }],
            )
            .await
            .unwrap();

        let tasks = manager.tasks.lock().await;
        let transfer = tasks.get(&transfer_id).unwrap();
        let file = transfer.files.get("file-1").unwrap();
        assert_eq!(file.token.as_deref(), Some("fresh-token"));
        assert_eq!(file.token_offset, 7);
        assert_eq!(file.token_receiver_device_id.as_deref(), Some("device-b"));
        assert_eq!(transfer.download_host.as_deref(), Some("192.168.1.10"));
        assert_eq!(transfer.download_port, Some(8765));
        assert_eq!(transfer.task.status, FileTransferStatus::Retrying);
        drop(tasks);
        let _ = fs::remove_file(partial);
    }

    #[tokio::test]
    async fn next_receive_plan_skips_files_that_are_already_complete() {
        let first_part = temp_path("completed-first.part");
        let second_part = temp_path("pending-second.part");
        fs::write(&first_part, b"first").unwrap();
        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task(
                "device-a",
                "first.txt",
                5,
                "hash-1",
                first_part.clone(),
            )
            .await;
        {
            let mut tasks = manager.tasks.lock().await;
            let transfer = tasks.get_mut(&transfer_id).unwrap();
            transfer.task.files[0].status = FileTransferFileStatus::Completed;
            transfer.task.files[0].transferred_bytes = 5;
            transfer.task.files.push(new_transfer_file(
                "file-2".to_string(),
                "second.txt".to_string(),
                6,
                "hash-2".to_string(),
                None,
            ));
            transfer.task.total_size = 11;
            transfer.files.insert(
                "file-2".to_string(),
                ManagedTransferFile {
                    source_path: None,
                    source_size: None,
                    source_modified_ms: None,
                    token: Some("second-token".to_string()),
                    token_consumed: false,
                    token_offset: 0,
                    token_expires_at: None,
                    token_receiver_device_id: None,
                    temp_path: Some(second_part.clone()),
                    final_path: None,
                },
            );
            transfer.download_host = Some("192.168.1.10".to_string());
            transfer.download_port = Some(8765);
        }

        let (_, _, _, plan) = manager
            .next_receive_plan(&transfer_id)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(plan.file.id, "file-2");
        assert_eq!(plan.token.as_deref(), Some("second-token"));
        let _ = fs::remove_file(first_part);
        let _ = fs::remove_file(second_part);
    }

    #[tokio::test]
    async fn accepted_transfer_status_is_persisted() {
        let root = temp_path("accepted-transfer-store");
        let source = root.join("source.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"hello").unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_send_task("device-b", source, "hello.txt", "token")
            .await;
        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        manager
            .mark_status(&transfer_id, FileTransferStatus::Accepted)
            .await
            .unwrap();

        let snapshots = file_transfer_store::load_all(&root).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].task.status, FileTransferStatus::Accepted);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn completed_transfer_removes_its_persisted_snapshot() {
        let root = temp_path("completed-transfer-store");
        let source = root.join("source.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"hello").unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_send_task("device-b", source, "hello.txt", "token")
            .await;
        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        manager.mark_completed(&transfer_id).await.unwrap();

        assert!(file_transfer_store::load_all(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn sender_snapshot_records_the_source_modified_time() {
        let root = temp_path("sender-mtime-store");
        let source = root.join("source.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&source, b"hello").unwrap();
        let manager = FileTransferManager::new();
        manager.set_store_root_for_test(root.clone()).await;
        let transfer_id = manager
            .insert_test_send_task("device-b", source, "hello.txt", "token")
            .await;

        manager
            .persist_transfer_for_test(&transfer_id)
            .await
            .unwrap();

        let snapshots = file_transfer_store::load_all(&root).unwrap();
        assert!(snapshots[0].files[0].source_modified_ms.is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn transient_failures_schedule_three_retries_then_pause_and_keep_partial_data() {
        let partial = temp_path("retry-preserved.part");
        fs::write(&partial, b"partial").unwrap();
        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task(
                "device-a",
                "video.mp4",
                20,
                "hash",
                partial.clone(),
            )
            .await;

        assert_eq!(
            manager
                .schedule_transient_retry(&transfer_id, "connection reset")
                .await
                .unwrap(),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            manager
                .schedule_transient_retry(&transfer_id, "connection reset")
                .await
                .unwrap(),
            Some(Duration::from_secs(3))
        );
        assert_eq!(
            manager
                .schedule_transient_retry(&transfer_id, "connection reset")
                .await
                .unwrap(),
            Some(Duration::from_secs(10))
        );
        assert_eq!(
            manager
                .schedule_transient_retry(&transfer_id, "connection reset")
                .await
                .unwrap(),
            None
        );

        let tasks = manager.tasks.lock().await;
        let transfer = tasks.get(&transfer_id).unwrap();
        assert_eq!(transfer.retry_count, 3);
        assert_eq!(transfer.task.status, FileTransferStatus::Paused);
        assert!(partial.exists());
        drop(tasks);
        let _ = fs::remove_file(partial);
    }

    #[tokio::test]
    async fn peer_disconnect_moves_receive_tasks_to_waiting_without_consuming_a_retry() {
        let partial = temp_path("disconnect-preserved.part");
        fs::write(&partial, b"partial").unwrap();
        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task(
                "device-a",
                "video.mp4",
                20,
                "hash",
                partial.clone(),
            )
            .await;

        let changed = manager.mark_peer_disconnected("device-a").await.unwrap();

        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].transfer_id, transfer_id);
        assert_eq!(changed[0].status, FileTransferStatus::WaitingForPeer);
        let tasks = manager.tasks.lock().await;
        assert_eq!(tasks[&transfer_id].retry_count, 0);
        assert!(partial.exists());
        drop(tasks);
        let _ = fs::remove_file(partial);
    }

    #[tokio::test]
    async fn manual_resume_resets_the_retry_budget() {
        let partial = temp_path("manual-retry.part");
        fs::write(&partial, b"partial").unwrap();
        let manager = FileTransferManager::new();
        let transfer_id = manager
            .insert_test_receive_task(
                "device-a",
                "video.mp4",
                20,
                "hash",
                partial.clone(),
            )
            .await;
        for _ in 0..4 {
            manager
                .schedule_transient_retry(&transfer_id, "connection reset")
                .await
                .unwrap();
        }

        let task = manager.reset_manual_retry(&transfer_id).await.unwrap();

        assert_eq!(task.status, FileTransferStatus::Retrying);
        assert_eq!(manager.tasks.lock().await[&transfer_id].retry_count, 0);
        let _ = fs::remove_file(partial);
    }

    #[test]
    fn progress_updates_use_the_configured_checkpoint_and_event_intervals() {
        let now = std::time::Instant::now();

        assert!(!super::should_persist_progress(
            0,
            now,
            8 * 1024 * 1024 - 1,
            now + Duration::from_millis(999),
        ));
        assert!(super::should_persist_progress(
            0,
            now,
            8 * 1024 * 1024,
            now + Duration::from_millis(1),
        ));
        assert!(super::should_persist_progress(
            0,
            now,
            1,
            now + Duration::from_secs(1),
        ));
        assert!(!super::should_emit_progress(
            Some(now),
            now + Duration::from_millis(199),
        ));
        assert!(super::should_emit_progress(
            Some(now),
            now + Duration::from_millis(200),
        ));
    }

    #[tokio::test]
    async fn cancel_task_sets_canceled_status() {
        let manager = FileTransferManager::new();
        let source = temp_path("source.txt");
        fs::write(&source, b"hello").unwrap();
        let transfer_id = manager
            .insert_test_send_task("device-b", source.clone(), "hello.txt", "token")
            .await;

        manager.cancel_local(&transfer_id).await.unwrap();

        assert_eq!(
            manager.task_status_for_test(&transfer_id).await,
            Some(FileTransferStatus::Canceled)
        );
        let _ = fs::remove_file(source);
    }

    #[tokio::test]
    async fn completed_transfer_tasks_are_pruned_to_recent_entries() {
        let manager = FileTransferManager::new();
        let mut sources = Vec::new();

        for index in 0..120 {
            let source = temp_path(&format!("source-{index}.txt"));
            fs::write(&source, b"hello").unwrap();
            let transfer_id = manager
                .insert_test_send_task(
                    "device-b",
                    source.clone(),
                    &format!("hello-{index}.txt"),
                    &format!("token-{index}"),
                )
                .await;
            manager.mark_completed(&transfer_id).await.unwrap();
            sources.push(source);
        }

        assert_eq!(
            manager.tasks().await.len(),
            100,
            "completed transfer tasks should be capped"
        );

        for source in sources {
            let _ = fs::remove_file(source);
        }
    }
}
