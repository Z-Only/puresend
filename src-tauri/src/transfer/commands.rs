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
    /// 网络地址列表
    pub network_addresses: Vec<String>,
    /// 分享码
    pub share_code: String,
    /// 是否自动接收
    pub auto_receive: bool,
    /// 是否覆盖同名文件
    pub file_overwrite: bool,
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

    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(format!("文件不存在：{}", file_path));
    }

    // 路径规范化验证，防止路径遍历攻击
    let path = tokio::fs::canonicalize(&path)
        .await
        .map_err(|e| format!("无法解析文件路径：{}", e))?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let metadata = tokio::fs::metadata(&path).await.map_err(|e| e.to_string())?;
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
                        .unwrap_or_default()
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
    // 读取当前接收设置
    let current_settings = {
        let settings = get_receive_settings_lock()
            .read()
            .map_err(|e| e.to_string())?;
        settings.clone()
    };

    // 检查是否已经在接收
    {
        let receiving_state = state.receiving_state.lock().await;
        if receiving_state.is_receiving {
            return Ok(ReceivingState {
                is_receiving: true,
                port: receiving_state.port,
                network_addresses: receiving_state.network_addresses.clone(),
                share_code: receiving_state.share_code.clone(),
                auto_receive: current_settings.auto_receive,
                file_overwrite: current_settings.file_overwrite,
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

    // 设置接收配置
    use crate::transfer::local::ReceiveConfig;
    let receive_config = ReceiveConfig {
        auto_receive: current_settings.auto_receive,
        file_overwrite: current_settings.file_overwrite,
        receive_directory: std::env::temp_dir(), // 使用临时目录作为默认接收目录
    };
    transport.set_receive_config(receive_config).await;

    // 获取监听端口
    let listen_port = transport
        .get_listen_port()
        .await
        .map_err(|e| e.to_string())?;

    // 获取本地所有 IP 地址
    let network_addresses = crate::network::get_local_ips();

    // 生成分享码（6 位数字，基于端口和时间戳）
    let share_code = format!(
        "{:06}",
        (listen_port as u32
            + std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u32)
            % 1000000
    );

    // 保存传输实例
    {
        let mut local_transport = state.local_transport.lock().await;
        *local_transport = Some(transport);
    }

    // 更新接收状态并返回结果
    let result = {
        let mut receiving_state = state.receiving_state.lock().await;
        receiving_state.is_receiving = true;
        receiving_state.port = listen_port;
        receiving_state.network_addresses = network_addresses.clone();
        receiving_state.share_code = share_code.clone();

        ReceivingState {
            is_receiving: true,
            port: listen_port,
            network_addresses,
            share_code,
            auto_receive: current_settings.auto_receive,
            file_overwrite: current_settings.file_overwrite,
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
        receiving_state.network_addresses.clear();
        receiving_state.share_code.clear();
    }

    Ok(())
}

/// 获取网络信息（只返回真实服务器状态）
#[tauri::command]
pub async fn get_network_info(state: State<'_, TransferState>) -> Result<ReceivingState, String> {
    let receiving_state = state.receiving_state.lock().await;

    // 读取当前设置
    let settings = get_receive_settings_lock()
        .read()
        .map_err(|e| e.to_string())?;

    // 只返回真实的接收状态，不生成临时信息
    Ok(ReceivingState {
        is_receiving: receiving_state.is_receiving,
        port: receiving_state.port,
        network_addresses: receiving_state.network_addresses.clone(),
        share_code: receiving_state.share_code.clone(),
        auto_receive: settings.auto_receive,
        file_overwrite: settings.file_overwrite,
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

// ============ 接收设置相关命令 ============

/// 接收设置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveSettings {
    /// 是否自动接收
    pub auto_receive: bool,
    /// 是否覆盖同名文件
    pub file_overwrite: bool,
}

impl Default for ReceiveSettings {
    fn default() -> Self {
        Self {
            auto_receive: false,
            file_overwrite: false,
        }
    }
}

/// 接收设置状态（由前端 Tauri Store 管理，后端仅读取）
static RECEIVE_SETTINGS: std::sync::OnceLock<std::sync::RwLock<ReceiveSettings>> =
    std::sync::OnceLock::new();

fn get_receive_settings_lock() -> &'static std::sync::RwLock<ReceiveSettings> {
    RECEIVE_SETTINGS.get_or_init(|| std::sync::RwLock::new(ReceiveSettings::default()))
}

/// 获取接收设置
#[tauri::command]
pub async fn get_receive_settings() -> Result<ReceiveSettings, String> {
    let settings = get_receive_settings_lock()
        .read()
        .map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

/// 设置自动接收
#[tauri::command]
pub async fn set_auto_receive(enabled: bool) -> Result<(), String> {
    let mut settings = get_receive_settings_lock()
        .write()
        .map_err(|e| e.to_string())?;
    settings.auto_receive = enabled;
    Ok(())
}

/// 设置文件覆盖
#[tauri::command]
pub async fn set_file_overwrite(enabled: bool) -> Result<(), String> {
    let mut settings = get_receive_settings_lock()
        .write()
        .map_err(|e| e.to_string())?;
    settings.file_overwrite = enabled;
    Ok(())
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
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("无法创建接收目录 '{}': {}", directory, e))?;
    }

    // 验证目录是否可写
    let test_file = path.join(".write_test");
    if std::fs::File::create(&test_file).is_err() {
        return Err(format!("接收目录 '{}' 不可写", directory));
    }
    // 删除测试文件
    let _ = std::fs::remove_file(&test_file);

    Ok(())
}

// ============ 文件重命名工具函数 ============

/// 生成不冲突的文件名
///
/// 如果目标文件已存在，自动生成新文件名（如 `file.txt` -> `file (1).txt`）
/// 支持边界情况：
/// - 多次重命名（file (1).txt -> file (2).txt）
/// - 无扩展名文件
/// - 多个扩展名（如 .tar.gz）
/// - 隐藏文件（如 .gitignore）
pub fn generate_unique_filename(
    directory: &PathBuf,
    original_name: &str,
) -> Result<String, String> {
    let path = directory.join(original_name);

    // 如果文件不存在，直接使用原文件名
    if !path.exists() {
        return Ok(original_name.to_string());
    }

    // 解析文件名和扩展名
    let (stem, extension) = parse_filename(original_name);

    // 尝试找到可用的文件名
    let mut counter = 1u32;
    loop {
        let new_name = if extension.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, extension)
        };

        let new_path = directory.join(&new_name);
        if !new_path.exists() {
            return Ok(new_name);
        }

        counter += 1;

        // 防止无限循环（最多尝试 10000 次）
        if counter > 10000 {
            return Err(format!("无法生成唯一文件名：{}", original_name));
        }
    }
}

/// 解析文件名为（主文件名，扩展名）
///
/// 特殊处理：
/// - `.gitignore` -> (".gitignore", "")
/// - `file.tar.gz` -> ("file", "tar.gz")
/// - `file` -> ("file", "")
fn parse_filename(filename: &str) -> (String, String) {
    // 特殊情况：以点开头的隐藏文件
    if filename.starts_with('.') && filename.matches('.').count() == 1 {
        return (filename.to_string(), String::new());
    }

    // 查找最后一个点
    if let Some(dot_pos) = filename.rfind('.') {
        let stem = &filename[..dot_pos];
        let ext = &filename[dot_pos + 1..];

        // 检查是否为复合扩展名（如 .tar.gz）
        if let Some(inner_dot) = stem.rfind('.') {
            let inner_ext = &stem[inner_dot + 1..];
            // 常见复合扩展名
            const COMPOUND_EXTENSIONS: &[&str] = &["tar", "zip"];
            if COMPOUND_EXTENSIONS.contains(&inner_ext) {
                return (
                    stem[..inner_dot].to_string(),
                    format!("{}.{}", inner_ext, ext),
                );
            }
        }

        (stem.to_string(), ext.to_string())
    } else {
        (filename.to_string(), String::new())
    }
}

/// 获取接收文件的实际保存路径
///
/// 根据 file_overwrite 设置决定：
/// - true: 直接覆盖同名文件
/// - false: 自动重命名避免冲突
pub fn get_receive_file_path(
    directory: &PathBuf,
    filename: &str,
    file_overwrite: bool,
) -> Result<PathBuf, String> {
    // 如果允许覆盖，直接返回原路径
    if file_overwrite {
        return Ok(directory.join(filename));
    }

    // 不允许覆盖，生成唯一文件名
    let unique_name = generate_unique_filename(directory, filename)?;
    Ok(directory.join(unique_name))
}

/// 获取接收文件的实际保存路径（Tauri 命令版本）
#[tauri::command]
pub async fn get_unique_file_path(
    directory: String,
    filename: String,
    file_overwrite: bool,
) -> Result<String, String> {
    let dir_path = PathBuf::from(&directory);

    // 确保目录存在
    if !dir_path.exists() {
        std::fs::create_dir_all(&dir_path)
            .map_err(|e| format!("无法创建目录 '{}': {}", directory, e))?;
    }

    let result_path = get_receive_file_path(&dir_path, &filename, file_overwrite)?;
    Ok(result_path.to_string_lossy().to_string())
}

// ============ 加密设置相关命令 ============

/// 获取加密是否启用
#[tauri::command]
pub async fn get_encryption_enabled() -> Result<bool, String> {
    Ok(crate::transfer::crypto::is_encryption_enabled())
}

/// 设置加密启用状态
#[tauri::command]
pub async fn set_encryption_enabled(enabled: bool) -> Result<(), String> {
    crate::transfer::crypto::set_encryption_enabled_internal(enabled);
    Ok(())
}

// ============ 压缩设置相关命令 ============

/// 获取压缩是否启用
#[tauri::command]
pub async fn get_compression_enabled() -> Result<bool, String> {
    let config = crate::transfer::compression::get_compression_config();
    Ok(config.enabled)
}

/// 设置压缩启用状态
#[tauri::command]
pub async fn set_compression_enabled(enabled: bool) -> Result<(), String> {
    crate::transfer::compression::set_compression_enabled_internal(enabled);
    Ok(())
}

/// 设置压缩模式
#[tauri::command]
pub async fn set_compression_mode(mode: String) -> Result<(), String> {
    if mode != "smart" && mode != "manual" {
        return Err(format!("无效的压缩模式: {}，支持 smart 或 manual", mode));
    }
    crate::transfer::compression::set_compression_mode_internal(mode);
    Ok(())
}

/// 设置压缩级别
#[tauri::command]
pub async fn set_compression_level(level: i32) -> Result<(), String> {
    if !(1..=19).contains(&level) {
        return Err(format!("无效的压缩级别: {}，范围为 1-19", level));
    }
    crate::transfer::compression::set_compression_level_internal(level);
    Ok(())
}

// ============ 断点续传相关命令 ============

/// 获取可恢复的任务列表
#[tauri::command]
pub async fn get_resumable_tasks() -> Result<Vec<crate::transfer::resume::ResumableTaskInfo>, String> {
    let storage_dir = crate::transfer::resume::default_resume_storage_dir();
    let manager = crate::transfer::resume::ResumeManager::new(storage_dir);
    manager.load().await.map_err(|e| e.to_string())?;
    Ok(manager.get_resumable_tasks().await)
}

/// 恢复传输（当前仅清除断点信息，实际续传逻辑在传输管道中处理）
#[tauri::command]
pub async fn resume_transfer(task_id: String) -> Result<(), String> {
    let storage_dir = crate::transfer::resume::default_resume_storage_dir();
    let manager = crate::transfer::resume::ResumeManager::new(storage_dir);
    manager.load().await.map_err(|e| e.to_string())?;

    let resume_info = manager.get_resume_info(&task_id).await;
    if resume_info.is_none() {
        return Err(format!("未找到任务 {} 的断点信息，可能已过期", task_id));
    }

    Ok(())
}

/// 清理断点信息
#[tauri::command]
pub async fn cleanup_resume_info(task_id: Option<String>) -> Result<(), String> {
    let storage_dir = crate::transfer::resume::default_resume_storage_dir();
    let manager = crate::transfer::resume::ResumeManager::new(storage_dir);
    manager.load().await.map_err(|e| e.to_string())?;

    if let Some(id) = task_id {
        manager.remove_resume_info(&id).await.map_err(|e| e.to_string())?;
    } else {
        manager.cleanup_all().await.map_err(|e| e.to_string())?;
    }

    Ok(())
}
