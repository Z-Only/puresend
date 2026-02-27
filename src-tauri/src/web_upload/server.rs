//! Web 上传 HTTP 服务器实现
//!
//! 提供文件上传的 HTTP 服务，支持分块上传、断点续传、传输加密和动态压缩

use axum::extract::DefaultBodyLimit;
use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, Multipart, Path, State as AxumState},
    http::{header, HeaderMap, HeaderName, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use super::models::{UploadRequest, UploadRequestStatus, WebUploadRecord, WebUploadState};
use crate::transfer::compression::Compressor;
use crate::transfer::crypto::is_encryption_enabled;
use crate::transfer::http_crypto::{
    HandshakeRequest, HandshakeResponse, HttpCryptoSessionManager,
};

static FAVICON_ICO: &[u8] = include_bytes!("../../icons/32x32.png");

const HTTP_CHUNK_SIZE: usize = 1024 * 1024; // 1MB
const UPLOAD_SESSION_EXPIRY_SECS: u64 = 24 * 3600; // 24h

/// Chunked upload session
#[derive(Debug)]
pub struct ChunkedUploadSession {
    id: String,
    file_name: String,
    file_size: u64,
    chunk_size: usize,
    chunk_count: usize,
    received_chunks: HashSet<usize>,
    temp_dir: PathBuf,
    client_ip: String,
    request_id: String,
    created_at: Instant,
}

impl ChunkedUploadSession {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() > UPLOAD_SESSION_EXPIRY_SECS
    }

    fn is_complete(&self) -> bool {
        self.received_chunks.len() == self.chunk_count
    }
}

#[derive(Debug)]
pub struct UploadServerState {
    pub upload_state: Arc<Mutex<WebUploadState>>,
    pub app_handle: AppHandle,
    pub crypto_sessions: Arc<Mutex<HttpCryptoSessionManager>>,
    pub upload_sessions: Arc<Mutex<HashMap<String, ChunkedUploadSession>>>,
}

pub struct WebUploadServer {
    pub addr: SocketAddr,
    pub state: Arc<UploadServerState>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl WebUploadServer {
    pub fn new(upload_state: Arc<Mutex<WebUploadState>>, app_handle: AppHandle) -> Self {
        let addr = SocketAddr::from(([0, 0, 0, 0], 0));

        Self {
            addr,
            state: Arc::new(UploadServerState {
                upload_state,
                app_handle,
                crypto_sessions: Arc::new(Mutex::new(HttpCryptoSessionManager::new())),
                upload_sessions: Arc::new(Mutex::new(HashMap::new())),
            }),
            shutdown_tx: None,
        }
    }

    pub async fn start(&mut self) -> Result<u16, String> {
        let app = Router::new()
            .route("/", get(index_handler))
            .route("/favicon.ico", get(favicon_handler))
            .route("/request-status", get(request_status_handler))
            .route("/capabilities", get(capabilities_handler))
            .route("/crypto/handshake", post(crypto_handshake_handler))
            .route("/upload/init", post(upload_init_handler))
            .route(
                "/upload/chunk",
                post(upload_chunk_handler).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
            )
            .route("/upload/status/{upload_id}", get(upload_session_status_handler))
            .route(
                "/upload",
                post(upload_handler).layer(DefaultBodyLimit::max(10 * 1024 * 1024 * 1024)),
            )
            .fallback(fallback_handler)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
                    .allow_headers([
                        header::CONTENT_TYPE,
                        header::ACCEPT,
                        HeaderName::from_static("x-upload-id"),
                        HeaderName::from_static("x-chunk-index"),
                        HeaderName::from_static("x-encryption-session"),
                        HeaderName::from_static("x-compression"),
                    ])
                    .expose_headers([
                        HeaderName::from_static("x-file-hash"),
                    ]),
            )
            .with_state(self.state.clone());

        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| format!("绑定端口失败: {}", e))?;

        let actual_port = listener
            .local_addr()
            .map_err(|e| format!("获取端口失败: {}", e))?
            .port();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        // Periodic cleanup of expired sessions
        let crypto_sessions = self.state.crypto_sessions.clone();
        let upload_sessions = self.state.upload_sessions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                crypto_sessions.lock().await.cleanup_expired();
                upload_sessions.lock().await.retain(|_, s| !s.is_expired());
            }
        });

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

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn favicon_handler() -> impl IntoResponse {
    let mut response = Response::new(Body::from(FAVICON_ICO));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("image/png"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("max-age=86400"),
    );
    response
}

async fn capabilities_handler() -> Json<ServerCapabilities> {
    let encryption = is_encryption_enabled();
    let compression_config = crate::transfer::compression::get_compression_config();
    Json(ServerCapabilities {
        encryption,
        compression: compression_config.enabled,
        chunk_size: HTTP_CHUNK_SIZE,
    })
}

async fn crypto_handshake_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<UploadServerState>>,
    Json(payload): Json<HandshakeRequest>,
) -> Json<HandshakeResponse> {
    if !is_encryption_enabled() {
        return Json(HandshakeResponse {
            encryption: false,
            server_public_key: None,
            session_id: None,
        });
    }

    let client_ip = client_addr.ip().to_string();
    let mut crypto_sessions = state.crypto_sessions.lock().await;

    match crypto_sessions.handshake(&payload.client_public_key, client_ip) {
        Ok((session_id, server_pub_key)) => Json(HandshakeResponse {
            encryption: true,
            server_public_key: Some(server_pub_key),
            session_id: Some(session_id),
        }),
        Err(e) => {
            eprintln!("加密握手失败: {}", e);
            Json(HandshakeResponse {
                encryption: false,
                server_public_key: None,
                session_id: None,
            })
        }
    }
}

