//! 设备（Peer）模型

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    /// 设备 ID
    pub id: String,
    /// 设备名称
    pub name: String,
    /// IP 地址
    pub ip: String,
    /// 端口号
    pub port: u16,
    /// 设备类型
    pub device_type: DeviceType,
    /// 发现时间戳
    pub discovered_at: u64,
    /// 最后活跃时间戳
    pub last_seen: u64,
    /// 设备状态
    pub status: PeerStatus,
}

impl PeerInfo {
    /// 创建新的设备信息
    pub fn new(name: String, ip: String, port: u16) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            id: Uuid::new_v4().to_string(),
            name,
            ip,
            port,
            device_type: DeviceType::Unknown,
            discovered_at: now,
            last_seen: now,
            status: PeerStatus::Available,
        }
    }

    /// 更新最后活跃时间
    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            "Unknown Device"
        } else {
            &self.name
        }
    }

    /// 获取地址字符串
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    /// 检查设备是否在线（5秒内有活动）
    pub fn is_online(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now.saturating_sub(self.last_seen) < 5000
    }
}

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    /// 桌面端
    Desktop,
    /// 移动端
    Mobile,
    /// 网页端
    Web,
    /// 未知类型
    Unknown,
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 设备状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerStatus {
    /// 可用
    Available,
    /// 忙碌中（正在传输）
    Busy,
    /// 离线
    Offline,
}

impl Default for PeerStatus {
    fn default() -> Self {
        Self::Available
    }
}

/// 设备发现事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerDiscoveryEvent {
    /// 事件类型
    pub event_type: PeerEventType,
    /// 设备信息
    pub peer: PeerInfo,
}

/// 设备事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerEventType {
    /// 发现新设备
    Discovered,
    /// 设备更新
    Updated,
    /// 设备离线
    Offline,
}
