//! HTTP 服务器实现
//!
//! 提供文件分享的 HTTP 服务，支持断点续传、传输加密和动态压缩

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, Path, State as AxumState},
    http::{header, HeaderMap, HeaderName, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use futures::Stream;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tauri::{AppHandle, Emitter};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;
use tokio_util::io::ReaderStream;
use super::models::{ShareState, ShareUploadRecord};
use crate::http_common::{
    self, HasCryptoSessions, ServerCapabilities, HTTP_CHUNK_SIZE,
};
use crate::models::FileMetadata;
use crate::transfer::compression::{
    create_compressor_from_config, get_compression_config, Compressor,
};
use crate::transfer::crypto::is_encryption_enabled;
use crate::transfer::http_crypto::HttpCryptoSessionManager;

#[derive(Debug)]
struct ChunkDownloadSession {
    upload_id: String,
    file_name: String,
    file_size: u64,
    chunk_count: usize,
    downloaded_chunks: HashSet<usize>,
    client_ip: String,
    start_time: std::time::Instant,
}

#[derive(Debug)]
pub struct ServerState {
    pub share_state: Arc<Mutex<ShareState>>,
    pub file_paths: Arc<Mutex<std::collections::HashMap<String, PathBuf>>>,
    pub hash_to_filename: Arc<Mutex<std::collections::HashMap<String, String>>>,
    pub app_handle: AppHandle,
    pub crypto_sessions: Arc<Mutex<HttpCryptoSessionManager>>,
    chunk_download_sessions: Arc<Mutex<std::collections::HashMap<String, ChunkDownloadSession>>>,
}

impl HasCryptoSessions for ServerState {
    fn crypto_sessions(&self) -> &Arc<Mutex<HttpCryptoSessionManager>> {
        &self.crypto_sessions
    }
}

pub struct ShareServer {
    pub addr: SocketAddr,
    pub state: Arc<ServerState>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ShareServer {
    pub fn new(share_state: Arc<Mutex<ShareState>>, app_handle: AppHandle, port: u16) -> Self {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        Self {
            addr,
            state: Arc::new(ServerState {
                share_state,
                file_paths: Arc::new(Mutex::new(std::collections::HashMap::new())),
                hash_to_filename: Arc::new(Mutex::new(std::collections::HashMap::new())),
                app_handle,
                crypto_sessions: Arc::new(Mutex::new(HttpCryptoSessionManager::new())),
                chunk_download_sessions: Arc::new(Mutex::new(std::collections::HashMap::new())),
            }),
            shutdown_tx: None,
        }
    }

    pub async fn start(&mut self, files: Vec<(FileMetadata, PathBuf)>) -> Result<u16, String> {
        {
            let mut file_paths = self.state.file_paths.lock().await;
            let mut hash_to_filename = self.state.hash_to_filename.lock().await;
            for (metadata, path) in files {
                let hash = Sha256::digest(path.to_string_lossy().as_bytes());
                let hash_id = hex::encode(hash);

                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&metadata.name)
                    .to_string();

                file_paths.insert(hash_id.clone(), path);
                hash_to_filename.insert(hash_id, file_name);
            }
        }

        let app = Router::new()
            .route("/", get(index_handler))
            .route("/favicon.ico", get(http_common::favicon_handler))
            .route("/apple-touch-icon.png", get(http_common::favicon_handler))
            .route("/apple-touch-icon-precomposed.png", get(http_common::favicon_handler))
            .route("/files", get(list_files_handler))
            .route("/verify-pin", post(verify_pin_handler))
            .route("/request-status", get(request_status_handler))
            .route("/capabilities", get(share_capabilities_handler))
            .route("/crypto/handshake", post(http_common::crypto_handshake_handler::<ServerState>))
            .route("/download/{file_id}/meta", get(download_meta_handler))
            .route(
                "/download/{file_id}/chunk/{chunk_index}",
                get(download_chunk_handler),
            )
            .route("/download/{file_id}", get(file_download_handler))
            .fallback(http_common::fallback_handler)
            .layer(http_common::share_cors_layer())
            .with_state(self.state.clone());

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| format!("Failed to bind port: {}", e))?;

        let actual_port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get port: {}", e))?
            .port();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        http_common::spawn_crypto_session_cleanup(self.state.crypto_sessions.clone());

        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .ok();
        });

        Ok(actual_port)
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

// ─── Helper functions ───────────────────────────────────────────────────────

fn parse_range(range_str: &str, file_size: u64) -> Option<(u64, u64)> {
    let range_str = range_str.strip_prefix("bytes=")?;
    let parts: Vec<&str> = range_str.splitn(2, '-').collect();
    if parts.len() != 2 {
        return None;
    }

    let start = if parts[0].is_empty() {
        let suffix_len: u64 = parts[1].parse().ok()?;
        file_size.saturating_sub(suffix_len)
    } else {
        parts[0].parse().ok()?
    };

    let end = if parts[1].is_empty() {
        file_size - 1
    } else {
        parts[1].parse::<u64>().ok()?.min(file_size - 1)
    };

    if start > end || start >= file_size {
        return None;
    }

    Some((start, end))
}

