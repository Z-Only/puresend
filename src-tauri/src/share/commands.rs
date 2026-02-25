//! 分享相关 Tauri 命令

use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use super::models::{AccessRequest, ShareLinkInfo, ShareSettings, ShareState};
use super::server::ShareServer;
use crate::models::FileMetadata;

/// 分享管理器状态
pub struct ShareManagerState {
    /// 分享状态
    pub share_state: Arc<Mutex<ShareState>>,
    /// HTTP 服务器
    pub server: Arc<Mutex<Option<ShareServer>>>,
}

impl ShareManagerState {
    pub fn new() -> Self {
        Self {
            share_state: Arc::new(Mutex::new(ShareState::new())),
            server: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for ShareManagerState {
    fn default() -> Self {
        Self::new()
    }
}

/// 开始分享
#[tauri::command]
pub async fn start_share(
    app: AppHandle,
    state: State<'_, ShareManagerState>,
    files: Vec<FileMetadata>,
    settings: ShareSettings,
) -> Result<ShareLinkInfo, String> {
    // 验证文件存在性并收集路径
    let mut file_paths: Vec<(FileMetadata, PathBuf)> = Vec::new();
    // 保存验证通过的文件副本用于后续创建分享信息
    let mut valid_files: Vec<FileMetadata> = Vec::new();

    for file in files {
        if let Some(path_str) = &file.path {
            let path = PathBuf::from(path_str);
            if !path.exists() {
                return Err(format!("文件不存在：{}", path_str));
            }
            file_paths.push((file.clone(), path));
            valid_files.push(file);
        } else {
            return Err(format!("文件路径未设置：{}", file.name));
        }
    }

    // 允许空文件列表启动分享服务（Web 下载模式下可以先启动服务，后续再添加文件）

    // 创建并启动服务器
    let mut server = ShareServer::new(state.share_state.clone(), app, 0); // 自动分配端口

    let actual_port = server.start(file_paths).await?;

    // 获取本机 IP 地址
    let local_ip = get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    // 生成简洁的 URL 格式，只包含协议、IP 和端口
    let link = format!("http://{}:{}", local_ip, actual_port);

    // 创建分享信息
    let mut share_info = ShareLinkInfo::new(link, actual_port, valid_files);

    // 先克隆需要的值，避免所有权问题
    let pin_clone = settings.pin.clone();
    if settings.pin_enabled {
        if let Some(pin) = pin_clone {
            share_info = share_info.with_pin(pin);
        }
    }

    share_info = share_info.with_auto_accept(settings.auto_accept);

    // 更新分享状态，同时传入设置信息
    {
        let mut share_state = state.share_state.lock().await;
        share_state.start_share(share_info.clone(), settings);
    }

    // 保存服务器实例
    {
        let mut server_guard = state.server.lock().await;
        *server_guard = Some(server);
    }

    Ok(share_info)
}

/// 停止分享
#[tauri::command]
pub async fn stop_share(state: State<'_, ShareManagerState>) -> Result<(), String> {
    // 停止服务器
    {
        let mut server_guard = state.server.lock().await;
        if let Some(mut server) = server_guard.take() {
            server.stop();
        }
    }

    // 清理分享状态
    {
        let mut share_state = state.share_state.lock().await;
        share_state.stop_share();
    }

    Ok(())
}

/// 获取分享信息
#[tauri::command]
pub async fn get_share_info(
    state: State<'_, ShareManagerState>,
) -> Result<Option<ShareLinkInfo>, String> {
    let share_state = state.share_state.lock().await;
    Ok(share_state.share_info.clone())
}

/// 获取访问请求列表
#[tauri::command]
pub async fn get_access_requests(
    state: State<'_, ShareManagerState>,
) -> Result<Vec<AccessRequest>, String> {
    let share_state = state.share_state.lock().await;
    Ok(share_state.access_requests.values().cloned().collect())
}

/// 接受访问请求
#[tauri::command]
pub async fn accept_access_request(
    app: AppHandle,
    state: State<'_, ShareManagerState>,
    request_id: String,
) -> Result<(), String> {
    let mut share_state = state.share_state.lock().await;

    if let Some(request) = share_state.accept_request(&request_id) {
        // 发送事件通知（使用克隆的请求数据，避免借用问题）
        let request_clone = request.clone();
        let _ = app.emit("access-request-accepted", request_clone);
    } else {
        return Err("请求不存在".to_string());
    }

    Ok(())
}

/// 拒绝访问请求
#[tauri::command]
pub async fn reject_access_request(
    app: AppHandle,
    state: State<'_, ShareManagerState>,
    request_id: String,
) -> Result<(), String> {
    let mut share_state = state.share_state.lock().await;

    if let Some(request) = share_state.reject_request(&request_id) {
        // 发送事件通知（使用克隆的请求数据，避免借用问题）
        let request_clone = request.clone();
        let _ = app.emit("access-request-rejected", request_clone);
    } else {
        return Err("请求不存在".to_string());
    }

    Ok(())
}

/// 移除单个访问请求
#[tauri::command]
pub async fn remove_access_request(
    app: AppHandle,
    state: State<'_, ShareManagerState>,
    request_id: String,
) -> Result<(), String> {
    let mut share_state = state.share_state.lock().await;

    if share_state.remove_request(&request_id).is_some() {
        // 发送事件通知
        let _ = app.emit("access-request-removed", request_id);
    } else {
        return Err("请求不存在".to_string());
    }

    Ok(())
}

/// 移除所有访问请求
#[tauri::command]
pub async fn clear_access_requests(
    app: AppHandle,
    state: State<'_, ShareManagerState>,
) -> Result<(), String> {
    let mut share_state = state.share_state.lock().await;

    let removed_ids: Vec<String> = share_state.access_requests.keys().cloned().collect();

    share_state.access_requests.clear();

    // 发送事件通知
    for request_id in removed_ids {
        let _ = app.emit("access-request-removed", request_id);
    }

    Ok(())
}

/// 更新分享文件列表（动态同步已选文件到 HTTP 服务器）
#[tauri::command]
pub async fn update_share_files(
    state: State<'_, ShareManagerState>,
    files: Vec<FileMetadata>,
) -> Result<(), String> {
    // 验证文件存在性并收集路径
    let mut new_file_paths: Vec<(FileMetadata, std::path::PathBuf)> = Vec::new();
    let mut valid_files: Vec<FileMetadata> = Vec::new();

    for file in files {
        if let Some(path_str) = &file.path {
            let path = std::path::PathBuf::from(path_str);
            if !path.exists() {
                return Err(format!("文件不存在：{}", path_str));
            }
            new_file_paths.push((file.clone(), path));
            valid_files.push(file);
        } else {
            return Err(format!("文件路径未设置：{}", file.name));
        }
    }

    // 更新服务器的文件映射
    {
        let server_guard = state.server.lock().await;
        if let Some(server) = server_guard.as_ref() {
            let mut file_paths = server.state.file_paths.lock().await;
            let mut hash_to_filename = server.state.hash_to_filename.lock().await;

            // 清空旧映射
            file_paths.clear();
            hash_to_filename.clear();

            // 重建映射
            for (metadata, path) in new_file_paths {
                use sha2::{Digest, Sha256};
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
    }

    // 更新 share_state 中的文件列表
    {
        let mut share_state = state.share_state.lock().await;
        if let Some(ref mut share_info) = share_state.share_info {
            share_info.files = valid_files;
        }
    }

    Ok(())
}

/// 更新分享设置
#[tauri::command]
pub async fn update_share_settings(
    state: State<'_, ShareManagerState>,
    settings: ShareSettings,
) -> Result<(), String> {
    let mut share_state = state.share_state.lock().await;
    share_state.settings = settings;
    Ok(())
}

/// 获取本机 IP 地址
fn get_local_ip() -> Option<String> {
    use std::net::UdpSocket;

    // 尝试连接外部地址获取本机 IP（不会真正发送数据）
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|addr| addr.ip().to_string())
}
