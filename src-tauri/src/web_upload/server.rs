//! Web 上传 HTTP 服务器实现
//!
//! 提供文件上传的 HTTP 服务，采用按 IP 审批模式

use axum::extract::DefaultBodyLimit;
use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, Multipart, State as AxumState},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use super::models::{UploadRecord, UploadRequest, UploadRequestStatus, WebUploadState};

/// Favicon 图标数据
static FAVICON_ICO: &[u8] = include_bytes!("../../icons/32x32.png");

/// 上传服务器状态
#[derive(Debug)]
pub struct UploadServerState {
    /// Web 上传状态
    pub upload_state: Arc<Mutex<WebUploadState>>,
    /// Tauri 应用句柄
    pub app_handle: AppHandle,
}

/// Web 上传服务器实例
pub struct WebUploadServer {
    /// 监听地址
    pub addr: SocketAddr,
    /// 服务器状态
    pub state: Arc<UploadServerState>,
    /// 关闭信号
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl WebUploadServer {
    /// 创建新的上传服务器
    pub fn new(upload_state: Arc<Mutex<WebUploadState>>, app_handle: AppHandle) -> Self {
        let addr = SocketAddr::from(([0, 0, 0, 0], 0));

        Self {
            addr,
            state: Arc::new(UploadServerState {
                upload_state,
                app_handle,
            }),
            shutdown_tx: None,
        }
    }

    /// 启动服务器
    pub async fn start(&mut self) -> Result<u16, String> {
        let app = Router::new()
            .route("/", get(index_handler))
            .route("/favicon.ico", get(favicon_handler))
            .route("/request-status", get(request_status_handler))
            .route(
                "/upload",
                post(upload_handler).layer(DefaultBodyLimit::max(10 * 1024 * 1024 * 1024)),
            )
            .fallback(fallback_handler)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
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

    /// 停止服务器
    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Favicon 处理器
async fn favicon_handler() -> impl IntoResponse {
    let mut response = Response::new(Body::from(FAVICON_ICO));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, "image/png".parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "max-age=86400".parse().unwrap());
    response
}

/// 首页处理器 - 按 IP 检查审批状态，决定显示上传页面或等待页面
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

    // 检查该 IP 是否已被拒绝
    if upload_state.is_ip_rejected(&client_ip) {
        return Html(generate_rejected_page(is_english)).into_response();
    }

    // 检查该 IP 是否已有请求记录
    let has_request = upload_state
        .requests
        .values()
        .any(|r| r.client_ip == client_ip);

    if !has_request {
        if upload_state.auto_receive {
            // 自动接收：创建已接受的请求，添加到 allowed_ips
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
            // 需要审批：创建待处理的请求
            let mut request = UploadRequest::new(client_ip.clone());
            request.user_agent = user_agent;
            upload_state
                .requests
                .insert(request.id.clone(), request.clone());
            let _ = state.app_handle.emit("web-upload-task", &request);
        }
    }

    // 检查是否有上传权限
    let is_allowed = upload_state.is_ip_allowed(&client_ip);

    if is_allowed {
        Html(generate_upload_page(is_english)).into_response()
    } else {
        Html(generate_waiting_page(is_english)).into_response()
    }
}

/// 请求状态响应
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestStatusResponse {
    /// 是否存在请求记录
    has_request: bool,
    /// 请求状态
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

/// 上传 API 响应
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    success: bool,
    message: String,
}

/// 文件上传开始事件
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FileStartEvent {
    request_id: String,
    record_id: String,
    file_name: String,
    total_bytes: u64,
    client_ip: String,
}

/// 文件上传进度事件
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

/// 文件上传完成事件
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct FileCompleteEvent {
    request_id: String,
    record_id: String,
    file_name: String,
    total_bytes: u64,
    status: String,
}

/// 请求状态轮询处理器
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

/// 文件上传处理器（按 IP 授权，接收 multipart 文件数据）
async fn upload_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<UploadServerState>>,
    mut multipart: Multipart,
) -> Json<UploadResponse> {
    let client_ip = client_addr.ip().to_string();

    // 检查该 IP 是否已授权
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

        // 创建 UploadRecord 并添加到请求中
        let record = UploadRecord {
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

        // 发送文件开始事件
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

        // 确定文件保存路径
        let mut file_path = receive_dir.join(&file_name);
        if !file_overwrite && file_path.exists() {
            file_path = get_unique_path(&file_path);
        }

        // 流式接收文件数据并写入磁盘
        let start_time = std::time::Instant::now();
        let total_written: u64;

        match tokio::fs::File::create(&file_path).await {
            Ok(mut output_file) => {
                use tokio::io::AsyncWriteExt;

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

                            // 更新记录状态
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

                        // 发送进度事件
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

        // 更新记录为完成状态
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

        // 发送文件完成事件
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

/// 获取唯一文件路径（避免覆盖）
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

/// 404 处理器
async fn fallback_handler() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

/// 生成上传页面 HTML（已授权 IP 直接上传，无需审批流程）
fn generate_upload_page(is_english: bool) -> String {
    let title = if is_english {
        "PureSend - Upload Files"
    } else {
        "PureSend - 文件上传"
    };
    let select_files = if is_english {
        "Select Files"
    } else {
        "选择文件"
    };
    let drag_hint = if is_english {
        "or drag and drop files here"
    } else {
        "或将文件拖拽到此处"
    };
    let upload_btn = if is_english { "Upload" } else { "上传" };
    let transferring = if is_english {
        "Uploading files..."
    } else {
        "正在上传文件..."
    };
    let success = if is_english {
        "Files uploaded successfully!"
    } else {
        "文件上传成功！"
    };
    let failed = if is_english {
        "Upload failed"
    } else {
        "上传失败"
    };
    let file_label = if is_english { "file(s)" } else { "个文件" };
    let total_size_label = if is_english {
        "Total size"
    } else {
        "总大小"
    };
    let remove_label = if is_english { "Remove" } else { "移除" };

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

            <div class="drop-zone" id="dropZone">
                <div class="drop-zone-icon">📁</div>
                <div class="drop-zone-text">{drag_hint}</div>
                <button class="drop-zone-btn" onclick="document.getElementById('fileInput').click()">{select_files}</button>
                <input type="file" id="fileInput" multiple style="display:none" />
            </div>

            <div class="file-list hidden" id="fileList"></div>
            <div class="stats hidden" id="stats"></div>

            <button class="upload-btn" id="uploadBtn" disabled>{upload_btn}</button>

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
        let selectedFiles = [];

        function formatSize(bytes) {{
            if (bytes === 0) return "0 B";
            const k = 1024, sizes = ["B", "KB", "MB", "GB"];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
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

        function removeFile(index) {{
            selectedFiles.splice(index, 1);
            updateUI();
        }}

        function addFiles(files) {{
            for (const file of files) {{
                if (!selectedFiles.some(f => f.name === file.name && f.size === file.size)) {{
                    selectedFiles.push(file);
                }}
            }}
            statusEl.className = "status";
            statusEl.textContent = "";
            updateUI();
        }}

        dropZone.addEventListener("dragover", (e) => {{ e.preventDefault(); dropZone.classList.add("dragover"); }});
        dropZone.addEventListener("dragleave", () => {{ dropZone.classList.remove("dragover"); }});
        dropZone.addEventListener("drop", (e) => {{ e.preventDefault(); dropZone.classList.remove("dragover"); addFiles(e.dataTransfer.files); }});
        fileInput.addEventListener("change", () => {{ addFiles(fileInput.files); fileInput.value = ""; }});

        uploadBtn.addEventListener("click", async () => {{
            if (selectedFiles.length === 0) return;
            uploadBtn.disabled = true;
            statusEl.className = "status uploading";
            statusEl.textContent = "{transferring}";
            statusEl.style.display = "block";

            const formData = new FormData();
            selectedFiles.forEach(file => formData.append("files", file));

            try {{
                const response = await fetch("/upload", {{ method: "POST", body: formData }});
                const result = await response.json();

                if (result.success) {{
                    statusEl.className = "status success";
                    statusEl.textContent = "{success}";
                    selectedFiles = [];
                    updateUI();
                }} else {{
                    statusEl.className = "status error";
                    statusEl.textContent = result.message || "{failed}";
                }}
            }} catch (err) {{
                statusEl.className = "status error";
                statusEl.textContent = "{failed}: " + err.message;
                uploadBtn.disabled = false;
            }}
        }});
    </script>
</body>
</html>"##,
        lang = if is_english { "en" } else { "zh-CN" },
        title = title,
        select_files = select_files,
        drag_hint = drag_hint,
        upload_btn = upload_btn,
        transferring = transferring,
        success = success,
        failed = failed,
        file_label = file_label,
        total_size_label = total_size_label,
        remove_label = remove_label,
    )
}

/// 生成等待响应页面 HTML
fn generate_waiting_page(is_english: bool) -> String {
    let title = if is_english {
        "PureSend - Waiting"
    } else {
        "PureSend - 等待中"
    };
    let waiting_text = if is_english {
        "Waiting for approval..."
    } else {
        "等待接收方确认..."
    };
    let waiting_desc = if is_english {
        "Your upload request has been sent. Please wait for the receiver to approve."
    } else {
        "您的上传请求已发送，请等待接收方确认。"
    };
    let rejected_text = if is_english {
        "Access denied"
    } else {
        "访问被拒绝"
    };

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

/// 生成访问被拒绝页面 HTML
fn generate_rejected_page(is_english: bool) -> String {
    let title = if is_english {
        "PureSend - Access Denied"
    } else {
        "PureSend - 访问被拒绝"
    };
    let rejected_text = if is_english {
        "Access Denied"
    } else {
        "访问被拒绝"
    };
    let rejected_desc = if is_english {
        "Your upload request has been rejected by the receiver."
    } else {
        "您的上传请求已被接收方拒绝。"
    };

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
