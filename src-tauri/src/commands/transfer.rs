//! 传输相关 Tauri 命令

use crate::models::{FileMetadata, TransferDirection, TransferMode, TransferProgress, TransferTask};
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
        return Err(format!("文件不存在: {}", file_path));
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
            Err(crate::error::TransferError::Internal("传输服务未初始化".to_string()))
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

/// 发送文件（后台执行，立即返回任务ID）
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
                    Err(crate::error::TransferError::Internal("任务不存在".to_string()))
                }
            } else {
                Err(crate::error::TransferError::Internal("传输服务未初始化".to_string()))
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
            transport.cancel(&task_id).await.map_err(|e| e.to_string())?;
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
        .ok_or_else(|| format!("任务不存在: {}", task_id))
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
    state.checker.verify_file(&path, &expected_hash).map_err(|e| e.to_string())
}

/// 清理已完成的任务
#[tauri::command]
pub async fn cleanup_completed_tasks(
    state: State<'_, TransferState>,
) -> Result<usize, String> {
    let mut active_tasks = state.active_tasks.lock().await;
    let before_count = active_tasks.len();
    
    active_tasks.retain(|_, task| {
        task.status != crate::models::TaskStatus::Completed 
            && task.status != crate::models::TaskStatus::Cancelled
    });
    
    Ok(before_count - active_tasks.len())
}

/// 启动接收监听服务器（内部实现）
async fn start_receiving_impl(
    state: &State<'_, TransferState>,
) -> Result<ReceivingState, String> {
    use std::net::IpAddr;
    use std::str::FromStr;
    
    let mut receiving_state = state.receiving_state.lock().await;
    
    if receiving_state.is_receiving {
        return Err("接收服务已在运行".to_string());
    }
    
    // 获取本机 IP 地址
    let network_address = get_local_ip().unwrap_or_else(|| IpAddr::from_str("127.0.0.1").unwrap());
    let network_address_str = network_address.to_string();
    
    // 生成随机端口（10000-20000 范围）
    let port = 10000 + (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() % 10000) as u16;
    
    // 生成分享码（6 位随机数字）
    let share_code = format!("{:06}", port % 10000);
    
    receiving_state.is_receiving = true;
    receiving_state.port = port;
    receiving_state.network_address = network_address_str.clone();
    receiving_state.share_code = share_code.clone();
    
    Ok(ReceivingState {
        is_receiving: true,
        port,
        network_address: network_address_str,
        share_code,
    })
}

/// 启动接收监听服务器
#[tauri::command]
pub async fn start_receiving(
    app: AppHandle,
    state: State<'_, TransferState>,
) -> Result<ReceivingState, String> {
    let receiving_state = start_receiving_impl(&state).await?;
    
    // 启动接收监听（后台任务）
    let _receiving_state_clone = state.receiving_state.clone();
    let app_handle = app.clone();
    
    tokio::spawn(async move {
        // TODO: 实现接收监听逻辑
        // 这里需要启动 TCP 监听器，等待发送方连接
        let _ = app_handle.emit("receiving-started", &*_receiving_state_clone.lock().await);
    });
    
    Ok(receiving_state)
}

/// 停止接收监听服务器
#[tauri::command]
pub async fn stop_receiving(
    state: State<'_, TransferState>,
) -> Result<(), String> {
    let mut receiving_state = state.receiving_state.lock().await;
    
    if !receiving_state.is_receiving {
        return Err("接收服务未运行".to_string());
    }
    
    receiving_state.is_receiving = false;
    receiving_state.port = 0;
    receiving_state.network_address = String::new();
    receiving_state.share_code = String::new();
    
    Ok(())
}

/// 获取接收目录
#[tauri::command]
pub async fn get_receive_directory() -> Result<String, String> {
    let home_dir = std::env::var("HOME")
        .map_err(|_| "无法获取主目录".to_string())?;
    let receive_dir = std::path::PathBuf::from(home_dir)
        .join("Downloads")
        .join("PureSend");
    
    // 确保目录存在
    std::fs::create_dir_all(&receive_dir)
        .map_err(|e| format!("创建目录失败：{}", e))?;
    
    Ok(receive_dir.to_string_lossy().to_string())
}

/// 设置接收目录
#[tauri::command]
pub async fn set_receive_directory(directory: String) -> Result<(), String> {
    let path = PathBuf::from(&directory);
    
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("创建目录失败：{}", e))?;
    }
    
    // TODO: 保存到配置文件
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

/// 获取网络信息（不启动接收服务）
#[tauri::command]
pub async fn get_network_info(
    state: State<'_, TransferState>,
) -> Result<ReceivingState, String> {
    use std::net::IpAddr;
    use std::str::FromStr;
    
    let receiving_state = state.receiving_state.lock().await;
    
    // 如果已经在接收，返回当前的接收状态
    if receiving_state.is_receiving {
        return Ok(ReceivingState {
            is_receiving: receiving_state.is_receiving,
            port: receiving_state.port,
            network_address: receiving_state.network_address.clone(),
            share_code: receiving_state.share_code.clone(),
        });
    }
    
    // 否则，生成临时的网络信息（不启动接收服务）
    let network_address = get_local_ip().unwrap_or_else(|| IpAddr::from_str("127.0.0.1").unwrap());
    let network_address_str = network_address.to_string();
    
    // 生成随机端口（10000-20000 范围）
    let port = 10000 + (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() % 10000) as u16;
    
    // 生成分享码（6 位随机数字）
    let share_code = format!("{:06}", port % 10000);
    
    drop(receiving_state);
    
    Ok(ReceivingState {
        is_receiving: false,
        port,
        network_address: network_address_str,
        share_code,
    })
}