fn generate_etag(file_path: &std::path::Path, file_size: u64) -> String {
    let mtime = std::fs::metadata(file_path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let hash = Sha256::digest(format!("{}_{}", file_path.display(), mtime).as_bytes());
    format!("\"{}_{}_{}\"", &hex::encode(hash)[..8], file_size, mtime)
}

/// Check if current client IP has download access
async fn check_download_access(
    state: &Arc<ServerState>,
    client_ip: &str,
) -> Result<(), Response> {
    let share_state = state.share_state.lock().await;

    if share_state.share_info.is_none() {
        return Err(
            Html("<html><body><h1>分享已结束</h1></body></html>").into_response()
        );
    }

    if share_state.is_ip_rejected(client_ip) {
        return Err(
            Html("<html><body><h1>访问被拒绝</h1></body></html>").into_response()
        );
    }

    let has_pin = share_state.settings.pin.is_some()
        && !share_state
            .settings
            .pin
            .as_ref()
            .map_or(true, String::is_empty);
    let is_verified = share_state.is_ip_verified(client_ip);

    if has_pin && !is_verified {
        return Err(
            Html("<html><body><h1>需要验证 PIN</h1></body></html>").into_response()
        );
    }

    if !share_state.is_ip_allowed(client_ip) {
        return Err(
            Html("<html><body><h1>等待访问授权中，请稍后重试</h1></body></html>").into_response()
        );
    }

    Ok(())
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn share_capabilities_handler() -> Json<ServerCapabilities> {
    Json(ServerCapabilities::for_share())
}

/// Download metadata (chunk info for encrypted/compressed mode)
async fn download_meta_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<ServerState>>,
    Path(file_id): Path<String>,
) -> Response {
    let client_ip = client_addr.ip().to_string();
    if let Err(resp) = check_download_access(&state, &client_ip).await {
        return resp;
    }

    let file_path = {
        let file_paths = state.file_paths.lock().await;
        file_paths.get(&file_id).cloned()
    };

    let Some(path) = file_path else {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    };

    if !path.exists() || !path.is_file() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download")
        .to_string();
    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mime_type = FileMetadata::infer_mime_type(&file_name);

    let encryption = is_encryption_enabled();
    let compression_config = get_compression_config();
    let compression_active = compression_config.enabled
        && !Compressor::should_skip_compression(&mime_type);

    let chunk_count = ((file_size as f64) / (HTTP_CHUNK_SIZE as f64)).ceil() as usize;

    // When encryption or compression is active, the client will download via chunks
    // (not through upload_handler), so we need to track and emit events here.
    if encryption || compression_active {
        let upload_record = ShareUploadRecord::new(file_name.clone(), file_size);
        let upload_id = upload_record.id.clone();

        {
            let mut share_state = state.share_state.lock().await;
            if let Some(request) = share_state
                .access_requests
                .values_mut()
                .find(|r| r.ip == client_ip)
            {
                request.upload_records.insert(0, upload_record);
            }
        }

        let _ = state.app_handle.emit(
            "upload-start",
            UploadStartPayload {
                upload_id: upload_id.clone(),
                file_name: file_name.clone(),
                file_size: file_size as i64,
                client_ip: client_ip.clone(),
            },
        );

        let session_key = format!("{}_{}", file_id, client_ip);
        let mut sessions = state.chunk_download_sessions.lock().await;
        sessions.insert(
            session_key,
            ChunkDownloadSession {
                upload_id,
                file_name: file_name.clone(),
                file_size,
                chunk_count,
                downloaded_chunks: HashSet::new(),
                client_ip: client_ip.clone(),
                start_time: std::time::Instant::now(),
            },
        );
    }

    Json(DownloadMeta {
        file_id,
        file_name,
        file_size,
        chunk_size: HTTP_CHUNK_SIZE,
        chunk_count,
        encryption,
        compression: if compression_active {
            Some("zstd".to_string())
        } else {
            None
        },
        mime_type,
    })
    .into_response()
}

/// Download a single processed chunk (compressed + encrypted)
async fn download_chunk_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<ServerState>>,
    Path((file_id, chunk_index)): Path<(String, usize)>,
    headers: HeaderMap,
) -> Response {
    let client_ip = client_addr.ip().to_string();
    if let Err(resp) = check_download_access(&state, &client_ip).await {
        return resp;
    }

    let file_path = {
        let file_paths = state.file_paths.lock().await;
        file_paths.get(&file_id).cloned()
    };

    let Some(path) = file_path else {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    };

    if !path.exists() || !path.is_file() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("download")
        .to_string();
    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mime_type = FileMetadata::infer_mime_type(&file_name);

    // Read the chunk
    let buffer = match read_file_chunk(&path, chunk_index, file_size).await {
        Ok(data) => data,
        Err(resp) => return resp,
    };

    let original_size = buffer.len();

    // Pipeline: compress (optional) → encrypt (optional)
    let (data, compressed) = apply_compression_pipeline(buffer, &mime_type);
    let (data, encrypted) = apply_encryption_pipeline(data, &headers, &state.crypto_sessions).await;

    let mut response = Response::new(Body::from(data));
    *response.status_mut() = StatusCode::OK;
    let resp_headers = response.headers_mut();
    resp_headers.insert(
        HeaderName::from_static("x-chunk-index"),
        chunk_index.to_string().parse().unwrap(),
    );
    resp_headers.insert(
        HeaderName::from_static("x-original-size"),
        original_size.to_string().parse().unwrap(),
    );
    if compressed {
        resp_headers.insert(
            HeaderName::from_static("x-compression"),
            "zstd".parse().unwrap(),
        );
    }
    if encrypted {
        resp_headers.insert(
            HeaderName::from_static("x-encryption"),
            "aes-256-gcm".parse().unwrap(),
        );
    }

    // Track chunk download progress and emit events
    let session_key = format!("{}_{}", file_id, client_ip);
    let mut sessions = state.chunk_download_sessions.lock().await;
    if let Some(session) = sessions.get_mut(&session_key) {
        session.downloaded_chunks.insert(chunk_index);

        let downloaded = session.downloaded_chunks.len();
        let total = session.chunk_count;
        let progress = (downloaded as f64 / total as f64) * 100.0;
        let elapsed_secs = session.start_time.elapsed().as_secs_f64();
        let downloaded_bytes = (downloaded as u64).min(total as u64) * HTTP_CHUNK_SIZE as u64;
        let downloaded_bytes = downloaded_bytes.min(session.file_size);
        let speed = if elapsed_secs > 0.0 {
            (downloaded_bytes as f64 / elapsed_secs) as u64
        } else {
            0
        };

        let _ = state.app_handle.emit(
            "upload-progress",
            super::models::UploadProgress {
                upload_id: session.upload_id.clone(),
                file_name: session.file_name.clone(),
                progress,
                uploaded_bytes: downloaded_bytes,
                total_bytes: session.file_size,
                speed,
                client_ip: session.client_ip.clone(),
            },
        );

        // Update the upload record in share state
        {
            let upload_id = session.upload_id.clone();
            let file_size = session.file_size;
            let mut share_state = state.share_state.lock().await;
            for request in share_state.access_requests.values_mut() {
                if let Some(record) = request.upload_records.iter_mut().find(|r| r.id == upload_id)
                {
                    record.uploaded_bytes = downloaded_bytes;
                    record.progress = progress;
                    record.speed = speed;
                    break;
                }
            }

            if downloaded >= total {
                for request in share_state.access_requests.values_mut() {
                    if let Some(record) =
                        request.upload_records.iter_mut().find(|r| r.id == upload_id)
                    {
                        record.uploaded_bytes = file_size;
                        record.progress = 100.0;
                        record.speed = 0;
                        record.status = super::models::TransferStatus::Completed;
                        record.completed_at = Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                        );
                        break;
                    }
                }
            }
        }

        if downloaded >= total {
            let _ = state.app_handle.emit(
                "upload-complete",
                UploadCompletePayload {
                    upload_id: session.upload_id.clone(),
                    file_name: session.file_name.clone(),
                    file_size: session.file_size as i64,
                    client_ip: session.client_ip.clone(),
                },
            );
            sessions.remove(&session_key);
        }
    }

    response
}

/// Handle new visitor access request creation and auto-accept logic
/// Returns whether the visitor has been granted access
fn handle_new_visitor(
    share_state: &mut super::models::ShareState,
    client_ip: &str,
    user_agent: &str,
    app_handle: &AppHandle,
) -> bool {
    let has_request = share_state
        .access_requests
        .values()
        .any(|r| r.ip == client_ip);

    if !has_request {
        let mut new_request =
            super::models::AccessRequest::new(client_ip.to_string(), Some(user_agent.to_string()));

        if share_state.settings.auto_accept {
            new_request.status = super::models::AccessRequestStatus::Accepted;
        }

        share_state
            .access_requests
            .insert(new_request.id.clone(), new_request.clone());

        let _ = app_handle.emit("access-request", new_request.clone());

        if new_request.status == super::models::AccessRequestStatus::Accepted {
            if !share_state.verified_ips.contains(&client_ip.to_string()) {
                share_state.verified_ips.push(client_ip.to_string());
            }

            let _ = app_handle.emit("access-request-accepted", new_request);
            return true;
        }
    }

    false
}

