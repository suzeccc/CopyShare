use std::{
    collections::HashMap,
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use chrono::Utc;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::Mutex,
};
use uuid::Uuid;

use crate::{
    clipboard, history,
    error::{AppError, AppResult},
    models::{
        AppConfig, ClipboardContentType, ClipboardMessage, CopyHistoryResult, DeviceInfo,
        FileCompleteFile, FileOfferFile, FileTransferDirection, FileTransferFile, FileTransferFileStatus,
        FileTransferProgressEvent, FileTransferStatus, FileTransferTask, HistoryDirection,
        HistoryItem, SelectedTransferFile, WireMessage,
    },
    network,
    notifications,
    state::AppState,
};

const MAX_SINGLE_FILE_SIZE: u64 = 500 * 1024 * 1024;
const MAX_TRANSFER_FILE_COUNT: usize = 100;
const MAX_TRANSFER_TOTAL_SIZE: u64 = 1024 * 1024 * 1024;
const TRANSFER_CHUNK_SIZE: usize = 64 * 1024;

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
    clipboard_sync: bool,
}

#[derive(Debug, Clone)]
struct ManagedTransferFile {
    source_path: Option<PathBuf>,
    token: String,
    token_consumed: bool,
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
}

#[derive(Debug, Clone)]
struct ReceiveFilePlan {
    file: FileTransferFile,
    token: String,
}

