//! 设备发现相关 Tauri 命令

use crate::discovery::DiscoveryManager;
use crate::models::PeerInfo;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

/// 设备发现状态
pub struct DiscoveryState {
    /// 发现管理器
    manager: Arc<Mutex<Option<DiscoveryManager>>>,
}

impl DiscoveryState {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for DiscoveryState {
    fn default() -> Self {
        Self::new()
    }
}

/// 初始化设备发现服务
#[tauri::command]
pub async fn init_discovery(
    app: AppHandle,
    state: State<'_, DiscoveryState>,
    device_name: Option<String>,
    listen_port: Option<u16>,
) -> Result<(), String> {
    let manager = if let (Some(name), Some(port)) = (device_name, listen_port) {
        DiscoveryManager::new(name, port)
    } else {
        DiscoveryManager::default()
    };

    // 启动发现服务
    manager.start().await.map_err(|e| e.to_string())?;

    // 订阅发现事件并转发到前端
    let mut event_receiver = manager.subscribe();
    let app_handle = app.clone();
    
    tokio::spawn(async move {
        while let Ok(event) = event_receiver.recv().await {
            let _ = app_handle.emit("peer-discovery", &event);
        }
    });

    // 保存管理器
    let mut manager_guard = state.manager.lock().await;
    *manager_guard = Some(manager);

    Ok(())
}

/// 停止设备发现服务
#[tauri::command]
pub async fn stop_discovery(state: State<'_, DiscoveryState>) -> Result<(), String> {
    let mut manager_guard = state.manager.lock().await;
    
    if let Some(manager) = manager_guard.take() {
        manager.stop().await.map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 获取已发现的设备列表
#[tauri::command]
pub async fn get_peers(state: State<'_, DiscoveryState>) -> Result<Vec<PeerInfo>, String> {
    let manager_guard = state.manager.lock().await;
    
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.get_peers().await)
    } else {
        Err("设备发现服务未初始化".to_string())
    }
}

/// 获取指定设备信息
#[tauri::command]
pub async fn get_peer(state: State<'_, DiscoveryState>, peer_id: String) -> Result<Option<PeerInfo>, String> {
    let manager_guard = state.manager.lock().await;
    
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.get_peer(&peer_id).await)
    } else {
        Err("设备发现服务未初始化".to_string())
    }
}

/// 手动添加设备
#[tauri::command]
pub async fn add_peer_manual(
    state: State<'_, DiscoveryState>,
    ip: String,
    port: u16,
) -> Result<PeerInfo, String> {
    let manager_guard = state.manager.lock().await;
    
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.add_peer_manual(ip, port).await)
    } else {
        Err("设备发现服务未初始化".to_string())
    }
}

/// 检查设备是否在线
#[tauri::command]
pub async fn is_peer_online(
    state: State<'_, DiscoveryState>,
    peer_id: String,
) -> Result<bool, String> {
    let manager_guard = state.manager.lock().await;
    
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.is_peer_online(&peer_id).await)
    } else {
        Err("设备发现服务未初始化".to_string())
    }
}

/// 获取在线设备数量
#[tauri::command]
pub async fn get_online_count(state: State<'_, DiscoveryState>) -> Result<usize, String> {
    let manager_guard = state.manager.lock().await;
    
    if let Some(manager) = manager_guard.as_ref() {
        Ok(manager.online_count().await)
    } else {
        Err("设备发现服务未初始化".to_string())
    }
}