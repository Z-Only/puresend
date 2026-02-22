//! 传输相关 Tauri 命令

use crate::models::{
    FileMetadata, TransferDirection, TransferMode, TransferProgress, TransferTask,
};
use crate::transfer::{FileChunker, IntegrityChecker, LocalTransport, Transport};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

/// 传输管理器状态
pub struct TransferState {
    /// 本地传输实例
    local_transport: Arc<Mutex<Option<LocalTransport>>>,
    /// 活跃的传输任务
    active_tasks: Arc<Mutex<HashMap<String, TransferTask>>>,
    /// 分块器
    chunker: FileChunker,
    /// 校验器
    checker: IntegrityChecker,
    /// 接收状态
    receiving_state: Arc<Mutex<ReceivingState>>,
}

/// 接收状态
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivingState {
    /// 是否正在接收
    pub is_receiving: bool,
    /// 监听端口
    pub port: u16,
    /// 网络地址
    pub network_address: String,
    /// 分享码
    pub share_code: String,
}

impl TransferState {
    pub fn new() -> Self {
        Self {
            local_transport: Arc::new(Mutex::new(None)),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            chunker: FileChunker::default_chunker(),
            checker: IntegrityChecker::new(),
            receiving_state: Arc::new(Mutex::new(ReceivingState::default())),
        }
    }
}

impl Default for TransferState {
    fn default() -> Self {
        Self::new()
    }
}

/// 初始化传输服务
#[tauri::command]
pub async fn init_transfer(state: State<'_, TransferState>) -> Result<(), String> {
    let transport = LocalTransport::new();
    transport.initialize().await.map_err(|e| e.to_string())?;

    let mut local_transport = state.local_transport.lock().await;
    *local_transport = Some(transport);

    Ok(())
}

/// 获取本机监听端口
#[tauri::command]
pub async fn get_transfer_port(state: State<'_, TransferState>) -> Result<u16, String> {
    let local_transport = state.local_transport.lock().await;
    if let Some(transport) = local_transport.as_ref() {
        transport.get_listen_port().await.map_err(|e| e.to_string())
    } else {
        Err("传输服务未初始化".to_string())
    }
}

/// 准备文件传输（计算元数据和哈希）
#[tauri::command]
pub async fn prepare_file_transfer(
    state: State<'_, TransferState>,
    file_path: String,
) -> Result<FileMetadata, String> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("文件不存在：{}", file_path));
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let mime_type = FileMetadata::infer_mime_type(&file_name);

    let file_metadata = FileMetadata::new(file_name, metadata.len(), mime_type);

    // 计算文件哈希和分块信息
    let file_metadata = state
        .chunker
        .compute_metadata_with_hashes(file_metadata, &path)
        .map_err(|e| e.to_string())?;

    Ok(file_metadata)
}

/// 发送文件（同步执行，阻塞直到完成或失败）
#[tauri::command]
pub async fn send_file(
    app: AppHandle,
    state: State<'_, TransferState>,
    file_metadata: FileMetadata,
    peer_id: String,
    peer_ip: String,
    peer_port: u16,
) -> Result<String, String> {
    // 创建传输任务
    let mut task = TransferTask::new(
        file_metadata.clone(),
        TransferMode::Local,
        TransferDirection::Send,
    );

    // 设置目标设备
    let peer = crate::models::PeerInfo::new(peer_id.clone(), peer_ip, peer_port);
    task = task.with_peer(peer);

    let task_id = task.id.clone();

    // 标记任务开始
    task.start();

    // 保存任务
    {
        let mut active_tasks = state.active_tasks.lock().await;
        active_tasks.insert(task_id.clone(), task.clone());
    }

    // 获取传输实例
    let transport_result = {
        let local_transport = state.local_transport.lock().await;
        if let Some(transport) = local_transport.as_ref() {
            // 执行传输
            transport.send(&task).await
        } else {
            Err(crate::error::TransferError::Internal(
                "传输服务未初始化".to_string(),
            ))
        }
    };

    // 更新任务状态并发送事件
    let mut active_tasks = state.active_tasks.lock().await;
    if let Some(t) = active_tasks.get_mut(&task_id) {
        match transport_result {
            Ok(progress) => {
                t.progress = progress.progress;
                t.transferred_bytes = progress.transferred_bytes;
                t.speed = progress.speed;
                t.status = progress.status;
                t.completed_at = progress.estimated_time_remaining.map(|_| {
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64
                });

                // 发送进度事件
                let _ = app.emit("transfer-progress", &progress);

                // 如果完成，发送完成事件
                if progress.status == crate::models::TaskStatus::Completed {
                    let _ = app.emit("transfer-complete", &progress);
                }
            }
            Err(e) => {
                t.fail(e.to_string());

                // 发送错误事件
                let error_progress = TransferProgress::from(&*t);
                let _ = app.emit("transfer-error", &error_progress);
            }
        }
    }

    Ok(task_id)
}