/// Index handler
async fn index_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumState(state): AxumState<Arc<ServerState>>,
) -> Response {
    let client_ip = client_addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| http_common::parse_user_agent(s))
        .unwrap_or_default();

    let accept_language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("zh-CN");
    let is_english = accept_language.starts_with("en");

    {
        let share_state = state.share_state.lock().await;
        if share_state.share_info.is_none() {
            return Html(generate_share_ended_html(is_english)).into_response();
        }
    }

    {
        let share_state = state.share_state.lock().await;
        if share_state.is_ip_rejected(&client_ip) {
            return Html(generate_access_denied_html(is_english)).into_response();
        }
    }

    {
        let mut share_state = state.share_state.lock().await;

        let has_pin = share_state.settings.pin.is_some()
            && !share_state
                .settings
                .pin
                .as_ref()
                .map_or(true, String::is_empty);
        let is_verified = share_state.is_ip_verified(&client_ip);
        let has_access = share_state.is_ip_allowed(&client_ip);

        if has_pin && !is_verified && !has_access {
            let pin_attempt = share_state.pin_attempts.get(&client_ip).cloned();

            if let Some(attempt) = &pin_attempt {
                if attempt.is_still_locked() {
                    let remaining_ms = attempt.remaining_lock_time();
                    let remaining_secs = remaining_ms / 1000;
                    let locked_html = generate_locked_html(remaining_secs, is_english);
                    return Html(locked_html).into_response();
                }
            }

            return Html(generate_pin_input_html(is_english)).into_response();
        }

        if !has_pin {
            let granted_access = handle_new_visitor(&mut share_state, &client_ip, &user_agent, &state.app_handle);
            if !granted_access && !share_state.is_ip_allowed(&client_ip) {
                return Html(generate_waiting_response_html(is_english)).into_response();
            }
        }

        if !share_state.is_ip_allowed(&client_ip) {
            return Html(generate_waiting_response_html(is_english)).into_response();
        }
    }

    let share_state = state.share_state.lock().await;
    let has_access = share_state.is_ip_allowed(&client_ip);

    if !has_access {
        return Html(generate_waiting_response_html(is_english)).into_response();
    }

    let html = generate_file_list_html(is_english);
    Html(html).into_response()
}

/// File list API
async fn list_files_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<ServerState>>,
) -> impl IntoResponse {
    let share_state = state.share_state.lock().await;

    if share_state.share_info.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(FilesResponse {
                files: vec![],
                waiting_response: None,
            }),
        );
    }

    let client_ip = client_addr.ip().to_string();

    if share_state.is_ip_rejected(&client_ip) {
        return (
            StatusCode::FORBIDDEN,
            Json(FilesResponse {
                files: vec![],
                waiting_response: None,
            }),
        );
    }

    let has_pin = share_state.settings.pin.is_some()
        && !share_state
            .settings
            .pin
            .as_ref()
            .map_or(true, String::is_empty);
    let is_verified = share_state.is_ip_verified(&client_ip);
    let has_request = share_state
        .access_requests
        .values()
        .any(|r| r.ip == client_ip);
    let needs_pin = has_pin && !is_verified && !has_request;

    if needs_pin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(FilesResponse {
                files: vec![],
                waiting_response: None,
            }),
        );
    }

    let has_access = share_state.is_ip_allowed(&client_ip);

    if !has_access {
        return (
            StatusCode::ACCEPTED,
            Json(FilesResponse {
                files: vec![],
                waiting_response: Some(true),
            }),
        );
    }

    let share_info = share_state.share_info.as_ref().unwrap();
    let hash_to_filename = state.hash_to_filename.lock().await;
    let files: Vec<FileInfo> = hash_to_filename
        .iter()
        .map(|(hash_id, file_name)| {
            let file_size = share_info
                .files
                .iter()
                .find(|f| f.name == *file_name)
                .map(|f| f.size)
                .unwrap_or(0);
            let mime_type = share_info
                .files
                .iter()
                .find(|f| f.name == *file_name)
                .map(|f| f.mime_type.clone())
                .unwrap_or_else(|| "application/octet-stream".to_string());
            FileInfo {
                id: hash_id.clone(),
                name: file_name.clone(),
                size: file_size,
                mime_type,
            }
        })
        .collect();

    (
        StatusCode::OK,
        Json(FilesResponse {
            files,
            waiting_response: None,
        }),
    )
}

/// PIN verification
#[derive(Debug, Deserialize)]
struct VerifyPinRequest {
    pin: String,
}

async fn verify_pin_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumState(state): AxumState<Arc<ServerState>>,
    Json(payload): Json<VerifyPinRequest>,
) -> impl IntoResponse {
    let client_ip = client_addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| http_common::parse_user_agent(s).to_string());
    let mut share_state = state.share_state.lock().await;

    if let Some(attempt) = share_state.pin_attempts.get(&client_ip) {
        if attempt.is_still_locked() {
            return (
                StatusCode::FORBIDDEN,
                Json(super::models::PinVerifyResult {
                    success: false,
                    remaining_attempts: Some(0),
                    locked: true,
                    locked_until: attempt.locked_until,
                }),
            );
        }
    }

    let correct_pin = match &share_state.settings.pin {
        Some(pin) if !pin.is_empty() => pin,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(super::models::PinVerifyResult {
                    success: false,
                    remaining_attempts: None,
                    locked: false,
                    locked_until: None,
                }),
            );
        }
    };

    if payload.pin == *correct_pin {
        share_state.pin_attempts.remove(&client_ip);

        if !share_state.verified_ips.contains(&client_ip) {
            share_state.verified_ips.push(client_ip.clone());
        }

        let mut new_request = super::models::AccessRequest::new(client_ip.clone(), user_agent);

        if share_state.settings.auto_accept {
            new_request.status = super::models::AccessRequestStatus::Accepted;
        }

        share_state
            .access_requests
            .insert(new_request.id.clone(), new_request.clone());

        let _ = state.app_handle.emit("access-request", new_request.clone());
        if new_request.status == super::models::AccessRequestStatus::Accepted {
            let _ = state
                .app_handle
                .emit("access-request-accepted", new_request);
        }

        (
            StatusCode::OK,
            Json(super::models::PinVerifyResult {
                success: true,
                remaining_attempts: None,
                locked: false,
                locked_until: None,
            }),
        )
    } else {
        let attempt = share_state
            .pin_attempts
            .entry(client_ip.clone())
            .or_insert_with(|| super::models::PinAttemptState::new(client_ip.clone()));

        attempt.record_failure();

        let remaining = 3u32.saturating_sub(attempt.attempts);

        (
            StatusCode::UNAUTHORIZED,
            Json(super::models::PinVerifyResult {
                success: false,
                remaining_attempts: Some(remaining),
                locked: attempt.locked,
                locked_until: attempt.locked_until,
            }),
        )
    }
}