pub struct FileTransferManager {
    tasks: Mutex<HashMap<String, ManagedTransfer>>,
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
        })
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
        let selected_files = selected_files_from_paths(file_paths).await?;
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
            let task_file = new_transfer_file(
                file_id.clone(),
                selected.name.clone(),
                selected.size,
                selected.sha256.clone(),
            );
            managed_files.insert(
                file_id.clone(),
                ManagedTransferFile {
                    source_path: Some(PathBuf::from(&selected.path)),
                    token: token.clone(),
                    token_consumed: false,
                    temp_path: None,
                    final_path: None,
                },
            );
            offer_files.push(FileOfferFile {
                file_id,
                file_name: selected.name,
                file_size: selected.size,
                sha256: selected.sha256,
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
                clipboard_sync,
            },
        );
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
        mut stream: TcpStream,
    ) -> AppResult<()> {
        let Some(target) = read_http_target(&mut stream).await? else {
            write_http_response(&mut stream, 400, "Bad Request", b"bad request").await?;
            return Ok(());
        };
        let Some((transfer_id, file_id, token)) = parse_download_query(&target) else {
            write_http_response(&mut stream, 403, "Forbidden", b"forbidden").await?;
            return Ok(());
        };

        let claim = match self.claim_download(&transfer_id, &file_id, &token).await {
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

        let mut file = File::open(&claim.path).await?;
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            claim.size
        );
        stream.write_all(header.as_bytes()).await?;

        let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];
        let mut transferred = 0_u64;
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
            self.update_progress(&app, &claim.transfer_id, &claim.file_id, transferred)
                .await?;
        }
        Ok(())
    }

    async fn claim_download(
        &self,
        transfer_id: &str,
        file_id: &str,
        token: &str,
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

        let size = entry
            .task
            .files
            .iter()
            .find(|file| file.id == file_id)
            .map(|file| file.size)
            .ok_or(DownloadClaimError::NotFound)?;
        let managed_file = entry
            .files
            .get_mut(file_id)
            .ok_or(DownloadClaimError::NotFound)?;
        if managed_file.token != token || managed_file.token_consumed {
            return Err(DownloadClaimError::Forbidden);
        }
        let path = managed_file
            .source_path
            .clone()
            .ok_or(DownloadClaimError::NotFound)?;
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
        })
    }

    async fn update_progress(
        &self,
        app: &AppHandle,
        transfer_id: &str,
        file_id: &str,
        file_transferred_bytes: u64,
    ) -> AppResult<FileTransferTask> {
        let task = {
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
            }
            entry.task.clone()
        };
        emit_progress(app, &task, file_id);
        emit_task_updated(app, &task);
        Ok(task)
    }

    async fn is_canceled(&self, transfer_id: &str) -> bool {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| entry.task.status == FileTransferStatus::Canceled)
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
            let failed = self
                .mark_failed(&transfer_id, "cannot notify sender to start transfer")
                .await?;
            emit_task_failed(&app, &failed);
            return Err(AppError::InvalidInput(
                "sender is not connected or mutually trusted".to_string(),
            ));
        }
        emit_task_updated(&app, &task);

        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = manager
                .clone()
                .download_receive(app.clone(), state.clone(), transfer_id.clone())
                .await
            {
                if manager.is_canceled(&transfer_id).await {
                    manager.cleanup_temp(&transfer_id).await;
                    return;
                }
                if let Ok(task) = manager.mark_failed(&transfer_id, &error.to_string()).await {
                    manager.cleanup_temp(&transfer_id).await;
                    if task.clipboard_sync {
                        let _ = update_clipboard_file_history_status(&app, &state, &task, None)
                            .await;
                    }
                    let _ = state
                        .send_trusted_to_device(
                            &state.config().await,
                            &task.peer_device_id,
                            WireMessage::FileError {
                                transfer_id: transfer_id.clone(),
                                file_id: None,
                                device_id: state.status().await.device_id,
                                message: error.to_string(),
                            },
                        )
                        .await;
                    emit_task_failed(&app, &task);
                }
            }
        });

        Ok(task)
    }

    async fn download_receive(
        self: Arc<Self>,
        app: AppHandle,
        state: AppState,
        transfer_id: String,
    ) -> AppResult<()> {
        let (host, port, peer_device_id, plans) = self.receive_plan(&transfer_id).await?;
        let save_dir = transfer_save_dir(&state.config().await)?;
        fs::create_dir_all(&save_dir).await?;

        let mut completed_files = Vec::with_capacity(plans.len());
        for plan in plans {
            if self.is_canceled(&transfer_id).await {
                self.cleanup_temp(&transfer_id).await;
                return Ok(());
            }
            let result = self
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
                .await;
            match result {
                Ok(actual_hash) => completed_files.push(FileCompleteFile {
                    file_id: plan.file.id.clone(),
                    sha256: actual_hash,
                }),
                Err(_error) if self.is_canceled(&transfer_id).await => {
                    self.cleanup_temp(&transfer_id).await;
                    return Ok(());
                }
                Err(error) => {
                    let message = error.to_string();
                    let _ = self
                        .mark_file_failed(&transfer_id, &plan.file.id, &message)
                        .await;
                    self.cleanup_temp(&transfer_id).await;
                    return Err(error);
                }
            }
        }

        let completed = self.mark_completed(&transfer_id).await?;
        if self.is_clipboard_sync(&transfer_id).await {
            if let Err(error) = apply_completed_clipboard_file_sync(&app, &state, &completed).await {
                tracing::warn!("clipboard file sync apply failed: {error}");
            }
        }
        let _ = state
            .send_trusted_to_device(
                &state.config().await,
                &peer_device_id,
                WireMessage::FileComplete {
                    transfer_id: transfer_id.clone(),
                    device_id: state.status().await.device_id,
                    files: completed_files,
                },
            )
            .await;
        emit_task_completed(&app, &completed);
        Ok(())
    }

    async fn is_clipboard_sync(&self, transfer_id: &str) -> bool {
        self.tasks
            .lock()
            .await
            .get(transfer_id)
            .map(|entry| entry.clipboard_sync)
            .unwrap_or(false)
    }

    async fn receive_plan(
        &self,
        transfer_id: &str,
    ) -> AppResult<(String, u16, String, Vec<ReceiveFilePlan>)> {
        let tasks = self.tasks.lock().await;
        let entry = tasks
            .get(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        let host = entry
            .download_host
            .clone()
            .ok_or_else(|| AppError::InvalidInput("missing download host".to_string()))?;
        let port = entry
            .download_port
            .ok_or_else(|| AppError::InvalidInput("missing download port".to_string()))?;
        let peer_device_id = entry.task.peer_device_id.clone();
        let plans = entry
            .task
            .files
            .iter()
            .map(|file| {
                let token = entry
                    .files
                    .get(&file.id)
                    .map(|managed| managed.token.clone())
                    .ok_or_else(|| AppError::InvalidInput("missing download token".to_string()))?;
                Ok(ReceiveFilePlan {
                    file: file.clone(),
                    token,
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok((host, port, peer_device_id, plans))
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
        let final_path = unique_save_path(save_dir, &plan.file.name);
        let temp_path = part_path_for(&final_path);
        self.prepare_receive_file(
            app,
            transfer_id,
            &plan.file.id,
            temp_path.clone(),
            final_path.clone(),
        )
        .await?;

        let mut stream = TcpStream::connect((host, port)).await?;
        let request_target = download_request_target(transfer_id, &plan.file.id, &plan.token);
        let request = format!(
            "GET {request_target} HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await?;

        let (status_line, mut body) = read_http_response_head(&mut stream).await?;
        if !status_line.contains(" 200 ") {
            let _ = fs::remove_file(&temp_path).await;
            return Err(AppError::InvalidInput(format!(
                "file download service rejected request: {status_line}"
            )));
        }

        let mut output = File::create(&temp_path).await?;
        let mut hasher = Sha256::new();
        let mut transferred = 0_u64;
        if !body.is_empty() {
            output.write_all(&body).await?;
            hasher.update(&body);
            transferred += body.len() as u64;
            self.update_receive_progress(
                app,
                state,
                transfer_id,
                peer_device_id,
                &plan.file.id,
                transferred,
            )
            .await?;
        }

        let mut buffer = vec![0; TRANSFER_CHUNK_SIZE];
        loop {
            if self.is_canceled(transfer_id).await {
                let _ = fs::remove_file(&temp_path).await;
                return Err(AppError::InvalidInput("transfer canceled".to_string()));
            }
            let read = stream.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            body.clear();
            output.write_all(&buffer[..read]).await?;
            hasher.update(&buffer[..read]);
            transferred += read as u64;
            self.update_receive_progress(
                app,
                state,
                transfer_id,
                peer_device_id,
                &plan.file.id,
                transferred,
            )
            .await?;
        }
        output.flush().await?;

        if transferred != plan.file.size {
            let _ = fs::remove_file(&temp_path).await;
            return Err(AppError::InvalidInput("file size mismatch".to_string()));
        }

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
        Ok(actual_hash)
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
        let task = self
            .update_progress(app, transfer_id, file_id, transferred)
            .await?;
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
    ) -> AppResult<FileTransferTask> {
        if offer.file_count != offer.files.len() {
            return Err(AppError::InvalidInput("file count mismatch".to_string()));
        }
        let sizes = offer.files.iter().map(|file| file.file_size).collect::<Vec<_>>();
        validate_transfer_limits(offer.file_count, &sizes)?;
        let calculated_total = sizes.iter().sum::<u64>();
        if calculated_total != offer.total_size {
            return Err(AppError::InvalidInput("total size mismatch".to_string()));
        }

        let mut task_files = Vec::with_capacity(offer.files.len());
        let mut managed_files = HashMap::new();
        for offered_file in offer.files {
            let file_name = sanitize_file_name(&offered_file.file_name);
            task_files.push(new_transfer_file(
                offered_file.file_id.clone(),
                file_name,
                offered_file.file_size,
                offered_file.sha256,
            ));
            managed_files.insert(
                offered_file.file_id,
                ManagedTransferFile {
                    source_path: None,
                    token: offered_file.token,
                    token_consumed: false,
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
                clipboard_sync: offer.clipboard_sync,
            },
        );
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
        emit_task_failed(app, &task);
        Ok(task)
    }

    async fn mark_status(
        &self,
        transfer_id: &str,
        status: FileTransferStatus,
    ) -> AppResult<FileTransferTask> {
        let mut tasks = self.tasks.lock().await;
        let entry = tasks
            .get_mut(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        entry.task.status = status;
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
        if matches!(
            entry.task.status,
            FileTransferStatus::Completed
                | FileTransferStatus::Failed
                | FileTransferStatus::Canceled
                | FileTransferStatus::Rejected
        ) {
            entry.task.completed_at = Some(Utc::now());
        }
        Ok(entry.task.clone())
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
            entry.task.clone()
        };
        emit_progress(app, &task, file_id);
        emit_task_updated(app, &task);
        Ok(task)
    }

    async fn mark_completed(&self, transfer_id: &str) -> AppResult<FileTransferTask> {
        let mut tasks = self.tasks.lock().await;
        let entry = tasks
            .get_mut(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        entry.task.status = FileTransferStatus::Completed;
        entry.task.transferred_bytes = entry.task.total_size;
        entry.task.completed_at = Some(Utc::now());
        entry.task.error = None;
        for file in &mut entry.task.files {
            file.status = FileTransferFileStatus::Completed;
            file.transferred_bytes = file.size;
            file.error = None;
        }
        Ok(entry.task.clone())
    }

    async fn mark_error_text(
        &self,
        transfer_id: &str,
        message: String,
    ) -> AppResult<FileTransferTask> {
        let mut tasks = self.tasks.lock().await;
        let entry = tasks
            .get_mut(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        entry.task.error = Some(message);
        Ok(entry.task.clone())
    }

    async fn mark_failed(&self, transfer_id: &str, message: &str) -> AppResult<FileTransferTask> {
        let mut tasks = self.tasks.lock().await;
        let entry = tasks
            .get_mut(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        entry.task.status = FileTransferStatus::Failed;
        entry.task.error = Some(message.to_string());
        entry.task.completed_at = Some(Utc::now());
        for file in &mut entry.task.files {
            if file.status != FileTransferFileStatus::Completed {
                file.status = FileTransferFileStatus::Failed;
                file.error = Some(message.to_string());
            }
        }
        Ok(entry.task.clone())
    }

    async fn mark_file_failed(
        &self,
        transfer_id: &str,
        file_id: &str,
        message: &str,
    ) -> AppResult<FileTransferTask> {
        let mut tasks = self.tasks.lock().await;
        let entry = tasks
            .get_mut(transfer_id)
            .ok_or_else(|| AppError::InvalidInput("file transfer task not found".to_string()))?;
        entry.task.status = FileTransferStatus::Failed;
        entry.task.error = Some(message.to_string());
        entry.task.completed_at = Some(Utc::now());
        if let Some(file) = entry.task.files.iter_mut().find(|file| file.id == file_id) {
            file.status = FileTransferFileStatus::Failed;
            file.error = Some(message.to_string());
        }
        Ok(entry.task.clone())
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
            let size = std::fs::metadata(&source_path).unwrap().len();
            task_files.push(new_transfer_file(
                file_id.to_string(),
                file_name.to_string(),
                size,
                "hash".to_string(),
            ));
            managed_files.insert(
                file_id.to_string(),
                ManagedTransferFile {
                    source_path: Some(source_path),
                    token: token.to_string(),
                    token_consumed: false,
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
                clipboard_sync: false,
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
                        token: "token".to_string(),
                        token_consumed: false,
                        temp_path: Some(temp_path),
                        final_path: None,
                    },
                )]),
                download_host: None,
                download_port: None,
                clipboard_sync: false,
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
        self.claim_download(transfer_id, file_id, token).await
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

pub async fn get_file_transfers() -> AppResult<Vec<FileTransferTask>> {
    Ok(manager().tasks().await)
}

async fn apply_completed_clipboard_file_sync(
    app: &AppHandle,
    state: &AppState,
    task: &FileTransferTask,
) -> AppResult<()> {
    let paths = task
        .files
        .iter()
        .filter_map(|file| file.saved_path.as_deref())
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(());
    }

    clipboard::write_clipboard_files(app, &paths)?;
    let content = clipboard::file_paths_to_clipboard_content(&paths)?;
    let message = ClipboardMessage {
        message_id: task.transfer_id.clone(),
        source_device_id: task.peer_device_id.clone(),
        source_device_name: task.peer_device_name.clone(),
        content_type: ClipboardContentType::FileList,
        content_hash: clipboard_file_list_hash(&content),
        content: content.clone(),
        timestamp: Utc::now().timestamp(),
    };
    let _ = state.apply_remote_clipboard(&message).await;

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
        let _ = app.emit("clipboard-synced", item);
    }
    Ok(())
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
    let _ = app.emit("clipboard-synced", item);
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
        let _ = app.emit("clipboard-synced", item);
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
        FileTransferStatus::Accepted | FileTransferStatus::Transferring => {
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
    let manager = manager();
    if let Ok(task) = manager.handle_offer(app, offer).await {
        if clipboard_sync {
            let _ = push_pending_clipboard_file_history(app, state, &task).await;
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
    if !file_control_sender_is_trusted(state, connection_id, &receiver_device_id).await {
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
    if !file_control_sender_is_trusted(state, connection_id, &receiver_device_id).await {
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

async fn selected_files_from_paths(paths: Vec<PathBuf>) -> AppResult<Vec<SelectedTransferFile>> {
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
        });
    }
    validate_transfer_limits(selected.len(), &sizes)?;
    Ok(selected)
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
            "single file size cannot exceed 500MB".to_string(),
        ));
    }
    let total = sizes.iter().try_fold(0_u64, |total, size| {
        total.checked_add(*size).ok_or_else(|| {
            AppError::InvalidInput("transfer total size is too large".to_string())
        })
    })?;
    if total > MAX_TRANSFER_TOTAL_SIZE {
        return Err(AppError::InvalidInput(
            "transfer total size cannot exceed 1GB".to_string(),
        ));
    }
    Ok(())
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

fn new_transfer_file(id: String, name: String, size: u64, sha256: String) -> FileTransferFile {
    FileTransferFile {
        id,
        name,
        size,
        sha256,
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

fn unique_save_path(save_dir: &Path, file_name: &str) -> PathBuf {
    let sanitized = sanitize_file_name(file_name);
    let candidate = save_dir.join(&sanitized);
    if !candidate.exists() {
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
        if !candidate.exists() {
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

fn download_request_target(transfer_id: &str, file_id: &str, token: &str) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    serializer
        .append_pair("transfer_id", transfer_id)
        .append_pair("file_id", file_id)
        .append_pair("token", token);
    format!("/file-transfer?{}", serializer.finish())
}

fn parse_download_query(target: &str) -> Option<(String, String, String)> {
    let url = url::Url::parse(&format!("http://copyshare{target}")).ok()?;
    if url.path() != "/file-transfer" {
        return None;
    }
    let mut transfer_id = None;
    let mut file_id = None;
    let mut token = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "transfer_id" => transfer_id = Some(value.to_string()),
            "file_id" => file_id = Some(value.to_string()),
            "token" => token = Some(value.to_string()),
            _ => {}
        }
    }
    Some((transfer_id?, file_id?, token?))
}

async fn read_http_target(stream: &mut TcpStream) -> AppResult<Option<String>> {
    let (status_line, _) = read_http_response_head_like(stream).await?;
    let mut parts = status_line.split_whitespace();
    match (parts.next(), parts.next()) {
        (Some("GET"), Some(target)) => Ok(Some(target.to_string())),
        _ => Ok(None),
    }
}

async fn read_http_response_head(stream: &mut TcpStream) -> AppResult<(String, Vec<u8>)> {
    read_http_response_head_like(stream).await
}

async fn read_http_response_head_like(stream: &mut TcpStream) -> AppResult<(String, Vec<u8>)> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = find_header_end(&bytes) {
            let body = bytes.split_off(index + 4);
            let header = String::from_utf8_lossy(&bytes[..index]).to_string();
            let first_line = header.lines().next().unwrap_or_default().to_string();
            return Ok((first_line, body));
        }
        if bytes.len() > 16 * 1024 {
            return Err(AppError::InvalidInput("HTTP header too large".to_string()));
        }
    }
    Err(AppError::InvalidInput("incomplete HTTP response".to_string()))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
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

#[cfg(target_os = "macos")]
fn open_folder_with_system_file_manager(path: &Path) -> AppResult<()> {
    std::process::Command::new("open").arg(path).spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_folder_with_system_file_manager(path: &Path) -> AppResult<()> {
    std::process::Command::new("xdg-open").arg(path).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use uuid::Uuid;

    use crate::models::{DeviceInfo, DeviceStatus, FileTransferStatus};

    use super::{
        advertised_download_endpoint, current_transfer_save_dir, pending_clipboard_file_content,
        sanitize_file_name, transfer_save_dir, trusted_transfer_peer, unique_save_path,
        validate_transfer_limits, DownloadClaimError, FileTransferManager, MAX_SINGLE_FILE_SIZE, MAX_TRANSFER_FILE_COUNT,
        MAX_TRANSFER_TOTAL_SIZE,
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
}