/// 发送文件（后台执行，立即返回任务 ID）
#[tauri::command]
pub async fn send_file_async(
    app: AppHandle,
    state: State<'_, TransferState>,
    file_metadata: FileMetadata,
    peer_id: String,
    peer_ip: String,
    peer_port: u16,
) -> Result<String, String> {
    // 创建传输任务
    let mut task = TransferTask::new(
        file_metadata.clone(),
        TransferMode::Local,
        TransferDirection::Send,
    );

    // 设置目标设备
    let peer = crate::models::PeerInfo::new(peer_id.clone(), peer_ip, peer_port);
    task = task.with_peer(peer);

    let task_id = task.id.clone();

    // 标记任务开始
    task.start();

    // 保存任务
    {
        let mut active_tasks = state.active_tasks.lock().await;
        active_tasks.insert(task_id.clone(), task.clone());
    }

    // 克隆需要的资源用于后台任务
    let local_transport = state.local_transport.clone();
    let active_tasks = state.active_tasks.clone();
    let task_id_clone = task_id.clone();
    let app_handle = app.clone();

    // 在后台执行传输
    tokio::spawn(async move {
        let transport_result = {
            let local_transport = local_transport.lock().await;
            if let Some(transport) = local_transport.as_ref() {
                // 使用内部方法获取任务并发送
                let tasks = active_tasks.lock().await;
                if let Some(task) = tasks.get(&task_id_clone) {
                    let task_clone = task.clone();
                    drop(tasks); // 释放锁
                    transport.send(&task_clone).await
                } else {
                    Err(crate::error::TransferError::Internal(
                        "任务不存在".to_string(),
                    ))
                }
            } else {
                Err(crate::error::TransferError::Internal(
                    "传输服务未初始化".to_string(),
                ))
            }
        };

        // 更新任务状态并发送事件
        let mut tasks = active_tasks.lock().await;
        if let Some(t) = tasks.get_mut(&task_id_clone) {
            match transport_result {
                Ok(progress) => {
                    t.progress = progress.progress;
                    t.transferred_bytes = progress.transferred_bytes;
                    t.speed = progress.speed;
                    t.status = progress.status;

                    // 发送进度事件
                    let _ = app_handle.emit("transfer-progress", &progress);
                }
                Err(e) => {
                    t.fail(e.to_string());

                    // 发送错误事件
                    let error_progress = TransferProgress::from(&*t);
                    let _ = app_handle.emit("transfer-error", &error_progress);
                }
            }
        }
    });

    Ok(task_id)
}