/// Initialize chunked upload session
async fn upload_init_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<UploadServerState>>,
    Json(payload): Json<UploadInitRequest>,
) -> Json<UploadInitResponse> {
    let client_ip = client_addr.ip().to_string();

    let (is_allowed, receive_directory, request_id) = {
        let upload_state = state.upload_state.lock().await;
        let allowed = upload_state.is_ip_allowed(&client_ip);
        let req_id = upload_state
            .requests
            .values()
            .find(|r| r.client_ip == client_ip)
            .map(|r| r.id.clone())
            .unwrap_or_default();
        (allowed, upload_state.receive_directory.clone(), req_id)
    };

    if !is_allowed || request_id.is_empty() {
        return Json(UploadInitResponse {
            success: false,
            upload_id: String::new(),
            chunk_size: 0,
            chunk_count: 0,
            message: Some("未授权上传".to_string()),
        });
    }

    let chunk_size = if payload.chunk_size > 0 {
        payload.chunk_size
    } else {
        HTTP_CHUNK_SIZE
    };
    let chunk_count = ((payload.file_size as f64) / (chunk_size as f64)).ceil() as usize;
    let upload_id = uuid::Uuid::new_v4().to_string();

    // Create temp directory for chunks
    let temp_dir = PathBuf::from(&receive_directory)
        .join(".puresend_chunks")
        .join(&upload_id);
    if let Err(e) = tokio::fs::create_dir_all(&temp_dir).await {
        return Json(UploadInitResponse {
            success: false,
            upload_id: String::new(),
            chunk_size: 0,
            chunk_count: 0,
            message: Some(format!("创建临时目录失败: {}", e)),
        });
    }

    let session = ChunkedUploadSession {
        id: upload_id.clone(),
        file_name: payload.file_name.clone(),
        file_size: payload.file_size,
        chunk_size,
        chunk_count,
        received_chunks: HashSet::new(),
        temp_dir,
        client_ip,
        request_id,
        created_at: Instant::now(),
    };

    state
        .upload_sessions
        .lock()
        .await
        .insert(upload_id.clone(), session);

    Json(UploadInitResponse {
        success: true,
        upload_id,
        chunk_size,
        chunk_count,
        message: None,
    })
}

/// Upload a single chunk
async fn upload_chunk_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<UploadServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Json<UploadChunkResponse> {
    let client_ip = client_addr.ip().to_string();

    let upload_id = headers
        .get("x-upload-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let chunk_index: usize = headers
        .get("x-chunk-index")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if upload_id.is_empty() {
        return Json(UploadChunkResponse {
            success: false,
            message: "缺少 X-Upload-Id".to_string(),
            complete: false,
            file_hash: None,
        });
    }

    let mut data = body.to_vec();

    // Decrypt if needed
    let encryption_session_id = headers
        .get("x-encryption-session")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !encryption_session_id.is_empty() {
        let crypto_sessions = state.crypto_sessions.lock().await;
        if let Some(session) = crypto_sessions.get_session(&encryption_session_id) {
            match session.decrypt(&data) {
                Ok(decrypted) => data = decrypted,
                Err(e) => {
                    return Json(UploadChunkResponse {
                        success: false,
                        message: format!("解密失败: {}", e),
                        complete: false,
                        file_hash: None,
                    });
                }
            }
        }
    }

    // Decompress if needed
    let compression = headers
        .get("x-compression")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if compression == "zstd" {
        match Compressor::decompress(&data) {
            Ok(decompressed) => data = decompressed,
            Err(e) => {
                return Json(UploadChunkResponse {
                    success: false,
                    message: format!("解压失败: {}", e),
                    complete: false,
                    file_hash: None,
                });
            }
        }
    }

    // Save chunk to temp file and check completion
    let mut upload_sessions = state.upload_sessions.lock().await;
    let session = match upload_sessions.get_mut(&upload_id) {
        Some(s) if s.client_ip == client_ip => s,
        _ => {
            return Json(UploadChunkResponse {
                success: false,
                message: "上传会话不存在".to_string(),
                complete: false,
                file_hash: None,
            });
        }
    };

    let chunk_path = session.temp_dir.join(format!("chunk_{}", chunk_index));
    if let Err(e) = tokio::fs::write(&chunk_path, &data).await {
        return Json(UploadChunkResponse {
            success: false,
            message: format!("写入分块失败: {}", e),
            complete: false,
            file_hash: None,
        });
    }

    session.received_chunks.insert(chunk_index);

    // Emit progress event
    let progress = (session.received_chunks.len() as f64 / session.chunk_count as f64) * 100.0;
    let _ = state.app_handle.emit(
        "web-upload-file-progress",
        FileProgressEvent {
            request_id: session.request_id.clone(),
            record_id: session.id.clone(),
            file_name: session.file_name.clone(),
            uploaded_bytes: session.received_chunks.len() as u64 * session.chunk_size as u64,
            total_bytes: session.file_size,
            progress,
            speed: 0,
        },
    );

    if session.is_complete() {
        // Merge chunks into final file
        let file_name = session.file_name.clone();
        let file_size = session.file_size;
        let chunk_count = session.chunk_count;
        let temp_dir = session.temp_dir.clone();
        let request_id = session.request_id.clone();
        let record_id = session.id.clone();

        let (receive_directory, file_overwrite) = {
            let upload_state = state.upload_state.lock().await;
            (
                upload_state.receive_directory.clone(),
                upload_state.file_overwrite,
            )
        };

        let receive_dir = PathBuf::from(&receive_directory);
        let mut final_path = receive_dir.join(&file_name);
        if !file_overwrite && final_path.exists() {
            final_path = get_unique_path(&final_path);
        }

        // Merge all chunks
        let mut hasher = Sha256::new();
        match tokio::fs::File::create(&final_path).await {
            Ok(mut output) => {
                for i in 0..chunk_count {
                    let chunk_path = temp_dir.join(format!("chunk_{}", i));
                    match tokio::fs::read(&chunk_path).await {
                        Ok(chunk_data) => {
                            hasher.update(&chunk_data);
                            if let Err(e) = output.write_all(&chunk_data).await {
                                return Json(UploadChunkResponse {
                                    success: false,
                                    message: format!("合并分块失败: {}", e),
                                    complete: false,
                                    file_hash: None,
                                });
                            }
                        }
                        Err(e) => {
                            return Json(UploadChunkResponse {
                                success: false,
                                message: format!("读取分块失败: {}", e),
                                complete: false,
                                file_hash: None,
                            });
                        }
                    }
                }
            }
            Err(e) => {
                return Json(UploadChunkResponse {
                    success: false,
                    message: format!("创建目标文件失败: {}", e),
                    complete: false,
                    file_hash: None,
                });
            }
        }

        let file_hash = hex::encode(hasher.finalize());

        // Cleanup temp directory
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;

        // Update upload record
        {
            let mut upload_state = state.upload_state.lock().await;
            if let Some(req) = upload_state
                .requests
                .values_mut()
                .find(|r| r.id == request_id)
            {
                let record = WebUploadRecord {
                    id: record_id.clone(),
                    file_name: file_name.clone(),
                    uploaded_bytes: file_size,
                    total_bytes: file_size,
                    progress: 100.0,
                    speed: 0,
                    status: "completed".to_string(),
                    started_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    completed_at: Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                    ),
                };
                req.upload_records.push(record);
            }
        }

        let _ = state.app_handle.emit(
            "web-upload-file-complete",
            FileCompleteEvent {
                request_id,
                record_id,
                file_name,
                total_bytes: file_size,
                status: "completed".to_string(),
            },
        );

        // Remove the session
        upload_sessions.remove(&upload_id);

        return Json(UploadChunkResponse {
            success: true,
            message: "上传完成".to_string(),
            complete: true,
            file_hash: Some(file_hash),
        });
    }

    Json(UploadChunkResponse {
        success: true,
        message: format!("分块 {} 已接收", chunk_index),
        complete: false,
        file_hash: None,
    })
}

