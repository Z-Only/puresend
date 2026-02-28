//! 设备发现管理器
//!
//! 统一管理设备发现和连接

use crate::discovery::MdnsDiscovery;
use crate::error::DiscoveryResult;
use crate::models::{PeerDiscoveryEvent, PeerInfo};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

/// 设备发现管理器
pub struct DiscoveryManager {
    /// mDNS 发现服务
    mdns: Arc<MdnsDiscovery>,
    /// 是否已启动
    started: Arc<Mutex<bool>>,
}

impl DiscoveryManager {
    /// 创建新的发现管理器
    pub fn new(device_name: String, listen_port: u16) -> Self {
        Self {
            mdns: Arc::new(MdnsDiscovery::new(device_name, listen_port)),
            started: Arc::new(Mutex::new(false)),
        }
    }

    /// 使用默认配置创建发现管理器
    pub fn default_manager() -> Self {
        Self {
            mdns: Arc::new(MdnsDiscovery::default()),
            started: Arc::new(Mutex::new(false)),
        }
    }

    /// 启动发现服务
    pub async fn start(&self) -> DiscoveryResult<()> {
        let mut started = self.started.lock().await;
        if *started {
            return Ok(());
        }

        self.mdns.start().await?;
        *started = true;

        Ok(())
    }

    /// 停止发现服务
    pub async fn stop(&self) -> DiscoveryResult<()> {
        let mut started = self.started.lock().await;
        if !*started {
            return Ok(());
        }

        self.mdns.stop().await?;
        *started = false;

        Ok(())
    }

    /// 获取所有已发现的设备
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        self.mdns.get_peers().await
    }

    /// 获取指定设备
    pub async fn get_peer(&self, id: &str) -> Option<PeerInfo> {
        self.mdns.get_peer(id).await
    }

    /// 订阅设备发现事件
    pub fn subscribe(&self) -> broadcast::Receiver<PeerDiscoveryEvent> {
        self.mdns.subscribe()
    }

    /// 手动添加设备
    pub async fn add_peer_manual(&self, ip: String, port: u16) -> PeerInfo {
        self.mdns.add_peer_manual(ip, port).await
    }

    /// 检查设备是否在线
    pub async fn is_peer_online(&self, id: &str) -> bool {
        self.mdns
            .get_peer(id)
            .await
            .map(|p| p.is_online())
            .unwrap_or(false)
    }

    /// 获取在线设备数量
    pub async fn online_count(&self) -> usize {
        self.mdns
            .get_peers()
            .await
            .iter()
            .filter(|p| p.is_online())
            .count()
    }
}

impl Default for DiscoveryManager {
    fn default() -> Self {
        Self::default_manager()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_manager() {
        let manager = DiscoveryManager::new("TestDevice".to_string(), 8080);
        assert!(!*manager.started.lock().await);
    }

    #[tokio::test]
    async fn test_get_peers_empty() {
        let manager = DiscoveryManager::default();
        let peers = manager.get_peers().await;
        assert!(peers.is_empty());
    }
}
