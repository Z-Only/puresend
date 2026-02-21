//! 设备发现相关 Tauri 命令

use crate::discovery::DiscoveryManager;
use crate::models::PeerInfo;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

/// 获取本机设备名称
#[tauri::command]
pub async fn get_device_name() -> String {
    // 在 macOS 上尝试获取设备型号名称
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("system_profiler")
            .args(["SPHardwareDataType", "-detailLevel", "mini"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 解析输出，查找 "Model Name:" 行
            for line in stdout.lines() {
                let line = line.trim();
                if line.starts_with("Model Name:") {
                    if let Some(model_name) = line.strip_prefix("Model Name:") {
                        let name = model_name.trim().to_string();
                        if !name.is_empty() {
                            return name;
                        }
                    }
                }
            }
        }
    }

    // 在 Linux 上尝试获取设备型号
    #[cfg(target_os = "linux")]
    {
        // 方法1: 读取 DMI 信息（适用于大多数 Linux 发行版）
        if let Ok(model) = std::fs::read_to_string("/sys/class/dmi/id/product_name") {
            let model = model.trim().to_string();
            if !model.is_empty() && model != "To Be Filled By O.E.M." {
                return model;
            }
        }

        // 方法2: 使用 hostnamectl 命令（systemd 系统）
        if let Ok(output) = std::process::Command::new("hostnamectl")
            .arg("--static")
            .output()
        {
            let hostname = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !hostname.is_empty() && hostname != "localhost" {
                // 尝试获取硬件型号
                if let Ok(hw_output) = std::process::Command::new("hostnamectl")
                    .output()
                {
                    let hw_stdout = String::from_utf8_lossy(&hw_output.stdout);
                    for line in hw_stdout.lines() {
                        if line.contains("Hardware Model:") || line.contains("Machine:") {
                            if let Some(pos) = line.find(':') {
                                let model = line[pos + 1..].trim().to_string();
                                if !model.is_empty() {
                                    return model;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 方法3: 读取设备树信息（嵌入式 Linux）
        if let Ok(model) = std::fs::read_to_string("/sys/firmware/devicetree/base/model") {
            let model = model.trim().to_string();
            if !model.is_empty() {
                return model;
            }
        }
    }

    // 在 Android 上尝试获取设备型号
    #[cfg(target_os = "android")]
    {
        // 通过 getprop 获取设备型号信息
        if let Ok(output) = std::process::Command::new("getprop")
            .arg("ro.product.model")
            .output()
        {
            let model = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !model.is_empty() {
                // 获取制造商信息，组合成更友好的名称
                if let Ok(manufacturer_output) = std::process::Command::new("getprop")
                    .arg("ro.product.manufacturer")
                    .output()
                {
                    let manufacturer = String::from_utf8_lossy(&manufacturer_output)
                        .trim()
                        .to_string();
                    if !manufacturer.is_empty() {
                        return format!("{} {}", manufacturer, model);
                    }
                }
                return model;
            }
        }

        // 备选方案：尝试获取品牌和设备名称
        if let Ok(output) = std::process::Command::new("getprop")
            .arg("ro.product.device")
            .output()
        {
            let device = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !device.is_empty() {
                return device;
            }
        }
    }

    // 在 Windows 上尝试获取设备型号
    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(["csproduct", "get", "name"])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                let line = line.trim();
                if !line.is_empty() && line != "Name" {
                    return line.to_string();
                }
            }
        }
    }

    // 降级方案：使用 hostname
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "Unknown Device".to_string());

    // 如果主机名看起来是默认值或太短，尝试获取更有意义的设备名
    let device_name = if hostname.len() < 3 || hostname == "localhost" {
        // 尝试从环境变量获取设备名（某些系统会设置）
        std::env::var("DEVICE_NAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| hostname)
    } else {
        // 移除 .local 后缀（macOS 常见格式）
        let name = hostname.strip_suffix(".local").unwrap_or(&hostname);
        name.to_string()
    };

    // 美化设备名称：将中划线替换为空格，处理常见格式
    let formatted = device_name
        .replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    formatted
}

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
pub async fn get_peer(
    state: State<'_, DiscoveryState>,
    peer_id: String,
) -> Result<Option<PeerInfo>, String> {
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