/// Request status handler
async fn request_status_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumState(state): AxumState<Arc<ServerState>>,
) -> impl IntoResponse {
    let client_ip = client_addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| http_common::parse_user_agent(s))
        .unwrap_or_default();
    let mut share_state = state.share_state.lock().await;

    let request = share_state
        .access_requests
        .values()
        .find(|r| r.ip == client_ip);

    let response = match request {
        Some(req) => {
            let status_str = match req.status {
                super::models::AccessRequestStatus::Pending => "pending",
                super::models::AccessRequestStatus::Accepted => "accepted",
                super::models::AccessRequestStatus::Rejected => "rejected",
            };
            RequestStatusResponse {
                has_request: true,
                status: Some(status_str.to_string()),
                waiting_response: req.status == super::models::AccessRequestStatus::Pending,
            }
        }
        None => {
            let auto_accept = share_state.settings.auto_accept;
            let has_pin = share_state.settings.pin.is_some()
                && !share_state
                    .settings
                    .pin
                    .as_ref()
                    .map_or(true, String::is_empty);
            let is_verified = share_state.is_ip_verified(&client_ip);

            if auto_accept && !has_pin && !is_verified {
                let mut new_request = super::models::AccessRequest::new(
                    client_ip.clone(),
                    Some(user_agent.to_string()),
                );
                new_request.status = super::models::AccessRequestStatus::Accepted;
                share_state
                    .access_requests
                    .insert(new_request.id.clone(), new_request.clone());

                if !share_state.verified_ips.contains(&client_ip) {
                    share_state.verified_ips.push(client_ip.clone());
                }

                let _ = state.app_handle.emit("access-request", new_request.clone());
                let _ = state
                    .app_handle
                    .emit("access-request-accepted", new_request);

                RequestStatusResponse {
                    has_request: true,
                    status: Some("accepted".to_string()),
                    waiting_response: false,
                }
            } else if is_verified {
                RequestStatusResponse {
                    has_request: true,
                    status: Some("accepted".to_string()),
                    waiting_response: false,
                }
            } else {
                RequestStatusResponse {
                    has_request: false,
                    status: None,
                    waiting_response: false,
                }
            }
        }
    };

    (StatusCode::OK, Json(response))
}

/// Build a Range partial content response
async fn build_range_response(
    path: &std::path::Path,
    file_name: &str,
    file_size: u64,
    start: u64,
    end: u64,
    mime_type: &str,
    etag: &str,
) -> Response {
    let content_length = end - start + 1;

    match File::open(path).await {
        Ok(mut file) => {
            if let Err(e) = file.seek(std::io::SeekFrom::Start(start)).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Seek failed: {}", e),
                )
                    .into_response();
            }

            let limited = file.take(content_length);
            let stream = ReaderStream::new(limited);
            let body = Body::from_stream(stream);

            let mut response = Response::new(body);
            *response.status_mut() = StatusCode::PARTIAL_CONTENT;
            let resp_headers = response.headers_mut();
            resp_headers.insert(
                header::CONTENT_RANGE,
                format!("bytes {}-{}/{}", start, end, file_size)
                    .parse()
                    .unwrap(),
            );
            resp_headers.insert(
                header::CONTENT_LENGTH,
                content_length.to_string().parse().unwrap(),
            );
            resp_headers.insert(
                header::ACCEPT_RANGES,
                "bytes".parse().unwrap(),
            );
            resp_headers.insert(header::ETAG, etag.parse().unwrap());
            if let Ok(mime_header) = mime_type.parse() {
                resp_headers.insert(header::CONTENT_TYPE, mime_header);
            }
            let encoded_filename = urlencoding::encode(file_name);
            resp_headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename*=UTF-8''{}", encoded_filename)
                    .parse()
                    .unwrap(),
            );

            response
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Open file failed: {}", e),
        )
            .into_response(),
    }
}

/// Build a full file download response with progress tracking stream
async fn build_full_download_response(
    path: &std::path::Path,
    file_name: &str,
    file_size: u64,
    mime_type: &str,
    etag: &str,
    state: &Arc<ServerState>,
    upload_id: String,
    client_ip: String,
) -> Response {
    match File::open(path).await {
        Ok(file) => {
            let reader_stream = ReaderStream::new(file);
            let progress_stream = ProgressTrackingStream::new(
                reader_stream,
                state.app_handle.clone(),
                state.share_state.clone(),
                upload_id.clone(),
                file_name.to_string(),
                client_ip,
                file_size,
            );
            let body = Body::from_stream(progress_stream);

            let mut response = Response::new(body);
            *response.status_mut() = StatusCode::OK;
            let resp_headers = response.headers_mut();
            if let Ok(mime_header) = mime_type.parse() {
                resp_headers.insert(header::CONTENT_TYPE, mime_header);
            } else {
                resp_headers.insert(
                    header::CONTENT_TYPE,
                    "application/octet-stream".parse().unwrap(),
                );
            }
            let encoded_filename = urlencoding::encode(file_name);
            resp_headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename*=UTF-8''{}", encoded_filename)
                    .parse()
                    .unwrap(),
            );
            resp_headers.insert(
                header::CONTENT_LENGTH,
                file_size.to_string().parse().unwrap(),
            );
            resp_headers.insert(
                header::ACCEPT_RANGES,
                "bytes".parse().unwrap(),
            );
            resp_headers.insert(header::ETAG, etag.parse().unwrap());

            response
        }
        Err(e) => {
            let mut share_state = state.share_state.lock().await;
            for request in share_state.access_requests.values_mut() {
                if let Some(record) = request
                    .upload_records
                    .iter_mut()
                    .find(|r| r.id == upload_id)
                {
                    record.status = super::models::TransferStatus::Failed;
                    break;
                }
            }
            let error_html =
                format!("<html><body><h1>Failed to open file: {}</h1></body></html>", e);
            Html(error_html).into_response()
        }
    }
}

/// File download handler with Range support
async fn file_download_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<ServerState>>,
    Path(file_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let client_ip = client_addr.ip().to_string();

    if let Err(resp) = check_download_access(&state, &client_ip).await {
        return resp;
    }

    let file_path = {
        let file_paths = state.file_paths.lock().await;
        file_paths.get(&file_id).cloned()
    };

    match file_path {
        Some(path) => {
            if !path.exists() || !path.is_file() {
                return Html("<html><body><h1>文件不存在</h1></body></html>").into_response();
            }

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download")
                .to_string();

            let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let mime_type = FileMetadata::infer_mime_type(&file_name);
            let etag = generate_etag(&path, file_size);

            // Check If-None-Match for caching
            if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
                if if_none_match.to_str().ok() == Some(&etag) {
                    return StatusCode::NOT_MODIFIED.into_response();
                }
            }

            let upload_record = ShareUploadRecord::new(file_name.clone(), file_size);
            let upload_id = upload_record.id.clone();
            {
                let mut share_state = state.share_state.lock().await;
                if let Some(request) = share_state
                    .access_requests
                    .values_mut()
                    .find(|r| r.ip == client_ip)
                {
                    request.upload_records.insert(0, upload_record);
                }
            }

            let _ = state.app_handle.emit(
                "upload-start",
                UploadStartPayload {
                    upload_id: upload_id.clone(),
                    file_name: file_name.clone(),
                    file_size: file_size as i64,
                    client_ip: client_ip.clone(),
                },
            );

            // Check for Range request (plaintext mode)
            let range_header = headers
                .get(header::RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| parse_range(s, file_size));

            if let Some((start, end)) = range_header {
                return build_range_response(&path, &file_name, file_size, start, end, &mime_type, &etag).await;
            }

            // Full file download with progress tracking
            build_full_download_response(
                &path,
                &file_name,
                file_size,
                &mime_type,
                &etag,
                &state,
                upload_id,
                client_ip,
            )
            .await
        }
        None => {
            Html("<html><body><h1>文件不存在</h1></body></html>").into_response()
        }
    }
}

