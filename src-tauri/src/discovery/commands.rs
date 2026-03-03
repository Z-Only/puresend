//! 设备发现相关 Tauri 命令

use crate::discovery::DiscoveryManager;
use crate::models::PeerInfo;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

/// 设备发现状态（用于 Tauri 状态管理）
///
/// 包装 DiscoveryManager，支持延迟初始化（init_discovery 时创建）。
pub struct DiscoveryState {
    pub manager: Arc<Mutex<Option<Arc<DiscoveryManager>>>>,
}

impl Default for DiscoveryState {
    fn default() -> Self {
        Self {
            manager: Arc::new(Mutex::new(None)),
        }
    }
}

/// 获取本机设备名称
#[tauri::command]
pub async fn get_device_name() -> Result<String, String> {
    Ok(hostname::get()
        .map(|h| h.into_string().unwrap_or_else(|_| "Unknown Device".to_string()))
        .unwrap_or_else(|_| "Unknown Device".to_string()))
}

/// 初始化设备发现服务
///
/// 创建 DiscoveryManager 并启动 mDNS 发现，订阅设备发现事件发送到前端。
#[tauri::command]
pub async fn init_discovery(
    state: tauri::State<'_, DiscoveryState>,
    app: AppHandle,
    device_name: Option<String>,
    listen_port: Option<u16>,
) -> Result<(), String> {
    let manager = match (device_name, listen_port) {
        (Some(name), Some(port)) => Arc::new(DiscoveryManager::new(name, port)),
        _ => Arc::new(DiscoveryManager::default()),
    };

    manager.start().await.map_err(|e| e.to_string())?;

    // 订阅设备发现事件并发送到前端
    let mut receiver = manager.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            let _ = app.emit("peer-discovery", event);
        }
    });

    *state.manager.lock().await = Some(manager);
    Ok(())
}

/// 停止设备发现服务
#[tauri::command]
pub async fn stop_discovery(
    state: tauri::State<'_, DiscoveryState>,
) -> Result<(), String> {
    let manager_guard = state.manager.lock().await;
    if let Some(manager) = manager_guard.as_ref() {
        manager.stop().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 获取所有已发现的设备
#[tauri::command]
pub async fn get_peers(state: tauri::State<'_, DiscoveryState>) -> Result<Vec<PeerInfo>, String> {
    let manager_guard = state.manager.lock().await;
    match manager_guard.as_ref() {
        Some(manager) => Ok(manager.get_peers().await),
        None => Ok(Vec::new()),
    }
}

/// 获取指定设备信息
#[tauri::command]
pub async fn get_peer(
    state: tauri::State<'_, DiscoveryState>,
    peer_id: String,
) -> Result<Option<PeerInfo>, String> {
    let manager_guard = state.manager.lock().await;
    match manager_guard.as_ref() {
        Some(manager) => Ok(manager.get_peer(&peer_id).await),
        None => Ok(None),
    }
}

/// 手动添加设备
#[tauri::command]
pub async fn add_peer_manual(
    state: tauri::State<'_, DiscoveryState>,
    ip: String,
    port: u16,
) -> Result<PeerInfo, String> {
    let manager_guard = state.manager.lock().await;
    match manager_guard.as_ref() {
        Some(manager) => Ok(manager.add_peer_manual(ip, port).await),
        None => Err("Discovery service not initialized".to_string()),
    }
}

/// 检查设备是否在线
#[tauri::command]
pub async fn is_peer_online(
    state: tauri::State<'_, DiscoveryState>,
    id: String,
) -> Result<bool, String> {
    let manager_guard = state.manager.lock().await;
    match manager_guard.as_ref() {
        Some(manager) => Ok(manager.is_peer_online(&id).await),
        None => Ok(false),
    }
}

/// 获取在线设备数量
#[tauri::command]
pub async fn get_online_count(
    state: tauri::State<'_, DiscoveryState>,
) -> Result<usize, String> {
    let manager_guard = state.manager.lock().await;
    match manager_guard.as_ref() {
        Some(manager) => Ok(manager.online_count().await),
        None => Ok(0),
    }
}

/// 重启设备发现服务
#[tauri::command]
pub async fn restart_discovery(
    state: tauri::State<'_, DiscoveryState>,
) -> Result<(), String> {
    let manager_guard = state.manager.lock().await;
    if let Some(manager) = manager_guard.as_ref() {
        manager.restart().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}