/// Query upload session status (for resume)
async fn upload_session_status_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<UploadServerState>>,
    Path(upload_id): Path<String>,
) -> Json<UploadSessionStatusResponse> {
    let client_ip = client_addr.ip().to_string();
    let upload_sessions = state.upload_sessions.lock().await;

    match upload_sessions.get(&upload_id) {
        Some(session) if session.client_ip == client_ip && !session.is_expired() => {
            let mut received: Vec<usize> = session.received_chunks.iter().copied().collect();
            received.sort();
            Json(UploadSessionStatusResponse {
                found: true,
                upload_id: session.id.clone(),
                file_name: Some(session.file_name.clone()),
                received_chunks: received,
                total_chunks: session.chunk_count,
                complete: session.is_complete(),
            })
        }
        _ => Json(UploadSessionStatusResponse {
            found: false,
            upload_id,
            file_name: None,
            received_chunks: vec![],
            total_chunks: 0,
            complete: false,
        }),
    }
}

/// Index handler
async fn index_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumState(state): AxumState<Arc<UploadServerState>>,
) -> Response {
    let client_ip = client_addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let accept_language = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("zh-CN");
    let is_english = accept_language.starts_with("en");

    let mut upload_state = state.upload_state.lock().await;

    if upload_state.is_ip_rejected(&client_ip) {
        return Html(generate_rejected_page(is_english)).into_response();
    }

    let has_request = upload_state
        .requests
        .values()
        .any(|r| r.client_ip == client_ip);

    if !has_request {
        if upload_state.auto_receive {
            let mut request = UploadRequest::new(client_ip.clone());
            request.status = UploadRequestStatus::Accepted;
            request.user_agent = user_agent;
            upload_state
                .requests
                .insert(request.id.clone(), request.clone());
            if !upload_state.allowed_ips.contains(&client_ip) {
                upload_state.allowed_ips.push(client_ip.clone());
            }
            let _ = state.app_handle.emit("web-upload-task", &request);
        } else {
            let mut request = UploadRequest::new(client_ip.clone());
            request.user_agent = user_agent;
            upload_state
                .requests
                .insert(request.id.clone(), request.clone());
            let _ = state.app_handle.emit("web-upload-task", &request);
        }
    }

    let is_allowed = upload_state.is_ip_allowed(&client_ip);

    if is_allowed {
        Html(generate_upload_page(is_english)).into_response()
    } else {
        Html(generate_waiting_page(is_english)).into_response()
    }
}

/// Request status handler
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestStatusResponse {
    has_request: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

async fn request_status_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<UploadServerState>>,
) -> Json<RequestStatusResponse> {
    let client_ip = client_addr.ip().to_string();
    let upload_state = state.upload_state.lock().await;

    let request = upload_state
        .requests
        .values()
        .find(|r| r.client_ip == client_ip);

    match request {
        Some(req) => {
            let status_str = match req.status {
                UploadRequestStatus::Pending => "pending",
                UploadRequestStatus::Accepted => "accepted",
                UploadRequestStatus::Rejected => "rejected",
                UploadRequestStatus::Expired => "expired",
            };
            Json(RequestStatusResponse {
                has_request: true,
                status: Some(status_str.to_string()),
            })
        }
        None => Json(RequestStatusResponse {
            has_request: false,
            status: None,
        }),
    }
}

