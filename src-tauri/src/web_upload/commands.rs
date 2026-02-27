//! Web 上传相关 Tauri 命令

use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use super::models::{UploadRequest, UploadRequestStatus, WebUploadState};
use super::server::WebUploadServer;

/// Web 上传管理器状态
pub struct WebUploadManagerState {
    /// Web 上传状态
    pub upload_state: Arc<Mutex<WebUploadState>>,
    /// HTTP 服务器
    pub server: Arc<Mutex<Option<WebUploadServer>>>,
}

impl WebUploadManagerState {
    pub fn new() -> Self {
        Self {
            upload_state: Arc::new(Mutex::new(WebUploadState::new())),
            server: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for WebUploadManagerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Web 上传服务器信息
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebUploadInfo {
    /// 是否已启动
    pub enabled: bool,
    /// 服务器端口
    pub port: u16,
    /// 上传链接列表
    pub urls: Vec<String>,
}

/// 启动 Web 上传服务器
#[tauri::command]
pub async fn start_web_upload(
    app: AppHandle,
    state: State<'_, WebUploadManagerState>,
    receive_directory: String,
    auto_receive: bool,
    file_overwrite: bool,
    preferred_port: Option<u16>,
) -> Result<WebUploadInfo, String> {
    // 如果已经启动，先停止
    {
        let mut server_guard = state.server.lock().await;
        if let Some(mut server) = server_guard.take() {
            server.stop();
        }
    }

    // 更新状态
    {
        let mut upload_state = state.upload_state.lock().await;
        upload_state.auto_receive = auto_receive;
        upload_state.file_overwrite = file_overwrite;
        upload_state.receive_directory = receive_directory;
        upload_state.requests.clear();
    }

    // 创建并启动服务器（优先使用首选端口，失败则自动分配）
    let port = preferred_port.unwrap_or(0);
    let mut server = WebUploadServer::new(state.upload_state.clone(), app.clone(), port);
    let actual_port = match server.start().await {
        Ok(p) => p,
        Err(_) if port != 0 => {
            server = WebUploadServer::new(state.upload_state.clone(), app, 0);
            server.start().await?
        }
        Err(e) => return Err(e),
    };

    // 获取本机 IP 地址
    let local_ips = crate::network::get_local_ips();
    let urls: Vec<String> = local_ips.iter().map(|ip| format!("http://{}:{}", ip, actual_port)).collect();

    // 保存服务器实例
    {
        let mut server_guard = state.server.lock().await;
        *server_guard = Some(server);
    }

    Ok(WebUploadInfo {
        enabled: true,
        port: actual_port,
        urls,
    })
}

/// 停止 Web 上传服务器
#[tauri::command]
pub async fn stop_web_upload(state: State<'_, WebUploadManagerState>) -> Result<(), String> {
    // 停止服务器
    {
        let mut server_guard = state.server.lock().await;
        if let Some(mut server) = server_guard.take() {
            server.stop();
        }
    }

    // 清理状态
    {
        let mut upload_state = state.upload_state.lock().await;
        upload_state.requests.clear();
        upload_state.allowed_ips.clear();
    }

    Ok(())
}

/// 获取 Web 上传请求列表
#[tauri::command]
pub async fn get_web_upload_requests(
    state: State<'_, WebUploadManagerState>,
) -> Result<Vec<UploadRequest>, String> {
    let upload_state = state.upload_state.lock().await;
    Ok(upload_state.requests.values().cloned().collect())
}

/// 同意 Web 上传请求（将该 IP 添加到 allowed_ips）
#[tauri::command]
pub async fn accept_web_upload(
    app: AppHandle,
    state: State<'_, WebUploadManagerState>,
    request_id: String,
) -> Result<(), String> {
    let mut upload_state = state.upload_state.lock().await;

    if !upload_state.requests.contains_key(&request_id) {
        return Err("请求不存在".to_string());
    }

    let client_ip;
    let request_clone;
    {
        let request = upload_state.requests.get_mut(&request_id).unwrap();
        request.status = UploadRequestStatus::Accepted;
        client_ip = request.client_ip.clone();
        request_clone = request.clone();
    }

    if !upload_state.allowed_ips.contains(&client_ip) {
        upload_state.allowed_ips.push(client_ip);
    }

    let _ = app.emit("web-upload-status-changed", &request_clone);
    Ok(())
}

/// 拒绝 Web 上传请求
#[tauri::command]
pub async fn reject_web_upload(
    app: AppHandle,
    state: State<'_, WebUploadManagerState>,
    request_id: String,
) -> Result<(), String> {
    let mut upload_state = state.upload_state.lock().await;

    if !upload_state.requests.contains_key(&request_id) {
        return Err("请求不存在".to_string());
    }

    let client_ip;
    let request_clone;
    {
        let request = upload_state.requests.get_mut(&request_id).unwrap();
        request.status = UploadRequestStatus::Rejected;
        client_ip = request.client_ip.clone();
        request_clone = request.clone();
    }

    upload_state.allowed_ips.retain(|ip| ip != &client_ip);

    let _ = app.emit("web-upload-status-changed", &request_clone);
    Ok(())
}

