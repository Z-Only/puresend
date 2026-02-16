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
}

impl TransferState {
    pub fn new() -> Self {
        Self {
            local_transport: Arc::new(Mutex::new(None)),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            chunker: FileChunker::default_chunker(),
            checker: IntegrityChecker::new(),
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
    
    if !path.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }
    
    state
        .checker
        .verify_file(&path, &expected_hash)
        .map_err(|e| e.to_string())
}

/// 清理已完成任务
#[tauri::command]
pub async fn cleanup_completed_tasks(
    state: State<'_, TransferState>,
) -> Result<usize, String> {
    let mut active_tasks = state.active_tasks.lock().await;
    let initial_count = active_tasks.len();
    
    active_tasks.retain(|_, task| {
        !matches!(
            task.status,
            crate::models::TaskStatus::Completed
                | crate::models::TaskStatus::Failed
                | crate::models::TaskStatus::Cancelled
        )
    });
    
    Ok(initial_count - active_tasks.len())
}