// ─── Helper functions for download_chunk_handler ─────────────────────────────

async fn read_file_chunk(
    path: &std::path::Path,
    chunk_index: usize,
    file_size: u64,
) -> Result<Vec<u8>, Response> {
    let offset = chunk_index as u64 * HTTP_CHUNK_SIZE as u64;
    if offset >= file_size {
        return Err(
            (StatusCode::BAD_REQUEST, "Chunk index out of range").into_response()
        );
    }
    let remaining = file_size - offset;
    let read_size = (remaining as usize).min(HTTP_CHUNK_SIZE);

    let mut file = match File::open(path).await {
        Ok(f) => f,
        Err(e) => {
            return Err(
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Open file failed: {}", e),
                )
                    .into_response()
            )
        }
    };

    if let Err(e) = file.seek(std::io::SeekFrom::Start(offset)).await {
        return Err(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Seek failed: {}", e),
            )
                .into_response()
        );
    }

    let mut buffer = vec![0u8; read_size];
    if let Err(e) = file.read_exact(&mut buffer).await {
        return Err(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Read failed: {}", e),
            )
                .into_response()
        );
    }

    Ok(buffer)
}

fn apply_compression_pipeline(data: Vec<u8>, mime_type: &str) -> (Vec<u8>, bool) {
    let compression_config = get_compression_config();
    let mut compressed = false;
    let mut result_data = data;

    if compression_config.enabled {
        if let Some(compressor) = create_compressor_from_config() {
            if let Some(level) = compressor.get_level(mime_type) {
                if let Ok(compressed_data) = Compressor::compress(&result_data, level) {
                    if compressed_data.len() < result_data.len() {
                        result_data = compressed_data;
                        compressed = true;
                    }
                }
            }
        }
    }

    (result_data, compressed)
}

async fn apply_encryption_pipeline(
    data: Vec<u8>,
    headers: &HeaderMap,
    crypto_sessions: &Arc<Mutex<HttpCryptoSessionManager>>,
) -> (Vec<u8>, bool) {
    let encryption_enabled = is_encryption_enabled();
    let mut encrypted = false;
    let mut result_data = data;

    if encryption_enabled {
        let session_id = headers
            .get("x-encryption-session")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !session_id.is_empty() {
            let mut crypto_sessions = crypto_sessions.lock().await;
            if let Some(session) = crypto_sessions.get_session_mut(session_id) {
                match session.encrypt(&result_data) {
                    Ok(encrypted_data) => {
                        result_data = encrypted_data;
                        encrypted = true;
                    }
                    Err(e) => {
                        eprintln!("Chunk encryption failed: {}", e);
                    }
                }
            }
        }
    }

    (result_data, encrypted)
}

// ─── Data types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
struct UploadStartPayload {
    upload_id: String,
    file_name: String,
    file_size: i64,
    client_ip: String,
}

#[derive(Debug, Clone, Serialize)]
struct UploadCompletePayload {
    upload_id: String,
    file_name: String,
    file_size: i64,
    client_ip: String,
}


#[derive(Debug, Serialize)]
struct DownloadMeta {
    file_id: String,
    file_name: String,
    file_size: u64,
    chunk_size: usize,
    chunk_count: usize,
    encryption: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    compression: Option<String>,
    mime_type: String,
}

#[derive(Debug, Serialize)]
struct FilesResponse {
    files: Vec<FileInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    waiting_response: Option<bool>,
}

#[derive(Debug, Serialize)]
struct FileInfo {
    id: String,
    name: String,
    size: u64,
    mime_type: String,
}

#[derive(Debug, Serialize)]
struct RequestStatusResponse {
    has_request: bool,
    status: Option<String>,
    waiting_response: bool,
}

// ─── Progress tracking stream ───────────────────────────────────────────────

struct ProgressTrackingStream {
    inner: ReaderStream<File>,
    app_handle: AppHandle,
    share_state: Arc<Mutex<ShareState>>,
    upload_id: String,
    file_name: String,
    client_ip: String,
    total_bytes: u64,
    transferred_bytes: u64,
    last_emit_time: std::time::Instant,
    last_emit_progress: f64,
    start_time: std::time::Instant,
}

impl ProgressTrackingStream {
    fn new(
        inner: ReaderStream<File>,
        app_handle: AppHandle,
        share_state: Arc<Mutex<ShareState>>,
        upload_id: String,
        file_name: String,
        client_ip: String,
        total_bytes: u64,
    ) -> Self {
        Self {
            inner,
            app_handle,
            share_state,
            upload_id,
            file_name,
            client_ip,
            total_bytes,
            transferred_bytes: 0,
            last_emit_time: std::time::Instant::now(),
            last_emit_progress: 0.0,
            start_time: std::time::Instant::now(),
        }
    }

    fn calculate_speed(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            (self.transferred_bytes as f64 / elapsed) as u64
        } else {
            0
        }
    }

    fn should_emit_progress(&self, current_progress: f64) -> bool {
        let time_elapsed = self.last_emit_time.elapsed() >= std::time::Duration::from_millis(500);
        let progress_changed = (current_progress - self.last_emit_progress) >= 1.0;
        time_elapsed || progress_changed
    }

    fn emit_progress(&mut self, progress: f64, speed: u64) {
        let payload = super::models::UploadProgress {
            upload_id: self.upload_id.clone(),
            file_name: self.file_name.clone(),
            progress,
            uploaded_bytes: self.transferred_bytes,
            total_bytes: self.total_bytes,
            speed,
            client_ip: self.client_ip.clone(),
        };
        let _ = self.app_handle.emit("upload-progress", payload);
        self.last_emit_time = std::time::Instant::now();
        self.last_emit_progress = progress;
    }

    fn emit_complete(&self) {
        let speed = self.calculate_speed();
        let payload = super::models::UploadProgress {
            upload_id: self.upload_id.clone(),
            file_name: self.file_name.clone(),
            progress: 100.0,
            uploaded_bytes: self.total_bytes,
            total_bytes: self.total_bytes,
            speed,
            client_ip: self.client_ip.clone(),
        };
        let _ = self.app_handle.emit("upload-progress", payload);

        let _ = self.app_handle.emit(
            "upload-complete",
            UploadCompletePayload {
                upload_id: self.upload_id.clone(),
                file_name: self.file_name.clone(),
                file_size: self.total_bytes as i64,
                client_ip: self.client_ip.clone(),
            },
        );
    }
}