/// 取消传输
#[tauri::command]
pub async fn cancel_transfer(
    state: State<'_, TransferState>,
    task_id: String,
) -> Result<(), String> {
    // 取消本地传输
    {
        let local_transport = state.local_transport.lock().await;
        if let Some(transport) = local_transport.as_ref() {
            transport
                .cancel(&task_id)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // 更新任务状态
    let mut active_tasks = state.active_tasks.lock().await;
    if let Some(task) = active_tasks.get_mut(&task_id) {
        task.cancel();
    }

    Ok(())
}

/// 获取传输进度
#[tauri::command]
pub async fn get_transfer_progress(
    state: State<'_, TransferState>,
    task_id: String,
) -> Result<TransferProgress, String> {
    let active_tasks = state.active_tasks.lock().await;
    active_tasks
        .get(&task_id)
        .map(|t| TransferProgress::from(t))
        .ok_or_else(|| format!("任务不存在：{}", task_id))
}

/// 获取所有活跃任务
#[tauri::command]
pub async fn get_active_tasks(
    state: State<'_, TransferState>,
) -> Result<Vec<TransferTask>, String> {
    let active_tasks = state.active_tasks.lock().await;
    Ok(active_tasks.values().cloned().collect())
}

/// 验证文件完整性
#[tauri::command]
pub async fn verify_file_integrity(
    state: State<'_, TransferState>,
    file_path: String,
    expected_hash: String,
) -> Result<bool, String> {
    let path = PathBuf::from(&file_path);
    state
        .checker
        .verify_file(&path, &expected_hash)
        .map_err(|e| e.to_string())
}

/// 清理已完成的任务
#[tauri::command]
pub async fn cleanup_completed_tasks(state: State<'_, TransferState>) -> Result<usize, String> {
    let mut active_tasks = state.active_tasks.lock().await;
    let before_count = active_tasks.len();

    active_tasks.retain(|_, task| {
        task.status != crate::models::TaskStatus::Completed
            && task.status != crate::models::TaskStatus::Cancelled
    });

    Ok(before_count - active_tasks.len())
}

/// 启动接收监听服务器
#[tauri::command]
pub async fn start_receiving(
    state: State<'_, TransferState>,
    port: Option<u16>,
) -> Result<ReceivingState, String> {
    use std::net::IpAddr;
    use std::str::FromStr;

    // 检查是否已经在接收
    {
        let receiving_state = state.receiving_state.lock().await;
        if receiving_state.is_receiving {
            return Ok(ReceivingState {
                is_receiving: true,
                port: receiving_state.port,
                network_address: receiving_state.network_address.clone(),
                share_code: receiving_state.share_code.clone(),
            });
        }
    }

    // 创建新的 LocalTransport 用于接收
    let transport = if let Some(p) = port {
        LocalTransport::with_port(p)
    } else {
        LocalTransport::new()
    };

    // 初始化传输服务
    transport.initialize().await.map_err(|e| e.to_string())?;

    // 获取监听端口
    let listen_port = transport
        .get_listen_port()
        .await
        .map_err(|e| e.to_string())?;

    // 获取本地 IP 地址
    let network_address = get_local_ip()
        .unwrap_or_else(|| IpAddr::from_str("127.0.0.1").unwrap());
    let network_address_str = network_address.to_string();

    // 生成分享码（6 位数字，基于端口和时间戳）
    let share_code = format!("{:06}", (listen_port as u32 + std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32) % 1000000);

    // 保存传输实例
    {
        let mut local_transport = state.local_transport.lock().await;
        *local_transport = Some(transport);
    }

    // 更新接收状态
    let result = {
        let mut receiving_state = state.receiving_state.lock().await;
        receiving_state.is_receiving = true;
        receiving_state.port = listen_port;
        receiving_state.network_address = network_address_str.clone();
        receiving_state.share_code = share_code.clone();

        ReceivingState {
            is_receiving: true,
            port: listen_port,
            network_address: network_address_str,
            share_code,
        }
    };

    Ok(result)
}

/// 停止接收监听服务器
#[tauri::command]
pub async fn stop_receiving(state: State<'_, TransferState>) -> Result<(), String> {
    // 检查是否有活跃任务
    {
        let active_tasks = state.active_tasks.lock().await;
        let has_active_tasks = active_tasks.values().any(|t| {
            t.status == crate::models::TaskStatus::Pending
                || t.status == crate::models::TaskStatus::Transferring
        });

        if has_active_tasks {
            return Err("有活跃的传输任务，无法停止接收服务".to_string());
        }
    }

    // 关闭传输服务
    {
        let mut local_transport = state.local_transport.lock().await;
        if let Some(transport) = local_transport.take() {
            transport.shutdown().await.map_err(|e| e.to_string())?;
        }
    }

    // 重置接收状态
    {
        let mut receiving_state = state.receiving_state.lock().await;
        receiving_state.is_receiving = false;
        receiving_state.port = 0;
        receiving_state.network_address.clear();
        receiving_state.share_code.clear();
    }

    Ok(())
}

/// 获取网络信息（只返回真实服务器状态）
#[tauri::command]
pub async fn get_network_info(state: State<'_, TransferState>) -> Result<ReceivingState, String> {
    let receiving_state = state.receiving_state.lock().await;

    // 只返回真实的接收状态，不生成临时信息
    Ok(ReceivingState {
        is_receiving: receiving_state.is_receiving,
        port: receiving_state.port,
        network_address: receiving_state.network_address.clone(),
        share_code: receiving_state.share_code.clone(),
    })
}

/// 获取文件元数据（不计算哈希，仅获取基本信息）
#[tauri::command]
pub async fn get_file_metadata(file_path: String) -> Result<FileMetadata, String> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("文件不存在：{}", file_path));
    }

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let mime_type = FileMetadata::infer_mime_type(&file_name);

    let file_metadata = FileMetadata::new(file_name, metadata.len(), mime_type);

    Ok(file_metadata)
}

