//! mDNS 服务发现模块
//!
//! 使用多播 DNS 在本地网络中发现 PureSend 设备

use crate::error::DiscoveryResult;
use crate::models::{DeviceType, PeerDiscoveryEvent, PeerEventType, PeerInfo, PeerStatus};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};

/// mDNS 服务名称
#[allow(dead_code)]
pub const SERVICE_NAME: &str = "_puresend._tcp.local.";

/// mDNS 多播地址
#[allow(dead_code)]
pub const MDNS_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);

/// mDNS 端口
pub const MDNS_PORT: u16 = 5353;

/// 发现超时时间
#[allow(dead_code)]
pub const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

/// 设备过期时间（10秒无响应视为离线）
pub const PEER_EXPIRE_TIMEOUT: Duration = Duration::from_secs(10);

/// mDNS 服务发现
pub struct MdnsDiscovery {
    /// 本机设备名称
    device_name: String,
    /// 本机监听端口
    listen_port: u16,
    /// 已发现的设备列表
    peers: Arc<Mutex<HashMap<String, PeerInfo>>>,
    /// 事件广播发送器
    event_sender: broadcast::Sender<PeerDiscoveryEvent>,
    /// 是否正在运行
    running: Arc<Mutex<bool>>,
}

impl MdnsDiscovery {
    /// 创建新的 mDNS 发现实例
    pub fn new(device_name: String, listen_port: u16) -> Self {
        let (event_sender, _) = broadcast::channel(100);
        Self {
            device_name,
            listen_port,
            peers: Arc::new(Mutex::new(HashMap::new())),
            event_sender,
            running: Arc::new(Mutex::new(false)),
        }
    }

    /// 获取事件接收器
    pub fn subscribe(&self) -> broadcast::Receiver<PeerDiscoveryEvent> {
        self.event_sender.subscribe()
    }

    /// 启动发现服务
    pub async fn start(&self) -> DiscoveryResult<()> {
        let mut running = self.running.lock().await;
        if *running {
            return Ok(());
        }

        // 由于 mdns_sd 库还未添加到依赖，这里使用简化的 UDP 广播实现
        // 实际生产环境应使用专业的 mDNS 库
        *running = true;

        // 启动广播和监听任务
        self.start_broadcast_task().await;
        self.start_listen_task().await;
        self.start_cleanup_task().await;

        Ok(())
    }

    /// 停止发现服务
    pub async fn stop(&self) -> DiscoveryResult<()> {
        let mut running = self.running.lock().await;
        *running = false;
        self.peers.lock().await.clear();
        Ok(())
    }