impl Stream for ProgressTrackingStream {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { self.get_unchecked_mut() };
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };

        match inner.poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                this.transferred_bytes += chunk.len() as u64;

                let progress = if this.total_bytes > 0 {
                    (this.transferred_bytes as f64 / this.total_bytes as f64) * 100.0
                } else {
                    0.0
                };

                let speed = this.calculate_speed();

                if this.should_emit_progress(progress) {
                    this.emit_progress(progress, speed);

                    let share_state = this.share_state.clone();
                    let upload_id = this.upload_id.clone();
                    let transferred = this.transferred_bytes;
                    let prog = progress;
                    let spd = speed;
                    tokio::spawn(async move {
                        let mut state = share_state.lock().await;
                        for request in state.access_requests.values_mut() {
                            if let Some(record) = request
                                .upload_records
                                .iter_mut()
                                .find(|r| r.id == upload_id)
                            {
                                record.uploaded_bytes = transferred;
                                record.progress = prog;
                                record.speed = spd;
                                break;
                            }
                        }
                    });
                }

                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            Poll::Ready(None) => {
                this.transferred_bytes = this.total_bytes;
                this.emit_complete();

                let share_state = this.share_state.clone();
                let upload_id = this.upload_id.clone();
                tokio::spawn(async move {
                    let mut state = share_state.lock().await;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    for request in state.access_requests.values_mut() {
                        if let Some(record) = request
                            .upload_records
                            .iter_mut()
                            .find(|r| r.id == upload_id)
                        {
                            record.uploaded_bytes = record.total_bytes;
                            record.progress = 100.0;
                            record.status = super::models::TransferStatus::Completed;
                            record.completed_at = Some(now);
                            break;
                        }
                    }
                });

                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}


/// Structure to hold internationalized labels for the file list page
#[derive(Debug, Clone)]
struct FileListPageLabels {
    /// Label for downloading status
    pub downloading: String,
    /// Label for download complete status
    pub download_complete: String,
    /// Label for download failed status
    pub download_failed: String,
    /// Label for encrypted files
    pub encrypted_label: String,
    /// Label for compressed files
    pub compressed_label: String,
    /// Label when no files are available
    pub no_files: String,
}