/// Legacy multipart upload handler (backward compatible)
async fn upload_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<UploadServerState>>,
    mut multipart: Multipart,
) -> Json<UploadResponse> {
    let client_ip = client_addr.ip().to_string();

    let (is_allowed, file_overwrite, receive_directory, request_id) = {
        let upload_state = state.upload_state.lock().await;
        let allowed = upload_state.is_ip_allowed(&client_ip);
        let req_id = upload_state
            .requests
            .values()
            .find(|r| r.client_ip == client_ip)
            .map(|r| r.id.clone())
            .unwrap_or_default();
        (
            allowed,
            upload_state.file_overwrite,
            upload_state.receive_directory.clone(),
            req_id,
        )
    };

    if !is_allowed {
        return Json(UploadResponse {
            success: false,
            message: "未授权上传".to_string(),
        });
    }

    if request_id.is_empty() {
        return Json(UploadResponse {
            success: false,
            message: "未找到对应的上传请求".to_string(),
        });
    }

    let receive_dir = PathBuf::from(&receive_directory);
    if !receive_dir.exists() {
        if let Err(err) = tokio::fs::create_dir_all(&receive_dir).await {
            return Json(UploadResponse {
                success: false,
                message: format!("创建接收目录失败: {}", err),
            });
        }
    }

    let mut uploaded_count: u32 = 0;

    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = field.file_name().unwrap_or("unknown").to_string();
        let content_length = field
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let record_id = uuid::Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let record = WebUploadRecord {
            id: record_id.clone(),
            file_name: file_name.clone(),
            uploaded_bytes: 0,
            total_bytes: content_length,
            progress: 0.0,
            speed: 0,
            status: "transferring".to_string(),
            started_at: now,
            completed_at: None,
        };

        {
            let mut upload_state = state.upload_state.lock().await;
            if let Some(req) = upload_state
                .requests
                .values_mut()
                .find(|r| r.client_ip == client_ip)
            {
                req.upload_records.push(record);
            }
        }

        let _ = state.app_handle.emit(
            "web-upload-file-start",
            FileStartEvent {
                request_id: request_id.clone(),
                record_id: record_id.clone(),
                file_name: file_name.clone(),
                total_bytes: content_length,
                client_ip: client_ip.clone(),
            },
        );

        let mut file_path = receive_dir.join(&file_name);
        if !file_overwrite && file_path.exists() {
            file_path = get_unique_path(&file_path);
        }

        let start_time = std::time::Instant::now();
        let total_written: u64;

        match tokio::fs::File::create(&file_path).await {
            Ok(mut output_file) => {
                match field.bytes().await {
                    Ok(data) => {
                        let data_len = data.len() as u64;
                        if let Err(err) = output_file.write_all(&data).await {
                            let _ = state.app_handle.emit(
                                "web-upload-file-complete",
                                FileCompleteEvent {
                                    request_id: request_id.clone(),
                                    record_id: record_id.clone(),
                                    file_name: file_name.clone(),
                                    total_bytes: data_len,
                                    status: "failed".to_string(),
                                },
                            );

                            let mut upload_state = state.upload_state.lock().await;
                            if let Some(req) = upload_state
                                .requests
                                .values_mut()
                                .find(|r| r.client_ip == client_ip)
                            {
                                if let Some(rec) =
                                    req.upload_records.iter_mut().find(|r| r.id == record_id)
                                {
                                    rec.status = "failed".to_string();
                                    rec.completed_at = Some(
                                        std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs(),
                                    );
                                }
                            }

                            return Json(UploadResponse {
                                success: false,
                                message: format!("写入文件失败: {}", err),
                            });
                        }

                        total_written = data_len;

                        let elapsed = start_time.elapsed().as_secs_f64();
                        let speed = if elapsed > 0.0 {
                            (total_written as f64 / elapsed) as u64
                        } else {
                            0
                        };
                        let actual_total = if content_length > 0 {
                            content_length
                        } else {
                            total_written
                        };
                        let progress = if actual_total > 0 {
                            (total_written as f64 / actual_total as f64) * 100.0
                        } else {
                            100.0
                        };

                        let _ = state.app_handle.emit(
                            "web-upload-file-progress",
                            FileProgressEvent {
                                request_id: request_id.clone(),
                                record_id: record_id.clone(),
                                file_name: file_name.clone(),
                                uploaded_bytes: total_written,
                                total_bytes: actual_total,
                                progress,
                                speed,
                            },
                        );
                    }
                    Err(err) => {
                        let mut upload_state = state.upload_state.lock().await;
                        if let Some(req) = upload_state
                            .requests
                            .values_mut()
                            .find(|r| r.client_ip == client_ip)
                        {
                            if let Some(rec) =
                                req.upload_records.iter_mut().find(|r| r.id == record_id)
                            {
                                rec.status = "failed".to_string();
                                rec.completed_at = Some(
                                    std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                );
                            }
                        }

                        let _ = state.app_handle.emit(
                            "web-upload-file-complete",
                            FileCompleteEvent {
                                request_id: request_id.clone(),
                                record_id: record_id.clone(),
                                file_name: file_name.clone(),
                                total_bytes: 0,
                                status: "failed".to_string(),
                            },
                        );

                        return Json(UploadResponse {
                            success: false,
                            message: format!("读取文件数据失败: {}", err),
                        });
                    }
                }
            }
            Err(err) => {
                let mut upload_state = state.upload_state.lock().await;
                if let Some(req) = upload_state
                    .requests
                    .values_mut()
                    .find(|r| r.client_ip == client_ip)
                {
                    if let Some(rec) = req.upload_records.iter_mut().find(|r| r.id == record_id) {
                        rec.status = "failed".to_string();
                        rec.completed_at = Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                        );
                    }
                }

                return Json(UploadResponse {
                    success: false,
                    message: format!("创建文件失败: {}", err),
                });
            }
        }

        let completed_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = start_time.elapsed().as_secs_f64();
        let final_speed = if elapsed > 0.0 {
            (total_written as f64 / elapsed) as u64
        } else {
            0
        };

        {
            let mut upload_state = state.upload_state.lock().await;
            if let Some(req) = upload_state
                .requests
                .values_mut()
                .find(|r| r.client_ip == client_ip)
            {
                if let Some(rec) = req.upload_records.iter_mut().find(|r| r.id == record_id) {
                    rec.uploaded_bytes = total_written;
                    rec.total_bytes = total_written;
                    rec.progress = 100.0;
                    rec.speed = final_speed;
                    rec.status = "completed".to_string();
                    rec.completed_at = Some(completed_at);
                }
            }
        }

        let _ = state.app_handle.emit(
            "web-upload-file-complete",
            FileCompleteEvent {
                request_id: request_id.clone(),
                record_id: record_id.clone(),
                file_name: file_name.clone(),
                total_bytes: total_written,
                status: "completed".to_string(),
            },
        );

        uploaded_count += 1;
    }

    if uploaded_count == 0 {
        return Json(UploadResponse {
            success: false,
            message: "未接收到任何文件数据".to_string(),
        });
    }

    Json(UploadResponse {
        success: true,
        message: format!("成功上传 {} 个文件", uploaded_count),
    })
}