    /// 启动广播任务
    async fn start_broadcast_task(&self) {
        let device_name = self.device_name.clone();
        let listen_port = self.listen_port;
        let running = self.running.clone();

        tokio::spawn(async move {
            // 创建 UDP socket
            let socket = match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(_) => return,
            };

            let broadcast_addr =
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)), MDNS_PORT);

            // 构造广播消息
            let message = DiscoveryMessage {
                device_name: device_name.clone(),
                port: listen_port,
                device_type: DeviceType::Desktop,
            };
            let message_bytes = match serde_json::to_vec(&message) {
                Ok(b) => b,
                Err(_) => return,
            };

            loop {
                let is_running = *running.lock().await;
                if !is_running {
                    break;
                }

                // 发送广播
                if socket
                    .send_to(&message_bytes, broadcast_addr)
                    .await
                    .is_err()
                {
                    // 发送失败，可能网络不可用，继续尝试
                }

                // 每 3 秒广播一次
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        });
    }

    /// 启动监听任务
    async fn start_listen_task(&self) {
        let peers = self.peers.clone();
        let event_sender = self.event_sender.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            // 创建 UDP socket 监听广播
            let socket = match tokio::net::UdpSocket::bind(format!("0.0.0.0:{}", MDNS_PORT)).await {
                Ok(s) => s,
                Err(_) => {
                    // 端口可能被占用，尝试使用其他端口
                    match tokio::net::UdpSocket::bind("0.0.0.0:0").await {
                        Ok(s) => s,
                        Err(_) => return,
                    }
                }
            };

            let mut buf = vec![0u8; 4096];

            loop {
                let is_running = *running.lock().await;
                if !is_running {
                    break;
                }

                // 接收消息
                match socket.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        // 解析消息
                        if let Ok(message) = serde_json::from_slice::<DiscoveryMessage>(&buf[..len])
                        {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64;

                            let peer = PeerInfo {
                                id: format!("{}-{}", message.device_name, addr.ip()),
                                name: message.device_name.clone(),
                                ip: addr.ip().to_string(),
                                port: message.port,
                                device_type: message.device_type,
                                discovered_at: now,
                                last_seen: now,
                                status: PeerStatus::Available,
                            };

                            // 更新设备列表
                            let mut peers_guard = peers.lock().await;
                            let event_type = if peers_guard.contains_key(&peer.id) {
                                PeerEventType::Updated
                            } else {
                                PeerEventType::Discovered
                            };

                            peers_guard.insert(peer.id.clone(), peer.clone());

                            // 发送事件
                            let _ = event_sender.send(PeerDiscoveryEvent { event_type, peer });
                        }
                    }
                    Err(_) => continue,
                }
            }
        });
    }

    /// 启动清理任务（清理过期设备）
    async fn start_cleanup_task(&self) {
        let peers = self.peers.clone();
        let event_sender = self.event_sender.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            loop {
                let is_running = *running.lock().await;
                if !is_running {
                    break;
                }

                // 每 5 秒清理一次
                tokio::time::sleep(Duration::from_secs(5)).await;

                let mut peers_guard = peers.lock().await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;

                // 找出过期设备
                let expired: Vec<String> = peers_guard
                    .iter()
                    .filter(|(_, peer)| {
                        now.saturating_sub(peer.last_seen) > PEER_EXPIRE_TIMEOUT.as_millis() as u64
                    })
                    .map(|(id, _)| id.clone())
                    .collect();

                // 移除过期设备并发送事件
                for id in expired {
                    if let Some(peer) = peers_guard.remove(&id) {
                        let _ = event_sender.send(PeerDiscoveryEvent {
                            event_type: PeerEventType::Offline,
                            peer,
                        });
                    }
                }
            }
        });
    }

    /// 获取当前发现的设备列表
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        self.peers.lock().await.values().cloned().collect()
    }

    /// 获取指定设备信息
    pub async fn get_peer(&self, id: &str) -> Option<PeerInfo> {
        self.peers.lock().await.get(id).cloned()
    }

    /// 手动添加设备（用于手动连接）
    #[allow(dead_code)]
    pub async fn add_peer_manual(&self, ip: String, port: u16) -> PeerInfo {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let peer = PeerInfo {
            id: format!("manual-{}:{}", ip, port),
            name: format!("手动添加 ({}:{})", ip, port),
            ip,
            port,
            device_type: DeviceType::Unknown,
            discovered_at: now,
            last_seen: now,
            status: PeerStatus::Available,
        };

        let mut peers = self.peers.lock().await;
        peers.insert(peer.id.clone(), peer.clone());

        let _ = self.event_sender.send(PeerDiscoveryEvent {
            event_type: PeerEventType::Discovered,
            peer: peer.clone(),
        });

        peer
    }
}

/// 发现消息格式
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DiscoveryMessage {
    /// 设备名称
    device_name: String,
    /// 监听端口
    port: u16,
    /// 设备类型
    device_type: DeviceType,
}

impl Default for MdnsDiscovery {
    fn default() -> Self {
        Self::new(
            hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "PureSend Device".to_string()),
            0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_name() {
        assert!(SERVICE_NAME.contains("puresend"));
    }

    #[tokio::test]
    async fn test_create_discovery() {
        let discovery = MdnsDiscovery::new("TestDevice".to_string(), 8080);
        assert_eq!(discovery.device_name, "TestDevice");
        assert_eq!(discovery.listen_port, 8080);
    }
}