/// Returns the CSS styles for the file list page
fn file_list_page_css() -> &'static str {
    r#"        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }
        h1 { color: #333; }
        ul { list-style: none; padding: 0; }
        li { padding: 12px; border-bottom: 1px solid #eee; display: flex; align-items: center; justify-content: space-between; }
        a { color: #1976d2; text-decoration: none; cursor: pointer; }
        a:hover { text-decoration: underline; }
        .warning { background: #fff3cd; padding: 10px; border-radius: 4px; margin-bottom: 20px; }
        .empty { color: #999; text-align: center; padding: 40px 0; }
        .badges { display: flex; gap: 6px; margin-left: 10px; }
        .badge { font-size: 11px; padding: 2px 6px; border-radius: 4px; color: #fff; }
        .badge-enc { background: #2e7d32; }
        .badge-comp { background: #1565c0; }
        .progress-bar { width: 100%; height: 4px; background: #e0e0e0; border-radius: 2px; margin-top: 6px; overflow: hidden; }
        .progress-fill { height: 100%; background: #1976d2; transition: width 0.3s; }
        .progress-text { font-size: 12px; color: #666; margin-top: 4px; }
        .file-info { flex: 1; }
        .file-size { color: #888; font-size: 13px; margin-left: 8px; }"#
}

/// Returns the JavaScript code for the file list page with internationalized labels
fn file_list_page_javascript(labels: &FileListPageLabels) -> String {
    format!(
        r#"
        var caps = null;
        var cryptoKey = null;
        var sessionId = null;

        function formatSize(bytes) {{
            if (bytes === 0) return '0 B';
            var units = ['B', 'KB', 'MB', 'GB', 'TB'];
            var i = Math.floor(Math.log(bytes) / Math.log(1024));
            return (bytes / Math.pow(1024, i)).toFixed(2) + ' ' + units[i];
        }}

        async function initEnhanced() {{
            try {{
                var resp = await fetch('/capabilities');
                caps = await resp.json();
                if (caps.encryption) {{
                    await performHandshake();
                }}
            }} catch(e) {{
                console.warn('Enhanced transfer init failed:', e);
                caps = {{ encryption: false, compression: false }};
            }}
        }}

        async function performHandshake() {{
            try {{
                var keyPair = await crypto.subtle.generateKey(
                    {{ name: 'ECDH', namedCurve: 'P-256' }},
                    true,
                    ['deriveBits']
                );
                var pubRaw = await crypto.subtle.exportKey('raw', keyPair.publicKey);
                var pubB64 = btoa(String.fromCharCode.apply(null, new Uint8Array(pubRaw)));

                var resp = await fetch('/crypto/handshake', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ client_public_key: pubB64 }})
                }});
                var result = await resp.json();
                if (!result.encryption) return;

                sessionId = result.session_id;

                var serverPubBytes = Uint8Array.from(atob(result.server_public_key), function(c) {{ return c.charCodeAt(0); }});
                var serverPubKey = await crypto.subtle.importKey(
                    'raw', serverPubBytes,
                    {{ name: 'ECDH', namedCurve: 'P-256' }},
                    false, []
                );

                var sharedBits = await crypto.subtle.deriveBits(
                    {{ name: 'ECDH', public: serverPubKey }},
                    keyPair.privateKey, 256
                );

                var hkdfKey = await crypto.subtle.importKey('raw', sharedBits, 'HKDF', false, ['deriveKey']);
                cryptoKey = await crypto.subtle.deriveKey(
                    {{
                        name: 'HKDF', hash: 'SHA-256',
                        salt: new Uint8Array(0),
                        info: new TextEncoder().encode('puresend-http-encryption')
                    }},
                    hkdfKey,
                    {{ name: 'AES-GCM', length: 256 }},
                    false, ['decrypt']
                );
            }} catch(e) {{
                console.warn('Handshake failed:', e);
                caps.encryption = false;
            }}
        }}

        async function decryptChunk(data) {{
            var nonce = data.slice(0, 12);
            var ciphertext = data.slice(12);
            var decrypted = await crypto.subtle.decrypt(
                {{ name: 'AES-GCM', iv: nonce }},
                cryptoKey, ciphertext
            );
            return new Uint8Array(decrypted);
        }}

        async function downloadDirect(fileId, fileName, fileSize) {{
            var li = document.getElementById('dl-' + fileId);
            var progressBar = li.querySelector('.progress-fill');
            var progressText = li.querySelector('.progress-text');
            if (progressBar) progressBar.style.width = '0%';
            if (progressText) progressText.textContent = '{}';

            try {{
                var resp = await fetch('/download/' + fileId);
                var contentLength = parseInt(resp.headers.get('Content-Length') || fileSize);
                var reader = resp.body.getReader();
                var chunks = [];
                var received = 0;

                while (true) {{
                    var result = await reader.read();
                    if (result.done) break;
                    chunks.push(result.value);
                    received += result.value.length;
                    var pct = contentLength > 0 ? Math.min(100, Math.round(received / contentLength * 100)) : 0;
                    if (progressBar) progressBar.style.width = pct + '%';
                    if (progressText) progressText.textContent = pct + '% (' + formatSize(received) + ' / ' + formatSize(contentLength) + ')';
                }}

                var blob = new Blob(chunks);
                var url = URL.createObjectURL(blob);
                var a = document.createElement('a');
                a.href = url;
                a.download = fileName;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);

                if (progressBar) {{ progressBar.style.width = '100%'; progressBar.style.background = '#4caf50'; }}
                if (progressText) progressText.textContent = '{}';
            }} catch(e) {{
                console.error('Download failed:', e);
                if (progressText) {{ progressText.textContent = '{}: ' + e.message; progressText.style.color = '#d32f2f'; }}
            }}
        }}

        async function downloadEnhanced(fileId, fileName, fileSize) {{
            var li = document.getElementById('dl-' + fileId);
            var progressBar = li.querySelector('.progress-fill');
            var progressText = li.querySelector('.progress-text');
            if (progressBar) progressBar.style.width = '0%';
            if (progressText) progressText.textContent = '{}';

            try {{
                var metaResp = await fetch('/download/' + fileId + '/meta');
                var meta = await metaResp.json();

                if (!meta.encryption && !meta.compression) {{
                    await downloadDirect(fileId, fileName, fileSize);
                    return;
                }}

                var chunks = [];
                var downloaded = 0;

                for (var i = 0; i < meta.chunk_count; i++) {{
                    var headers = {{}};
                    if (sessionId) headers['X-Encryption-Session'] = sessionId;

                    var resp = await fetch('/download/' + fileId + '/chunk/' + i, {{ headers: headers }});
                    var data = new Uint8Array(await resp.arrayBuffer());

                    var isEncrypted = resp.headers.get('x-encryption') === 'aes-256-gcm';
                    if (isEncrypted && cryptoKey) {{
                        data = await decryptChunk(data);
                    }}

                    chunks.push(data);
                    downloaded += data.length;

                    var pct = Math.min(100, Math.round(downloaded / meta.file_size * 100));
                    if (progressBar) progressBar.style.width = pct + '%';
                    if (progressText) progressText.textContent = pct + '% (' + formatSize(downloaded) + ' / ' + formatSize(meta.file_size) + ')';
                }}

                var blob = new Blob(chunks);
                var url = URL.createObjectURL(blob);
                var a = document.createElement('a');
                a.href = url;
                a.download = fileName;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);

                if (progressBar) {{ progressBar.style.width = '100%'; progressBar.style.background = '#4caf50'; }}
                if (progressText) progressText.textContent = '{}';
            }} catch(e) {{
                console.error('Download failed:', e);
                if (progressText) {{
                    progressText.textContent = '{}: ' + e.message;
                    progressText.style.color = '#d32f2f';
                }}
            }}
        }}

        function downloadFile(fileId, fileName, fileSize) {{
            if (caps && (caps.encryption || caps.compression)) {{
                downloadEnhanced(fileId, fileName, fileSize);
            }} else {{
                downloadDirect(fileId, fileName, fileSize);
            }}
        }}

        var lastJson = '';
        function refreshFiles() {{
            fetch('/files')
                .then(function(r) {{ return r.json(); }})
                .then(function(data) {{
                    var json = JSON.stringify(data.files);
                    if (json === lastJson) return;
                    lastJson = json;
                    var ul = document.getElementById('file-list');
                    if (!data.files || data.files.length === 0) {{
                        ul.innerHTML = '<li class="empty">{}</li>';
                        return;
                    }}
                    ul.innerHTML = data.files.map(function(f) {{
                        var badges = '';
                        if (caps && caps.encryption) badges += '<span class="badge badge-enc">{}</span>';
                        if (caps && caps.compression) badges += '<span class="badge badge-comp">{}</span>';
                        return '<li id="dl-' + f.id + '">'
                            + '<div class="file-info">'
                            + '<a onclick="downloadFile(\'' + f.id + '\',\'' + f.name.replace(/'/g, "\\'") + '\',' + f.size + ')">' + f.name + '</a>'
                            + '<span class="file-size">(' + formatSize(f.size) + ')</span>'
                            + (badges ? '<div class="badges">' + badges + '</div>' : '')
                            + '<div class="progress-bar"><div class="progress-fill" style="width:0%"></div></div>'
                            + '<div class="progress-text"></div>'
                            + '</div>'
                            + '</li>';
                    }}).join('');
                }})
                .catch(function() {{}});
        }}

        initEnhanced().then(function() {{
            refreshFiles();
            setInterval(refreshFiles, 2000);
        }});
"#,
        labels.downloading,
        labels.download_complete,
        labels.download_failed,
        labels.downloading,
        labels.download_complete,
        labels.download_failed,
        labels.no_files,
        labels.encrypted_label,
        labels.compressed_label
    )
}

// ─── HTML templates ─────────────────────────────────────────────────────────

fn generate_share_ended_html(is_english: bool) -> String {
    let title = if is_english { "PureSend - Share Ended" } else { "PureSend - 分享已结束" };
    let heading = if is_english { "Share Ended" } else { "分享已结束" };
    let lang = if is_english { "en" } else { "zh-CN" };

    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }}
        h1 {{ color: #666; }}
        .icon {{ font-size: 48px; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="icon">📁</div>
    <h1>{heading}</h1>
</body>
</html>"#
    )
}

fn generate_access_denied_html(is_english: bool) -> String {
    let title = if is_english { "PureSend - Access Denied" } else { "PureSend - 访问被拒绝" };
    let heading = if is_english { "Access Denied" } else { "访问被拒绝" };
    let lang = if is_english { "en" } else { "zh-CN" };

    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }}
        h1 {{ color: #d32f2f; }}
        .icon {{ font-size: 48px; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="icon">🚫</div>
    <h1>{heading}</h1>
</body>
</html>"#
    )
}

fn generate_locked_html(remaining_secs: u64, is_english: bool) -> String {
    let minutes = remaining_secs / 60;
    let seconds = remaining_secs % 60;
    let time_str = if is_english {
        if minutes > 0 { format!("{} min {} sec", minutes, seconds) } else { format!("{} sec", seconds) }
    } else {
        if minutes > 0 { format!("{} 分 {} 秒", minutes, seconds) } else { format!("{} 秒", seconds) }
    };

    let title = if is_english { "PureSend - Locked" } else { "PureSend - 已锁定" };
    let heading = if is_english { "Access Locked" } else { "访问已锁定" };
    let message = if is_english { "Too many PIN attempts. Please try again later." } else { "PIN 码验证失败次数过多，请稍后再试" };
    let timer_label = if is_english { "Time remaining:" } else { "剩余时间：" };
    let lang = if is_english { "en" } else { "zh-CN" };
    let min_unit = if is_english { "min " } else { "分 " };
    let sec_unit = if is_english { "sec" } else { "秒" };

    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }}
        h1 {{ color: #d32f2f; }}
        .lock-icon {{ font-size: 48px; margin: 20px 0; }}
        .message {{ color: #666; margin-top: 20px; }}
        .timer {{ font-size: 24px; color: #1976d2; margin: 20px 0; font-weight: bold; }}
    </style>
</head>
<body>
    <h1>{heading}</h1>
    <div class="lock-icon">🔒</div>
    <div class="message">{message}</div>
    <div class="timer" id="timer">{timer_label} {0}</div>
    <script>
        let remaining = {1};
        function updateTimer() {{
            if (remaining <= 0) {{
                window.location.reload();
                return;
            }}
            remaining--;
            const min = Math.floor(remaining / 60);
            const sec = remaining % 60;
            const timeStr = min > 0 ? min + ' {2}' + sec + ' {3}' : sec + ' {3}';
            document.getElementById('timer').textContent = '{4}' + timeStr;
            setTimeout(updateTimer, 1000);
        }}
        updateTimer();
    </script>
</body>
</html>"#,
        time_str, remaining_secs, min_unit, sec_unit, timer_label
    )
}

fn generate_pin_input_html(is_english: bool) -> String {
    let title = if is_english { "PureSend - PIN Verification" } else { "PureSend - PIN 验证" };
    let heading = if is_english { "Enter PIN Code" } else { "请输入 PIN 码" };
    let placeholder = if is_english { "Enter PIN" } else { "输入 PIN 码" };
    let button_text = if is_english { "Verify" } else { "验证" };
    let lang = if is_english { "en" } else { "zh-CN" };
    let empty_pin_error = if is_english { "Please enter PIN" } else { "请输入 PIN 码" };
    let locked_error = if is_english { "Too many attempts. Locked for 5 minutes." } else { "尝试次数过多，已锁定 5 分钟" };
    let incorrect_pin_prefix = if is_english { "Incorrect PIN. Remaining attempts: " } else { "PIN 码错误，剩余尝试次数：" };
    let verify_failed_error = if is_english { "Verification failed. Please try again." } else { "验证失败，请重试" };

    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }}
        h1 {{ color: #333; margin-bottom: 20px; }}
        .input-container {{ width: 100%; max-width: 300px; margin: 0 auto 15px; }}
        input {{ width: 100%; padding: 12px; font-size: 18px; text-align: center; border: 1px solid #ccc; border-radius: 4px; box-sizing: border-box; }}
        button {{ width: 100%; max-width: 300px; padding: 12px; background: #1976d2; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; }}
        button:hover {{ background: #1565c0; }}
        .error {{ color: #d32f2f; margin-top: 10px; }}
    </style>
</head>
<body>
    <h1>{heading}</h1>
    <div class="input-container">
        <input type="text" id="pin" placeholder="{placeholder}">
    </div>
    <button onclick="verify()">{button_text}</button>
    <div id="error" class="error"></div>
    
    <script>
        async function verify() {{
            const pin = document.getElementById('pin').value;
            const errorDiv = document.getElementById('error');
            
            if (!pin) {{
                errorDiv.textContent = '{empty_pin_error}';
                return;
            }}
            
            try {{
                const response = await fetch('/verify-pin', {{
                    method: 'POST',
                    headers: {{ 'Content-Type': 'application/json' }},
                    body: JSON.stringify({{ pin }})
                }});
                
                const result = await response.json();
                
                if (result.success) {{
                    window.location.reload();
                }} else {{
                    if (result.locked) {{
                        errorDiv.textContent = '{locked_error}';
                    }} else {{
                        errorDiv.textContent = '{incorrect_pin_prefix}' + (result.remainingAttempts || 0);
                    }}
                }}

            }} catch (e) {{
                errorDiv.textContent = '{verify_failed_error}';
            }}
        }}
        
        document.getElementById('pin').addEventListener('keypress', function(e) {{
            if (e.key === 'Enter') {{
                verify();
            }}
        }});
    </script>
</body>
</html>"#
    )
}

fn generate_waiting_response_html(is_english: bool) -> String {
    let title = if is_english { "PureSend - Waiting" } else { "PureSend - 等待响应" };
    let heading = if is_english { "Waiting for Response" } else { "等待响应中" };
    let message = if is_english { "Waiting for the sharer to accept your access request..." } else { "等待分享方接受您的访问请求..." };
    let checking = if is_english { "Checking status..." } else { "正在检查状态..." };
    let waiting = if is_english { "Waiting for approval..." } else { "等待分享方接受..." };
    let accepted = if is_english { "✓ Accepted! Redirecting..." } else { "✓ 已接受！正在跳转..." };
    let rejected = if is_english { "✗ Access request denied" } else { "✗ 访问请求被拒绝" };
    let lang = if is_english { "en" } else { "zh-CN" };

    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>{title}</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }}
        h1 {{ color: #1976d2; }}
        .spinner {{ border: 4px solid #f3f3f3; border-top: 4px solid #1976d2; border-radius: 50%; width: 40px; height: 40px; animation: spin 1s linear infinite; margin: 20px auto; }}
        @keyframes spin {{ 0% {{ transform: rotate(0deg); }} 100% {{ transform: rotate(360deg); }} }}
        .message {{ color: #666; margin-top: 20px; }}
        .status {{ margin-top: 15px; font-weight: bold; color: #1976d2; }}
    </style>
</head>
<body>
    <h1>{heading}</h1>
    <div class="spinner"></div>
    <div class="message">{message}</div>
    <div class="status" id="status">{checking}</div>
    <script>
        async function checkStatus() {{
            try {{
                const response = await fetch('/request-status');
                const result = await response.json();
                
                const statusDiv = document.getElementById('status');
                
                if (result.status === 'accepted') {{
                    statusDiv.textContent = '{accepted}';
                    statusDiv.style.color = '#4caf50';
                    setTimeout(() => {{
                        window.location.reload();
                    }}, 500);
                }} else if (result.status === 'rejected') {{
                    statusDiv.textContent = '{rejected}';
                    statusDiv.style.color = '#f44336';
                }} else {{
                    statusDiv.textContent = '{waiting}';
                    setTimeout(checkStatus, 1000);
                }}
            }} catch (e) {{
                console.error('Failed to check status:', e);
                setTimeout(checkStatus, 2000);
            }}
        }}
        
        checkStatus();
    </script>
</body>
</html>"#
    )
}

/// Enhanced file list page with encryption, compression, and resume support
fn generate_file_list_html(is_english: bool) -> String {
    let title = if is_english { "PureSend - File Sharing" } else { "PureSend - 文件分享" };
    let heading = if is_english { "PureSend File Sharing" } else { "PureSend 文件分享" };
    let warning = if is_english {
        "⚠️ This link is for trusted networks only. Do not share on public platforms."
    } else {
        "⚠️ 此链接仅限可信网络内使用，请勿分享到公共平台"
    };
    let files_heading = if is_english { "Available Files" } else { "可用文件" };
    let loading = if is_english { "Loading..." } else { "加载中..." };
    let lang = if is_english { "en" } else { "zh-CN" };

    let labels = FileListPageLabels {
        downloading: if is_english { "Downloading...".to_string() } else { "下载中...".to_string() },
        download_complete: if is_english { "Download complete".to_string() } else { "下载完成".to_string() },
        download_failed: if is_english { "Download failed".to_string() } else { "下载失败".to_string() },
        encrypted_label: if is_english { "Encrypted".to_string() } else { "已加密".to_string() },
        compressed_label: if is_english { "Compressed".to_string() } else { "已压缩".to_string() },
        no_files: if is_english { "No files available".to_string() } else { "暂无可用文件".to_string() },
    };

    let css = file_list_page_css().to_string();
    let javascript = file_list_page_javascript(&labels);

    format!(
        r##"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>{title}</title>
    <style>
{css}
    </style>
</head>
<body>
    <h1>{heading}</h1>
    <div class="warning">{warning}</div>
    <h2>{files_heading}</h2>
    <ul id="file-list">
        <li class="empty">{loading}</li>
    </ul>
    <script>
{javascript}
    </script>
</body>
</html>"##
    )
}
