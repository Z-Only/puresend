//! HTTP 服务器实现
//!
//! 提供文件分享的 HTTP 服务

use axum::{
    body::Body,
    extract::{connect_info::ConnectInfo, Path, State as AxumState},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use sha2::{Digest, Sha256};
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tauri::{AppHandle, Emitter};

use super::models::ShareState;
use crate::models::FileMetadata;

/// Favicon 图标数据（嵌入二进制）
/// 使用 32x32 PNG 格式，从项目图标转换
static FAVICON_ICO: &[u8] = include_bytes!("../../icons/32x32.png");

/// 分享服务器状态
#[derive(Debug)]
pub struct ServerState {
    /// 分享状态
    pub share_state: Arc<Mutex<ShareState>>,
    /// 分享的文件路径映射（哈希 ID -> 实际路径）
    pub file_paths: Arc<Mutex<std::collections::HashMap<String, PathBuf>>>,
    /// 哈希 ID 到文件名的映射（用于 HTML 显示）
    pub hash_to_filename: Arc<Mutex<std::collections::HashMap<String, String>>>,
    /// Tauri 应用句柄，用于发送事件
    pub app_handle: AppHandle,
}
/// 服务器实例
pub struct ShareServer {
    /// 监听地址
    pub addr: SocketAddr,
    /// 服务器状态
    pub state: Arc<ServerState>,
    /// 关闭信号
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl ShareServer {
    /// 创建新的分享服务器
    pub fn new(share_state: Arc<Mutex<ShareState>>, app_handle: AppHandle, port: u16) -> Self {
        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        Self {
            addr,
            state: Arc::new(ServerState {
                share_state,
                file_paths: Arc::new(Mutex::new(std::collections::HashMap::new())),
                hash_to_filename: Arc::new(Mutex::new(std::collections::HashMap::new())),
                app_handle,
            }),
            shutdown_tx: None,
        }
    }

    /// 启动服务器
    pub async fn start(&mut self, files: Vec<(FileMetadata, PathBuf)>) -> Result<u16, String> {
        // 更新文件路径映射，使用文件路径的 SHA256 哈希值作为 ID
        {
            let mut file_paths = self.state.file_paths.lock().await;
            let mut hash_to_filename = self.state.hash_to_filename.lock().await;
            for (metadata, path) in files {
                // 使用文件路径的 SHA256 哈希值作为下载 ID，隐藏真实路径
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

        // 创建路由
        let app = Router::new()
            .route("/", get(index_handler))
            .route("/favicon.ico", get(favicon_handler))
            .route("/api/files", get(list_files_handler))
            .route("/api/verify-pin", post(verify_pin_handler))
            .route("/api/request-status", get(request_status_handler))
            .route("/download/{file_id}", get(download_handler))
            .fallback(fallback_handler)
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
            .with_state(self.state.clone());

        // 绑定端口（如果端口为0则自动分配）
        let listener = tokio::net::TcpListener::bind(self.addr)
            .await
            .map_err(|e| format!("绑定端口失败: {}", e))?;

        let actual_port = listener
            .local_addr()
            .map_err(|e| format!("获取端口失败: {}", e))?
            .port();

        // 创建关闭通道
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        // 启动服务器，使用 into_make_service_with_connect_info 来支持 ConnectInfo
        tokio::spawn(async move {
            axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
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

/// 首页处理器
async fn index_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumState(state): AxumState<Arc<ServerState>>,
) -> Response {
    let client_ip = client_addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| parse_user_agent(s))
        .unwrap_or_default();

    // 首先检查分享是否活跃并获取必要信息
    {
        let share_state = state.share_state.lock().await;
        if share_state.share_info.is_none() {
            return Html("<html><body><h1>分享已结束</h1></body></html>").into_response();
        }
    }

    // 检查是否被拒绝
    {
        let share_state = state.share_state.lock().await;
        if share_state.is_ip_rejected(&client_ip) {
            return Html("<html><body><h1>访问被拒绝</h1></body></html>").into_response();
        }
    }

    // 检查是否需要 PIN 或需要自动创建请求
    {
        let mut share_state = state.share_state.lock().await;
        
        let has_pin = share_state.settings.pin.is_some() && !share_state.settings.pin.as_ref().map_or(true, String::is_empty);
        let _is_verified = share_state.is_ip_verified(&client_ip);
        let has_request = share_state.access_requests.values().any(|r| r.ip == client_ip);
        let has_access = share_state.is_ip_allowed(&client_ip);
                // 如果没有 PIN 且开启了自动接受，且没有请求记录，自动创建已接受的请求
        if !has_pin && share_state.settings.auto_accept && !has_request {
            let mut new_request = super::models::AccessRequest::new(client_ip.clone(), Some(user_agent.to_string()));
            new_request.status = super::models::AccessRequestStatus::Accepted;
            share_state.access_requests.insert(new_request.id.clone(), new_request.clone());
            
            // 添加到已验证 IP 列表
            if !share_state.verified_ips.contains(&client_ip) {
                share_state.verified_ips.push(client_ip.clone());
            }
            
            // 发送事件通知前端
            let _ = state.app_handle.emit("access-request", new_request);
            // 同时发送已接受事件
            let _ = state.app_handle.emit("access-request-accepted", 
                share_state.access_requests.values().find(|r| r.ip == client_ip).cloned());
        }
        
        // 如果没有 PIN 且没有开启自动接受，且没有请求记录，创建待处理的请求
        if !has_pin && !share_state.settings.auto_accept && !has_request {
            let new_request = super::models::AccessRequest::new(client_ip.clone(), Some(user_agent.to_string()));
            share_state.access_requests.insert(new_request.id.clone(), new_request.clone());
            
            // 发送事件通知前端有新的访问请求
            let _ = state.app_handle.emit("access-request", new_request);
        }
        
        // 如果没有访问权限，显示等待响应页面
        if !has_access && !share_state.settings.auto_accept {
            return Html(WAITING_RESPONSE_HTML).into_response();
        }
    }

    // 重新获取状态检查访问权限
    let share_state = state.share_state.lock().await;
    let has_access = share_state.is_ip_allowed(&client_ip);
    
    if !has_access {
        return Html(WAITING_RESPONSE_HTML).into_response();
    }
    
    // 有访问权限，显示文件列表页面
    let share_info = share_state.share_info.as_ref().unwrap();
    
    // 构建哈希 ID 到文件名的映射，用于 HTML 显示
    let hash_to_name = {
        let hash_to_filename = state.hash_to_filename.lock().await;
        hash_to_filename.clone()
    };
    
    let files_html = hash_to_name
        .iter()
        .map(|(hash_id, file_name)| {
            // 从 share_info 中查找文件大小
            let file_size = share_info
                .files
                .iter()
                .find(|f| {
                    // 通过文件名匹配
                    f.name == *file_name
                })
                .map(|f| f.size)
                .unwrap_or(0);
            
            format!(
                r#"<li><a href="/download/{}" download="{}">{}</a> ({})</li>"#,
                hash_id,
                file_name,
                file_name,
                format_size(file_size)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let html = format!(
        r#"<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>PureSend - 文件分享</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #333; }}
        ul {{ list-style: none; padding: 0; }}
        li {{ padding: 10px; border-bottom: 1px solid #eee; }}
        a {{ color: #1976d2; text-decoration: none; }}
        a:hover {{ text-decoration: underline; }}
        .warning {{ background: #fff3cd; padding: 10px; border-radius: 4px; margin-bottom: 20px; }}
    </style>
</head>
<body>
    <h1>PureSend 文件分享</h1>
    <div class="warning">
        ⚠️ 此链接仅限可信网络内使用，请勿分享到公共平台
    </div>
    <h2>可用文件</h2>
    <ul>
        {}
    </ul>
</body>
</html>"#,
        files_html
    );

    Html(html).into_response()
}

/// 文件列表 API
async fn list_files_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<ServerState>>,
) -> impl IntoResponse {
    let share_state = state.share_state.lock().await;

    if share_state.share_info.is_none() {
        return (StatusCode::NOT_FOUND, Json(FilesResponse { 
            files: vec![],
            waiting_response: None
        }));
    }

    let client_ip = client_addr.ip().to_string();

    // 检查访问权限
    if share_state.is_ip_rejected(&client_ip) {
        return (StatusCode::FORBIDDEN, Json(FilesResponse { 
            files: vec![],
            waiting_response: None
        }));
    }

    // 检查是否需要 PIN（只要设置了 PIN 码且未验证就需要）
    let has_pin = share_state.settings.pin.is_some() && !share_state.settings.pin.as_ref().map_or(true, String::is_empty);
    let is_verified = share_state.is_ip_verified(&client_ip);
    
    // 检查是否已有访问请求（无论状态如何）
    let has_request = share_state.access_requests.values().any(|r| r.ip == client_ip);
    
    // 如果需要 PIN 且没有请求记录，才返回未授权
    let needs_pin = has_pin && !is_verified && !has_request;
    
    if needs_pin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(FilesResponse { 
                files: vec![],
                waiting_response: None
            }),
        );
    }
    
    // 检查是否有访问权限（请求已被接受）
    let has_access = share_state.is_ip_allowed(&client_ip);
    
    // 如果没有访问权限，返回等待响应状态
    if !has_access {
        return (
            StatusCode::ACCEPTED,
            Json(FilesResponse { 
                files: vec![],
                waiting_response: Some(true)
            }),
        );
    }

    let share_info = share_state.share_info.as_ref().unwrap();
    let files: Vec<FileInfo> = share_info
        .files
        .iter()
        .map(|f| FileInfo {
            id: f.id.clone(),
            name: f.name.clone(),
            size: f.size,
            mime_type: f.mime_type.clone(),
        })
        .collect();

    (StatusCode::OK, Json(FilesResponse { 
        files,
        waiting_response: None
    }))
}

/// PIN 验证请求
#[derive(Debug, Deserialize)]
struct VerifyPinRequest {
    pin: String,
}

/// PIN 验证处理器
async fn verify_pin_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<ServerState>>,
    Json(payload): Json<VerifyPinRequest>,
) -> impl IntoResponse {
    let client_ip = client_addr.ip().to_string();
    let mut share_state = state.share_state.lock().await;

    let result = share_state.verify_pin(&client_ip, &payload.pin);

    if result.success {
        // 获取新创建的访问请求并发送事件通知
        if let Some(request) = share_state.access_requests.values().find(|r| r.ip == client_ip) {
            // 发送新请求事件（无论状态如何都发送，让前端知道有新请求）
            let _ = state.app_handle.emit("access-request", request.clone());
            
            // 如果是自动接受，同时发送接受事件
            if request.status == super::models::AccessRequestStatus::Accepted {
                let _ = state.app_handle.emit("access-request-accepted", request.clone());
            }
        }
        (StatusCode::OK, Json(result))
    } else {
        (StatusCode::UNAUTHORIZED, Json(result))
    }
}

/// 请求状态响应
#[derive(Debug, Serialize)]
struct RequestStatusResponse {
    has_request: bool,
    status: Option<String>,
    waiting_response: bool,
}

/// 请求状态处理器
async fn request_status_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumState(state): AxumState<Arc<ServerState>>,
) -> impl IntoResponse {
    let client_ip = client_addr.ip().to_string();
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| parse_user_agent(s))
        .unwrap_or_default();
    let mut share_state = state.share_state.lock().await;

    // 查找该 IP 的请求
    let request = share_state.access_requests.values().find(|r| r.ip == client_ip);

    let response = match request {
        Some(req) => {
            // 使用小写格式返回状态，与前端期望一致
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
        },
        None => {
            // 检查是否是自动接受模式
            let auto_accept = share_state.settings.auto_accept;
            let has_pin = share_state.settings.pin.is_some() && !share_state.settings.pin.as_ref().map_or(true, String::is_empty);
            let is_verified = share_state.is_ip_verified(&client_ip);
            
            // 如果是自动接受模式且没有 PIN，自动创建已接受的请求
            if auto_accept && !has_pin && !is_verified {
                let mut new_request = super::models::AccessRequest::new(client_ip.clone(), Some(user_agent.to_string()));
                new_request.status = super::models::AccessRequestStatus::Accepted;
                share_state.access_requests.insert(new_request.id.clone(), new_request.clone());
                
                // 添加到已验证 IP 列表
                if !share_state.verified_ips.contains(&client_ip) {
                    share_state.verified_ips.push(client_ip.clone());
                }
                
                // 发送事件通知前端
                let _ = state.app_handle.emit("access-request", new_request.clone());
                let _ = state.app_handle.emit("access-request-accepted", new_request);
                
                RequestStatusResponse {
                    has_request: true,
                    status: Some("accepted".to_string()),
                    waiting_response: false,
                }
            } else if is_verified {
                // 如果 IP 已验证（可能是通过 PIN 验证但没有创建请求的情况）
                RequestStatusResponse {
                    has_request: true,
                    status: Some("accepted".to_string()),
                    waiting_response: false,
                }
            } else {
                // 其他情况：没有请求记录
                RequestStatusResponse {
                    has_request: false,
                    status: None,
                    waiting_response: false,
                }
            }
        },
    };

    (StatusCode::OK, Json(response))
}

/// 回退处理器 - 用于调试未匹配的路由
async fn fallback_handler(uri: axum::http::Uri) -> impl IntoResponse {
    eprintln!("未匹配的路由: {}", uri);
    (StatusCode::NOT_FOUND, Html(format!(
        "<html><body><h1>404 - 路由未找到</h1><p>请求的路径: {}</p></body></html>",
        uri
    )))
}

/// 文件下载处理器
async fn download_handler(
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    AxumState(state): AxumState<Arc<ServerState>>,
    Path(file_id): Path<String>,
) -> Response {
    let client_ip = client_addr.ip().to_string();

    // 调试：记录下载请求
    eprintln!("下载请求开始 - client_ip: {}, file_id: {}", client_ip, file_id);

    // 检查访问权限
    {
        let share_state = state.share_state.lock().await;

        if share_state.share_info.is_none() {
            eprintln!("下载失败 - 分享已结束");
            return Html("<html><body><h1>分享已结束</h1></body></html>").into_response();
        }

        if share_state.is_ip_rejected(&client_ip) {
            eprintln!("下载失败 - IP 被拒绝: {}", client_ip);
            return Html("<html><body><h1>访问被拒绝</h1></body></html>").into_response();
        }

        // 检查是否需要 PIN（只要设置了 PIN 码且未验证就需要）
        let has_pin = share_state.settings.pin.is_some() && !share_state.settings.pin.as_ref().map_or(true, String::is_empty);
        let is_verified = share_state.is_ip_verified(&client_ip);
        let needs_pin = has_pin && !is_verified;
        
        // 如果需要 PIN，优先显示提示
        if needs_pin {
            eprintln!("下载失败 - 需要 PIN 验证: {}", client_ip);
            return Html("<html><body><h1>需要验证 PIN</h1></body></html>").into_response();
        }
        
        // 检查是否有访问权限
        let has_access = share_state.is_ip_allowed(&client_ip);
        
        eprintln!("下载权限检查 - client_ip: {}, has_access: {}, auto_accept: {}", 
            client_ip, has_access, share_state.settings.auto_accept);
        
        // 如果没有访问权限，显示等待响应
        if !has_access {
            eprintln!("下载失败 - 没有访问权限: {}", client_ip);
            return Html("<html><body><h1>等待访问授权中，请稍后重试</h1><p>请先在分享方接受您的访问请求</p></body></html>").into_response();
        }
    }

    // 获取文件路径
    let file_path = {
        let file_paths = state.file_paths.lock().await;
        file_paths.get(&file_id).cloned()
    };

    // 调试：记录查找结果
    eprintln!("下载请求 - file_id: {}, 找到路径：{:?}", file_id, file_path);

    match file_path {
        Some(path) => {
            // 验证路径安全性（防止路径遍历攻击）
            if !path.exists() || !path.is_file() {
                eprintln!("文件不存在或不是文件：{:?}", path);
                return Html("<html><body><h1>文件不存在</h1></body></html>").into_response();
            }

            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download")
                .to_string();

            // 获取文件大小
            // 获取文件大小
            let file_size = std::fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(0);

            let mime_type = FileMetadata::infer_mime_type(&file_name);
            
            eprintln!("开始传输文件 - file_name: {}, mime_type: {}", file_name, mime_type);

            // 更新传输进度状态为传输中
            {
                let mut share_state = state.share_state.lock().await;
                let total_files = share_state.share_info.as_ref().map(|s| s.files.len() as u32).unwrap_or(1);
                if let Some(request) = share_state.access_requests.values_mut().find(|r| r.ip == client_ip) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    request.transfer_progress = Some(super::models::TransferProgress {
                        file_name: file_name.clone(),
                        downloaded_bytes: 0,
                        total_bytes: file_size,
                        progress: 0.0,
                        speed: 0,
                        completed_files: 0,
                        total_files,
                        status: super::models::TransferStatus::Transferring,
                        started_at: Some(now),
                        completed_at: None,
                    });
                }
            }

            // 发送下载开始事件到前端
            let _ = state.app_handle.emit("download-start", DownloadStartPayload {
                file_name: file_name.clone(),
                file_size: file_size as i64,
                client_ip: client_ip.clone(),
            });

            // 使用流式传输文件，避免大文件内存问题
            match File::open(&path).await {
                Ok(file) => {
                    let stream = ReaderStream::new(file);
                    let body = Body::from_stream(stream);

                    let mut response = Response::new(body);
                    *response.status_mut() = StatusCode::OK;
                    let headers = response.headers_mut();
                    // 安全地设置 Content-Type，避免 parse 失败
                    if let Ok(mime_header) = mime_type.parse() {
                        headers.insert(header::CONTENT_TYPE, mime_header);
                    } else {
                        headers.insert(header::CONTENT_TYPE, "application/octet-stream".parse().unwrap());
                    }
                    // 使用 URL 编码文件名以支持中文等特殊字符
                    let encoded_filename = urlencoding::encode(&file_name);
                    headers.insert(
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename*=UTF-8''{}", encoded_filename)
                            .parse()
                            .unwrap(),
                    );
                    
                    // 更新传输进度状态为已完成
                    {
                        let mut share_state = state.share_state.lock().await;
                        let total_files = share_state.share_info.as_ref().map(|s| s.files.len() as u32).unwrap_or(1);
                        if let Some(request) = share_state.access_requests.values_mut().find(|r| r.ip == client_ip) {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;
                            if let Some(ref mut progress) = request.transfer_progress {
                                progress.downloaded_bytes = file_size;
                                progress.progress = 100.0;
                                progress.status = super::models::TransferStatus::Completed;
                                progress.completed_at = Some(now);
                                progress.completed_files += 1;
                            } else {
                                request.transfer_progress = Some(super::models::TransferProgress {
                                    file_name: file_name.clone(),
                                    downloaded_bytes: file_size,
                                    total_bytes: file_size,
                                    progress: 100.0,
                                    speed: 0,
                                    completed_files: 1,
                                    total_files,
                                    status: super::models::TransferStatus::Completed,
                                    started_at: None,
                                    completed_at: Some(now),
                                });
                            }
                        }
                    }
                    
                    // 发送下载完成事件到前端
                    let _ = state.app_handle.emit("download-complete", DownloadCompletePayload {
                        file_name: file_name.clone(),
                        file_size: file_size as i64,
                        client_ip: client_ip.clone(),
                    });
                    
                    eprintln!("文件传输响应已发送 - file_name: {}", file_name);
                    return response;
                }
                Err(e) => {
                    // 更新传输进度状态为失败
                    {
                        let mut share_state = state.share_state.lock().await;
                        let total_files = share_state.share_info.as_ref().map(|s| s.files.len() as u32).unwrap_or(1);
                        if let Some(request) = share_state.access_requests.values_mut().find(|r| r.ip == client_ip) {
                            request.transfer_progress = Some(super::models::TransferProgress {
                                file_name: file_name.clone(),
                                downloaded_bytes: 0,
                                total_bytes: file_size,
                                progress: 0.0,
                                speed: 0,
                                completed_files: 0,
                                total_files,
                                status: super::models::TransferStatus::Failed,
                                started_at: None,
                                completed_at: None,
                            });
                        }
                    }
                    eprintln!("打开文件失败：{:?}", e);
                    let error_html =
                        format!("<html><body><h1>打开文件失败：{}</h1></body></html>", e);
                    return Html(error_html).into_response();
                }
            }
        }
        None => {
            eprintln!("文件 ID 不存在: {}", file_id);
            return Html("<html><body><h1>文件不存在</h1></body></html>").into_response();
        }
    }
}

/// 下载开始事件载荷
#[derive(Debug, Clone, Serialize)]
struct DownloadStartPayload {
    /// 文件名
    file_name: String,
    /// 文件大小
    file_size: i64,
    /// 客户端 IP
    client_ip: String,
}

/// 下载完成事件载荷
#[derive(Debug, Clone, Serialize)]
struct DownloadCompletePayload {
    /// 文件名
    file_name: String,
    /// 文件大小
    file_size: i64,
    /// 客户端 IP
    client_ip: String,
}

/// 文件信息响应
#[derive(Debug, Serialize)]
struct FilesResponse {
    files: Vec<FileInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    waiting_response: Option<bool>,
}

/// 文件信息
#[derive(Debug, Serialize)]
struct FileInfo {
    id: String,
    name: String,
    size: u64,
    mime_type: String,
}

/// 格式化文件大小
fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// 解析 User-Agent 为简短的浏览器/平台信息
/// 例如: "Chrome(Android)", "Safari(iOS)", "Firefox(Windows)"
fn parse_user_agent(ua: &str) -> &'static str {
    let ua_lower = ua.to_lowercase();
    
    // 检测平台
    let platform = if ua_lower.contains("android") {
        "Android"
    } else if ua_lower.contains("iphone") || ua_lower.contains("ipad") || ua_lower.contains("ipod") {
        "iOS"
    } else if ua_lower.contains("mac") || ua_lower.contains("macos") {
        "macOS"
    } else if ua_lower.contains("windows") || ua_lower.contains("win") {
        "Windows"
    } else if ua_lower.contains("linux") {
        "Linux"
    } else {
        "Unknown"
    };
    
    // 检测浏览器
    let browser = if ua_lower.contains("edg/") || ua_lower.contains("edge") {
        "Edge"
    } else if ua_lower.contains("opr/") || ua_lower.contains("opera") {
        "Opera"
    } else if ua_lower.contains("firefox") || ua_lower.contains("fxios") {
        "Firefox"
    } else if ua_lower.contains("chrome") || ua_lower.contains("crios") {
        "Chrome"
    } else if ua_lower.contains("safari") && !ua_lower.contains("chrome") {
        "Safari"
    } else if ua_lower.contains("msie") || ua_lower.contains("trident") {
        "IE"
    } else {
        "Unknown"
    };
    
    // 返回静态字符串，格式: "Browser(Platform)"
    match (browser, platform) {
        ("Chrome", "Android") => "Chrome(Android)",
        ("Chrome", "iOS") => "Chrome(iOS)",
        ("Chrome", "macOS") => "Chrome(macOS)",
        ("Chrome", "Windows") => "Chrome(Windows)",
        ("Chrome", "Linux") => "Chrome(Linux)",
        ("Chrome", _) => "Chrome",
        ("Safari", "iOS") => "Safari(iOS)",
        ("Safari", "macOS") => "Safari(macOS)",
        ("Safari", _) => "Safari",
        ("Firefox", "Android") => "Firefox(Android)",
        ("Firefox", "iOS") => "Firefox(iOS)",
        ("Firefox", _) => "Firefox",
        ("Edge", "Windows") => "Edge(Windows)",
        ("Edge", "macOS") => "Edge(macOS)",
        ("Edge", _) => "Edge",
        ("Opera", "Android") => "Opera(Android)",
        ("Opera", _) => "Opera",
        ("IE", _) => "IE",
        (_, "Android") => "Browser(Android)",
        (_, "iOS") => "Browser(iOS)",
        (_, _) => "Browser",
    }
}

/// PIN 输入页面模板（内嵌）
#[allow(dead_code)]
static PIN_INPUT_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>PureSend - PIN 验证</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }
        h1 { color: #333; margin-bottom: 20px; }
        .input-container { width: 100%; max-width: 300px; margin: 0 auto 15px; }
        input { width: 100%; padding: 12px; font-size: 18px; text-align: center; border: 1px solid #ccc; border-radius: 4px; box-sizing: border-box; }
        button { width: 100%; max-width: 300px; padding: 12px; background: #1976d2; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; }
        button:hover { background: #1565c0; }
        .error { color: #d32f2f; margin-top: 10px; }
    </style>
</head>
<body>
    <h1>请输入 PIN 码</h1>
    <div class="input-container">
        <input type="text" id="pin" placeholder="输入 PIN 码">
    </div>
    <button onclick="verify()">验证</button>
    <div id="error" class="error"></div>
    
    <script>
        async function verify() {
            const pin = document.getElementById('pin').value;
            const errorDiv = document.getElementById('error');
            
            if (!pin) {
                errorDiv.textContent = '请输入 PIN 码';
                return;
            }
            
            try {
                const response = await fetch('/api/verify-pin', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ pin })
                });
                
                const result = await response.json();
                
                if (result.success) {
                    window.location.reload();
                } else {
                    if (result.locked) {
                        errorDiv.textContent = '尝试次数过多，已锁定 5 分钟';
                    } else {
                        errorDiv.textContent = 'PIN 码错误，剩余尝试次数：' + (result.remaining_attempts || 0);
                    }
                }
            } catch (e) {
                errorDiv.textContent = '验证失败，请重试';
            }
        }
        
        document.getElementById('pin').addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                verify();
            }
        });
    </script>
</body>
</html>
"#;

/// 等待响应页面模板（内嵌）
static WAITING_RESPONSE_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="icon" type="image/png" href="/favicon.ico">
    <title>PureSend - 等待响应</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }
        h1 { color: #1976d2; }
        .spinner { border: 4px solid #f3f3f3; border-top: 4px solid #1976d2; border-radius: 50%; width: 40px; height: 40px; animation: spin 1s linear infinite; margin: 20px auto; }
        @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
        .message { color: #666; margin-top: 20px; }
        .status { margin-top: 15px; font-weight: bold; color: #1976d2; }
    </style>
</head>
<body>
    <h1>等待响应中</h1>
    <div class="spinner"></div>
    <div class="message">等待分享方接受您的访问请求...</div>
    <div class="status" id="status">正在检查状态...</div>
    <script>
        async function checkStatus() {
            try {
                const response = await fetch('/api/request-status');
                const result = await response.json();
                
                const statusDiv = document.getElementById('status');
                
                if (result.status === 'accepted') {
                    statusDiv.textContent = '✓ 已接受！正在跳转...';
                    statusDiv.style.color = '#4caf50';
                    // 请求已被接受，刷新页面显示文件
                    setTimeout(() => {
                        window.location.reload();
                    }, 1000);
                } else if (result.status === 'rejected') {
                    statusDiv.textContent = '✗ 访问请求被拒绝';
                    statusDiv.style.color = '#f44336';
                } else {
                    // 继续轮询（包括 waiting_response=true、status=null、status='pending' 等情况）
                    statusDiv.textContent = '等待分享方接受...';
                    setTimeout(checkStatus, 3000);
                }
            } catch (e) {
                console.error('检查状态失败:', e);
                setTimeout(checkStatus, 5000);
            }
        }
        
        // 开始检查状态
        checkStatus();
    </script>
</body>
</html>
"#;

/// PIN 输入页面模板（内嵌，旧版保留）
static _PIN_INPUT_HTML_OLD: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PureSend - PIN 验证</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 400px; margin: 100px auto; padding: 20px; text-align: center; }
        h1 { color: #333; }
        input { width: 100%; padding: 12px; font-size: 18px; text-align: center; margin: 10px 0; border: 1px solid #ccc; border-radius: 4px; }
        button { width: 100%; padding: 12px; background: #1976d2; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; }
        button:hover { background: #1565c0; }
        .error { color: #d32f2f; margin-top: 10px; }
    </style>
</head>"
<body>
    <h1>请输入 PIN 码</h1>
    <input type="text" id="pin" maxlength="6" pattern="[0-9]*" inputmode="numeric" placeholder="输入 PIN 码">
    <button onclick="verify()">验证</button>
    <div id="error" class="error"></div>
    
    <script>
        async function verify() {
            const pin = document.getElementById('pin').value;
            const errorDiv = document.getElementById('error');
            
            if (!pin) {
                errorDiv.textContent = '请输入 PIN 码';
                return;
            }
            
            try {
                const response = await fetch('/api/verify-pin', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ pin })
                });
                
                const result = await response.json();
                
                if (result.success) {
                    window.location.reload();
                } else {
                    if (result.locked) {
                        errorDiv.textContent = '尝试次数过多，已锁定 5 分钟';
                    } else {
                        errorDiv.textContent = 'PIN 码错误，剩余 ' + result.remainingAttempts + ' 次尝试';
                    }
                }
            } catch (e) {
                errorDiv.textContent = '验证失败，请重试';
            }
        }
        
        document.getElementById('pin').addEventListener('keypress', function(e) {
            if (e.key === 'Enter') verify();
        });
    </script>
</body>
</html>
"#;