/// 文件信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileInfo {
    /// 文件路径
    pub path: String,
    /// 文件名
    pub name: String,
    /// 文件大小
    pub size: u64,
    /// 相对路径
    pub relative_path: String,
}

/// 递归获取文件夹下的所有文件
#[tauri::command]
pub async fn get_files_in_folder(folder_path: String) -> Result<Vec<FileInfo>, String> {
    let folder = PathBuf::from(&folder_path);

    if !folder.exists() {
        return Err(format!("文件夹不存在：{}", folder_path));
    }

    if !folder.is_dir() {
        return Err(format!("路径不是文件夹：{}", folder_path));
    }

    // 验证路径合法性（防止路径遍历攻击）
    let canonical_folder = folder
        .canonicalize()
        .map_err(|e| format!("路径验证失败：{}", e))?;

    let mut files = Vec::new();
    collect_files_recursive(&canonical_folder, &canonical_folder, &mut files)
        .map_err(|e| e.to_string())?;

    Ok(files)
}

/// 递归收集文件
fn collect_files_recursive(
    current_dir: &PathBuf,
    base_dir: &PathBuf,
    files: &mut Vec<FileInfo>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();

        // 计算相对路径
        let relative_path = path
            .strip_prefix(base_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            // 递归处理子目录
            collect_files_recursive(&path, base_dir, files)?;
        } else if path.is_file() {
            // 添加文件信息
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let metadata = std::fs::metadata(&path)?;
            let size = metadata.len();

            files.push(FileInfo {
                path: path.to_string_lossy().to_string(),
                name,
                size,
                relative_path,
            });
        }
    }

    Ok(())
}

/// 获取本机 IP 地址
fn get_local_ip() -> Option<std::net::IpAddr> {
    // 尝试获取本机 IP 地址
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return Some(addr.ip());
            }
        }
    }
    None
}

/// 默认接收目录
fn get_default_receive_directory() -> String {
    // 尝试获取用户下载目录
    if let Some(home) = std::env::var("HOME").ok() {
        let download_dir = PathBuf::from(home).join("Downloads").join("PureSend");
        return download_dir.to_string_lossy().to_string();
    }
    // Windows 系统
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let download_dir = PathBuf::from(userprofile)
            .join("Downloads")
            .join("PureSend");
        return download_dir.to_string_lossy().to_string();
    }
    // 降级到当前目录
    "./downloads".to_string()
}

/// 获取接收目录
#[tauri::command]
pub async fn get_receive_directory() -> Result<String, String> {
    // 返回默认接收目录
    Ok(get_default_receive_directory())
}

/// 设置接收目录
#[tauri::command]
pub async fn set_receive_directory(directory: String) -> Result<(), String> {
    // 验证目录是否存在，不存在则创建
    let path = PathBuf::from(&directory);
    if !path.exists() {
        std::fs::create_dir_all(&path).map_err(|e| {
            format!(
                "无法创建接收目录 '{}': {}",
                directory,
                e
            )
        })?;
    }

    // 验证目录是否可写
    let test_file = path.join(".write_test");
    if std::fs::File::create(&test_file).is_err() {
        return Err(format!(
            "接收目录 '{}' 不可写",
            directory
        ));
    }
    // 删除测试文件
    let _ = std::fs::remove_file(&test_file);

    Ok(())
}