fn get_unique_path(path: &PathBuf) -> PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let parent = path.parent().unwrap_or(path);

    let mut counter = 1;
    loop {
        let new_name = if extension.is_empty() {
            format!("{}_{}", stem, counter)
        } else {
            format!("{}_{}.{}", stem, counter, extension)
        };
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

async fn fallback_handler() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

// ─── Data types ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ServerCapabilities {
    encryption: bool,
    compression: bool,
    chunk_size: usize,
}

#[derive(Debug, Deserialize)]
struct UploadInitRequest {
    file_name: String,
    file_size: u64,
    #[serde(default)]
    chunk_size: usize,
}

#[derive(Debug, Serialize)]
struct UploadInitResponse {
    success: bool,
    upload_id: String,
    chunk_size: usize,
    chunk_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct UploadChunkResponse {
    success: bool,
    message: String,
    complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_hash: Option<String>,
}

#[derive(Debug, Serialize)]
struct UploadSessionStatusResponse {
    found: bool,
    upload_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_name: Option<String>,
    received_chunks: Vec<usize>,
    total_chunks: usize,
    complete: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    success: bool,
    message: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FileStartEvent {
    request_id: String,
    record_id: String,
    file_name: String,
    total_bytes: u64,
    client_ip: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FileProgressEvent {
    request_id: String,
    record_id: String,
    file_name: String,
    uploaded_bytes: u64,
    total_bytes: u64,
    progress: f64,
    speed: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FileCompleteEvent {
    request_id: String,
    record_id: String,
    file_name: String,
    total_bytes: u64,
    status: String,
}

// ─── HTML Templates ─────────────────────────────────────────────────────────

/// Enhanced upload page with chunked upload, encryption, compression, and resume
fn generate_upload_page(is_english: bool) -> String {
    let title = if is_english { "PureSend - Upload Files" } else { "PureSend - 文件上传" };
    let select_files = if is_english { "Select Files" } else { "选择文件" };
    let drag_hint = if is_english { "or drag and drop files here" } else { "或将文件拖拽到此处" };
    let upload_btn = if is_english { "Upload" } else { "上传" };
    let transferring = if is_english { "Uploading files..." } else { "正在上传文件..." };
    let success_msg = if is_english { "Files uploaded successfully!" } else { "文件上传成功！" };
    let failed_msg = if is_english { "Upload failed" } else { "上传失败" };
    let file_label = if is_english { "file(s)" } else { "个文件" };
    let total_size_label = if is_english { "Total size" } else { "总大小" };
    let remove_label = if is_english { "Remove" } else { "移除" };
    let encrypted_label = if is_english { "Encrypted" } else { "已加密" };
    let lang = if is_english { "en" } else { "zh-CN" };

    format!(
        r##"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="icon" type="image/png" href="/favicon.ico">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #f5f5f5; color: #333; min-height: 100vh; display: flex; align-items: center; justify-content: center; }}
        .container {{ max-width: 520px; width: 100%; padding: 20px; }}
        .card {{ background: #fff; border-radius: 16px; padding: 32px; box-shadow: 0 2px 12px rgba(0,0,0,0.08); }}
        h1 {{ font-size: 24px; font-weight: 600; margin-bottom: 8px; text-align: center; }}
        .subtitle {{ color: #666; text-align: center; margin-bottom: 24px; font-size: 14px; }}
        .badges {{ display: flex; gap: 6px; justify-content: center; margin-bottom: 16px; }}
        .badge {{ font-size: 11px; padding: 2px 8px; border-radius: 4px; color: #fff; background: #2e7d32; }}
        .drop-zone {{ border: 2px dashed #ddd; border-radius: 12px; padding: 40px 20px; text-align: center; cursor: pointer; transition: all 0.2s; }}
        .drop-zone:hover, .drop-zone.dragover {{ border-color: #1976d2; background: #e3f2fd; }}
        .drop-zone-icon {{ font-size: 48px; margin-bottom: 12px; }}
        .drop-zone-text {{ color: #666; font-size: 14px; }}
        .drop-zone-btn {{ display: inline-block; margin-top: 12px; padding: 8px 24px; background: #1976d2; color: #fff; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; }}
        .drop-zone-btn:hover {{ background: #1565c0; }}
        .file-list {{ margin-top: 16px; max-height: 200px; overflow-y: auto; }}
        .file-item {{ display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: #f9f9f9; border-radius: 8px; margin-bottom: 8px; font-size: 13px; }}
        .file-item .name {{ flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
        .file-item .size {{ color: #999; margin: 0 12px; white-space: nowrap; }}
        .file-item .remove {{ color: #f44336; cursor: pointer; border: none; background: none; font-size: 12px; }}
        .stats {{ margin-top: 8px; font-size: 13px; color: #666; }}
        .upload-btn {{ display: block; width: 100%; margin-top: 20px; padding: 14px; background: #4caf50; color: #fff; border: none; border-radius: 10px; font-size: 16px; font-weight: 500; cursor: pointer; transition: background 0.2s; }}
        .upload-btn:hover {{ background: #43a047; }}
        .upload-btn:disabled {{ background: #ccc; cursor: not-allowed; }}
        .status {{ margin-top: 20px; padding: 16px; border-radius: 10px; text-align: center; font-size: 14px; display: none; }}
        .status.uploading {{ display: block; background: #e3f2fd; color: #1565c0; }}
        .status.success {{ display: block; background: #e8f5e9; color: #2e7d32; }}
        .status.error {{ display: block; background: #ffebee; color: #c62828; }}
        .hidden {{ display: none !important; }}
        .progress-bar {{ width: 100%; height: 6px; background: #e0e0e0; border-radius: 3px; margin-top: 8px; overflow: hidden; }}
        .progress-fill {{ height: 100%; background: #1976d2; transition: width 0.3s; width: 0%; }}
        .progress-text {{ font-size: 12px; color: #666; margin-top: 4px; text-align: center; }}
        .resume-prompt {{ margin-top: 16px; padding: 12px; background: #fff3e0; border-radius: 8px; text-align: center; font-size: 13px; }}
        .resume-prompt button {{ margin: 8px 4px 0; padding: 6px 16px; border: none; border-radius: 6px; cursor: pointer; font-size: 13px; }}
        .resume-btn {{ background: #1976d2; color: #fff; }}
        .restart-btn {{ background: #e0e0e0; color: #333; }}
        @media (prefers-color-scheme: dark) {{
            body {{ background: #121212; color: #e0e0e0; }}
            .card {{ background: #1e1e1e; box-shadow: 0 2px 12px rgba(0,0,0,0.3); }}
            .drop-zone {{ border-color: #444; }}
            .drop-zone:hover, .drop-zone.dragover {{ border-color: #42a5f5; background: #1a237e33; }}
            .drop-zone-text {{ color: #aaa; }}
            .file-item {{ background: #2a2a2a; }}
            .file-item .size {{ color: #888; }}
            .stats {{ color: #aaa; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <h1>📤 {title}</h1>
            <p class="subtitle">PureSend</p>
            <div class="badges" id="capBadges"></div>

            <div class="drop-zone" id="dropZone">
                <div class="drop-zone-icon">📁</div>
                <div class="drop-zone-text">{drag_hint}</div>
                <button class="drop-zone-btn" onclick="document.getElementById('fileInput').click()">{select_files}</button>
                <input type="file" id="fileInput" multiple style="display:none" />
            </div>

            <div class="file-list hidden" id="fileList"></div>
            <div class="stats hidden" id="stats"></div>
            <div id="resumePrompt" class="resume-prompt hidden"></div>

            <button class="upload-btn" id="uploadBtn" disabled>{upload_btn}</button>

            <div class="progress-bar hidden" id="progressBar"><div class="progress-fill" id="progressFill"></div></div>
            <div class="progress-text hidden" id="progressText"></div>
            <div class="status" id="status"></div>
        </div>
    </div>

    <script>
        const dropZone = document.getElementById("dropZone");
        const fileInput = document.getElementById("fileInput");
        const fileListEl = document.getElementById("fileList");
        const statsEl = document.getElementById("stats");
        const uploadBtn = document.getElementById("uploadBtn");
        const statusEl = document.getElementById("status");
        const progressBar = document.getElementById("progressBar");
        const progressFill = document.getElementById("progressFill");
        const progressText = document.getElementById("progressText");
        let selectedFiles = [];
        let caps = null;
        let cryptoKey = null;
        let sessionId = null;
        let nonceCounter = 0;

        function formatSize(bytes) {{
            if (bytes === 0) return "0 B";
            const k = 1024, sizes = ["B", "KB", "MB", "GB"];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
        }}

        async function initEnhanced() {{
            try {{
                const resp = await fetch("/capabilities");
                caps = await resp.json();
                const badgesEl = document.getElementById("capBadges");
                if (caps.encryption) {{
                    badgesEl.innerHTML += '<span class="badge">{encrypted_label}</span>';
                    await performHandshake();
                }}
            }} catch(e) {{
                caps = {{ encryption: false, compression: false, chunk_size: 1048576 }};
            }}
        }}

        async function performHandshake() {{
            try {{
                const keyPair = await crypto.subtle.generateKey(
                    {{ name: "ECDH", namedCurve: "P-256" }}, true, ["deriveBits"]
                );
                const pubRaw = await crypto.subtle.exportKey("raw", keyPair.publicKey);
                const pubB64 = btoa(String.fromCharCode.apply(null, new Uint8Array(pubRaw)));

                const resp = await fetch("/crypto/handshake", {{
                    method: "POST",
                    headers: {{ "Content-Type": "application/json" }},
                    body: JSON.stringify({{ client_public_key: pubB64 }})
                }});
                const result = await resp.json();
                if (!result.encryption) return;

                sessionId = result.session_id;
                const serverPubBytes = Uint8Array.from(atob(result.server_public_key), c => c.charCodeAt(0));
                const serverPubKey = await crypto.subtle.importKey(
                    "raw", serverPubBytes, {{ name: "ECDH", namedCurve: "P-256" }}, false, []
                );
                const sharedBits = await crypto.subtle.deriveBits(
                    {{ name: "ECDH", public: serverPubKey }}, keyPair.privateKey, 256
                );
                const hkdfKey = await crypto.subtle.importKey("raw", sharedBits, "HKDF", false, ["deriveKey"]);
                cryptoKey = await crypto.subtle.deriveKey(
                    {{ name: "HKDF", hash: "SHA-256", salt: new Uint8Array(0), info: new TextEncoder().encode("puresend-http-encryption") }},
                    hkdfKey,
                    {{ name: "AES-GCM", length: 256 }},
                    false, ["encrypt"]
                );
            }} catch(e) {{
                console.warn("Handshake failed:", e);
                if (caps) caps.encryption = false;
            }}
        }}

        function generateNonce() {{
            const nonce = new Uint8Array(12);
            const view = new DataView(nonce.buffer);
            view.setUint32(0, nonceCounter & 0xFFFFFFFF, true);
            view.setUint32(4, (nonceCounter / 0x100000000) & 0xFFFFFFFF, true);
            const rand = crypto.getRandomValues(new Uint8Array(4));
            nonce.set(rand, 8);
            nonceCounter++;
            return nonce;
        }}

        async function encryptChunk(data) {{
            const nonce = generateNonce();
            const encrypted = await crypto.subtle.encrypt(
                {{ name: "AES-GCM", iv: nonce }}, cryptoKey, data
            );
            const output = new Uint8Array(12 + encrypted.byteLength);
            output.set(nonce, 0);
            output.set(new Uint8Array(encrypted), 12);
            return output;
        }}

        function updateUI() {{
            fileListEl.innerHTML = "";
            if (selectedFiles.length === 0) {{
                fileListEl.classList.add("hidden");
                statsEl.classList.add("hidden");
                uploadBtn.disabled = true;
                return;
            }}
            fileListEl.classList.remove("hidden");
            statsEl.classList.remove("hidden");
            uploadBtn.disabled = false;
            let totalSize = 0;
            selectedFiles.forEach((file, index) => {{
                totalSize += file.size;
                const item = document.createElement("div");
                item.className = "file-item";
                item.innerHTML = `<span class="name">${{file.name}}</span><span class="size">${{formatSize(file.size)}}</span><button class="remove" onclick="removeFile(${{index}})">{remove_label}</button>`;
                fileListEl.appendChild(item);
            }});
            statsEl.textContent = `${{selectedFiles.length}} {file_label}，{total_size_label}: ${{formatSize(totalSize)}}`;
        }}

        function removeFile(index) {{ selectedFiles.splice(index, 1); updateUI(); }}

        function addFiles(files) {{
            for (const file of files) {{
                if (!selectedFiles.some(f => f.name === file.name && f.size === file.size)) {{
                    selectedFiles.push(file);
                }}
            }}
            statusEl.className = "status"; statusEl.textContent = "";
            updateUI();
        }}

        dropZone.addEventListener("dragover", e => {{ e.preventDefault(); dropZone.classList.add("dragover"); }});
        dropZone.addEventListener("dragleave", () => dropZone.classList.remove("dragover"));
        dropZone.addEventListener("drop", e => {{ e.preventDefault(); dropZone.classList.remove("dragover"); addFiles(e.dataTransfer.files); }});
        fileInput.addEventListener("change", () => {{ addFiles(fileInput.files); fileInput.value = ""; }});

        async function uploadChunked(file) {{
            const chunkSize = (caps && caps.chunk_size) || 1048576;
            const initResp = await fetch("/upload/init", {{
                method: "POST",
                headers: {{ "Content-Type": "application/json" }},
                body: JSON.stringify({{ file_name: file.name, file_size: file.size, chunk_size: chunkSize }})
            }});
            const initResult = await initResp.json();
            if (!initResult.success) throw new Error(initResult.message);

            const uploadId = initResult.upload_id;
            sessionStorage.setItem("puresend_upload_id_" + file.name, uploadId);

            let startChunk = 0;
            try {{
                const statusResp = await fetch("/upload/status/" + uploadId);
                const statusResult = await statusResp.json();
                if (statusResult.found && statusResult.received_chunks.length > 0) {{
                    startChunk = statusResult.received_chunks.length;
                }}
            }} catch(e) {{}}

            const totalChunks = initResult.chunk_count;
            for (let i = startChunk; i < totalChunks; i++) {{
                const start = i * chunkSize;
                const end = Math.min(start + chunkSize, file.size);
                let chunk = new Uint8Array(await file.slice(start, end).arrayBuffer());

                const hdrs = {{ "X-Upload-Id": uploadId, "X-Chunk-Index": String(i) }};
                if (cryptoKey && sessionId) {{
                    chunk = await encryptChunk(chunk);
                    hdrs["X-Encryption-Session"] = sessionId;
                }}

                const resp = await fetch("/upload/chunk", {{ method: "POST", headers: hdrs, body: chunk }});
                const result = await resp.json();
                if (!result.success) throw new Error(result.message);

                const pct = Math.round((i + 1) / totalChunks * 100);
                progressFill.style.width = pct + "%";
                progressText.textContent = pct + "% (" + formatSize(end) + " / " + formatSize(file.size) + ")";

                if (result.complete) {{
                    sessionStorage.removeItem("puresend_upload_id_" + file.name);
                    return result.file_hash;
                }}
            }}
            return null;
        }}

        async function uploadLegacy() {{
            const formData = new FormData();
            selectedFiles.forEach(file => formData.append("files", file));
            const response = await fetch("/upload", {{ method: "POST", body: formData }});
            return await response.json();
        }}

        uploadBtn.addEventListener("click", async () => {{
            if (selectedFiles.length === 0) return;
            uploadBtn.disabled = true;
            statusEl.className = "status uploading";
            statusEl.textContent = "{transferring}";
            statusEl.style.display = "block";
            progressBar.classList.remove("hidden");
            progressText.classList.remove("hidden");
            progressFill.style.width = "0%";

            try {{
                if (caps && (caps.encryption || caps.compression)) {{
                    for (const file of selectedFiles) {{
                        await uploadChunked(file);
                    }}
                    statusEl.className = "status success";
                    statusEl.textContent = "{success_msg}";
                    progressFill.style.background = "#4caf50";
                    selectedFiles = [];
                    updateUI();
                }} else {{
                    const result = await uploadLegacy();
                    if (result.success) {{
                        statusEl.className = "status success";
                        statusEl.textContent = "{success_msg}";
                        progressFill.style.width = "100%";
                        progressFill.style.background = "#4caf50";
                        selectedFiles = [];
                        updateUI();
                    }} else {{
                        statusEl.className = "status error";
                        statusEl.textContent = result.message || "{failed_msg}";
                        uploadBtn.disabled = false;
                    }}
                }}
            }} catch(err) {{
                statusEl.className = "status error";
                statusEl.textContent = "{failed_msg}: " + err.message;
                uploadBtn.disabled = false;
            }}
        }});

        initEnhanced();
    </script>
</body>
</html>"##,
        lang = lang,
        title = title,
        select_files = select_files,
        drag_hint = drag_hint,
        upload_btn = upload_btn,
        transferring = transferring,
        success_msg = success_msg,
        failed_msg = failed_msg,
        file_label = file_label,
        total_size_label = total_size_label,
        remove_label = remove_label,
        encrypted_label = encrypted_label,
    )
}

fn generate_waiting_page(is_english: bool) -> String {
    let title = if is_english { "PureSend - Waiting" } else { "PureSend - 等待中" };
    let waiting_text = if is_english { "Waiting for approval..." } else { "等待接收方确认..." };
    let waiting_desc = if is_english { "Your upload request has been sent. Please wait for the receiver to approve." } else { "您的上传请求已发送，请等待接收方确认。" };
    let rejected_text = if is_english { "Access denied" } else { "访问被拒绝" };

    format!(
        r##"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="icon" type="image/png" href="/favicon.ico">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #f5f5f5; color: #333; min-height: 100vh; display: flex; align-items: center; justify-content: center; }}
        .container {{ max-width: 420px; width: 100%; padding: 20px; text-align: center; }}
        .card {{ background: #fff; border-radius: 16px; padding: 48px 32px; box-shadow: 0 2px 12px rgba(0,0,0,0.08); }}
        .icon {{ font-size: 64px; margin-bottom: 20px; }}
        h1 {{ font-size: 22px; font-weight: 600; margin-bottom: 12px; }}
        .desc {{ color: #666; font-size: 14px; line-height: 1.6; }}
        .spinner {{ display: inline-block; width: 32px; height: 32px; border: 3px solid #e0e0e0; border-top-color: #1976d2; border-radius: 50%; animation: spin 0.8s linear infinite; margin-top: 24px; }}
        @keyframes spin {{ to {{ transform: rotate(360deg); }} }}
        .rejected {{ display: none; color: #c62828; margin-top: 20px; padding: 16px; background: #ffebee; border-radius: 10px; }}
        @media (prefers-color-scheme: dark) {{
            body {{ background: #121212; color: #e0e0e0; }}
            .card {{ background: #1e1e1e; box-shadow: 0 2px 12px rgba(0,0,0,0.3); }}
            .desc {{ color: #aaa; }}
            .spinner {{ border-color: #444; border-top-color: #42a5f5; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <div class="icon">⏳</div>
            <h1 id="statusTitle">{waiting_text}</h1>
            <p class="desc" id="statusDesc">{waiting_desc}</p>
            <div class="spinner" id="spinner"></div>
            <div class="rejected" id="rejectedMsg">{rejected_text}</div>
        </div>
    </div>
    <script>
        (function() {{
            const poll = async () => {{
                try {{
                    const res = await fetch("/request-status");
                    const data = await res.json();
                    if (data.status === "accepted") {{
                        window.location.reload();
                    }} else if (data.status === "rejected") {{
                        document.getElementById("statusTitle").textContent = "{rejected_text}";
                        document.getElementById("statusDesc").style.display = "none";
                        document.getElementById("spinner").style.display = "none";
                        document.getElementById("rejectedMsg").style.display = "block";
                    }} else {{
                        setTimeout(poll, 1500);
                    }}
                }} catch {{
                    setTimeout(poll, 3000);
                }}
            }};
            poll();
        }})();
    </script>
</body>
</html>"##,
        lang = if is_english { "en" } else { "zh-CN" },
        title = title,
        waiting_text = waiting_text,
        waiting_desc = waiting_desc,
        rejected_text = rejected_text,
    )
}

fn generate_rejected_page(is_english: bool) -> String {
    let title = if is_english { "PureSend - Access Denied" } else { "PureSend - 访问被拒绝" };
    let rejected_text = if is_english { "Access Denied" } else { "访问被拒绝" };
    let rejected_desc = if is_english { "Your upload request has been rejected by the receiver." } else { "您的上传请求已被接收方拒绝。" };

    format!(
        r##"<!DOCTYPE html>
<html lang="{lang}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="icon" type="image/png" href="/favicon.ico">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; background: #f5f5f5; color: #333; min-height: 100vh; display: flex; align-items: center; justify-content: center; }}
        .container {{ max-width: 420px; width: 100%; padding: 20px; text-align: center; }}
        .card {{ background: #fff; border-radius: 16px; padding: 48px 32px; box-shadow: 0 2px 12px rgba(0,0,0,0.08); }}
        .icon {{ font-size: 64px; margin-bottom: 20px; }}
        h1 {{ font-size: 22px; font-weight: 600; margin-bottom: 12px; color: #c62828; }}
        .desc {{ color: #666; font-size: 14px; line-height: 1.6; }}
        @media (prefers-color-scheme: dark) {{
            body {{ background: #121212; color: #e0e0e0; }}
            .card {{ background: #1e1e1e; box-shadow: 0 2px 12px rgba(0,0,0,0.3); }}
            .desc {{ color: #aaa; }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <div class="icon">🚫</div>
            <h1>{rejected_text}</h1>
            <p class="desc">{rejected_desc}</p>
        </div>
    </div>
</body>
</html>"##,
        lang = if is_english { "en" } else { "zh-CN" },
        title = title,
        rejected_text = rejected_text,
        rejected_desc = rejected_desc,
    )
}
