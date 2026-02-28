//! 网络工具模块

use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

/// 获取本地所有有效的 IPv4 地址
///
/// 返回的地址列表按优先级排序：
/// - 私有网段（192.168.x.x、10.x.x.x、172.16-31.x.x）优先
/// - 公网 IP 次之
///
/// 如果没有找到任何有效 IP，返回 localhost 地址作为回退
pub fn get_local_ips() -> Vec<String> {
    use local_ip_address::list_afinet_netifas;

    let mut ips: Vec<(String, u8)> = Vec::new();

    // 枚举所有网络接口
    if let Ok(network_interfaces) = list_afinet_netifas() {
        for (_, ip_addr) in network_interfaces {
            // 只处理 IPv4 地址
            if let std::net::IpAddr::V4(ipv4) = ip_addr {
                // 过滤掉回环地址（127.x.x.x）
                if ipv4.is_loopback() {
                    continue;
                }

                // 过滤掉 link-local 地址（169.254.x.x）
                if is_link_local(ipv4) {
                    continue;
                }

                // 根据优先级分配权重
                let priority = get_ip_priority(ipv4);
                ips.push((ipv4.to_string(), priority));
            }
        }
    }

    // 按优先级排序（权重越小优先级越高）
    ips.sort_by_key(|(_, priority)| *priority);

    // 提取 IP 地址
    let result: Vec<String> = ips.into_iter().map(|(ip, _)| ip).collect();

    // 如果没有找到任何有效 IP，返回 localhost 作为回退
    if result.is_empty() {
        vec!["127.0.0.1".to_string()]
    } else {
        result
    }
}

/// 判断是否为 link-local 地址（169.254.x.x）
fn is_link_local(ip: Ipv4Addr) -> bool {
    ip.octets()[0] == 169 && ip.octets()[1] == 254
}

/// 获取 IP 地址的优先级权重
///
/// 返回值越小，优先级越高：
/// - 0: 192.168.x.x
/// - 1: 10.x.x.x
/// - 2: 172.16-31.x.x
/// - 3: 其他公网 IP
fn get_ip_priority(ip: Ipv4Addr) -> u8 {
    let octets = ip.octets();

    // 192.168.x.x - 优先级最高
    if octets[0] == 192 && octets[1] == 168 {
        return 0;
    }

    // 10.x.x.x
    if octets[0] == 10 {
        return 1;
    }

    // 172.16-31.x.x
    if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
        return 2;
    }

    // 其他公网 IP
    3
}

// ============ 网络变化检测 ============

/// 网络变化轮询间隔
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// 防抖窗口时长
const DEBOUNCE_WINDOW: Duration = Duration::from_secs(2);

/// 防抖最大等待时长
const DEBOUNCE_MAX_WAIT: Duration = Duration::from_secs(10);

/// 网络变化类型
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkChangeType {
    /// IP 地址发生变化（网络切换）
    IpChanged,
    /// 网络完全断开
    Disconnected,
    /// 网络从断开状态恢复
    Reconnected,
}

/// 网络变化事件载荷
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkChangedPayload {
    /// 变化类型
    pub change_type: NetworkChangeType,
    /// 新的 IP 地址列表（网络断开时为空）
    pub ip_addresses: Vec<String>,
    /// 上一次的 IP 地址列表
    pub previous_ip_addresses: Vec<String>,
}

/// 网络变化回调类型
pub type NetworkChangeCallback = Arc<dyn Fn(NetworkChangedPayload) + Send + Sync>;

/// 网络状态监视器
///
/// 通过定时轮询 `get_local_ips()` 检测网络变化，
/// 并在防抖窗口结束后通过 Tauri 事件通知前端。
pub struct NetworkWatcher {
    /// 是否正在运行
    running: Arc<Mutex<bool>>,
    /// 网络变化时的额外回调（用于 mDNS 重启等）
    on_change_callback: Arc<Mutex<Option<NetworkChangeCallback>>>,
}

impl NetworkWatcher {
    /// 创建新的网络监视器
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            on_change_callback: Arc::new(Mutex::new(None)),
        }
    }

    /// 设置网络变化回调（用于 mDNS 重启等外部联动）
    pub async fn set_on_change_callback(&self, callback: NetworkChangeCallback) {
        let mut cb = self.on_change_callback.lock().await;
        *cb = Some(callback);
    }

    /// 启动网络监视器
    pub async fn start(&self, app_handle: AppHandle) {
        let mut running = self.running.lock().await;
        if *running {
            return;
        }
        *running = true;

        let running_flag = self.running.clone();
        let on_change_callback = self.on_change_callback.clone();

        tokio::spawn(async move {
            let mut last_ips = get_local_ips();
            // 防抖状态：首次检测到变化的时间
            let mut debounce_first_change: Option<Instant> = None;
            // 防抖状态：最后一次检测到变化的时间
            let mut debounce_last_change: Option<Instant> = None;
            // 防抖开始前的 IP 列表（用于 previous_ip_addresses）
            let mut ips_before_debounce: Vec<String> = Vec::new();

            loop {
                // 检查是否仍在运行
                {
                    let is_running = *running_flag.lock().await;
                    if !is_running {
                        break;
                    }
                }

                tokio::time::sleep(POLL_INTERVAL).await;

                // 再次检查运行状态（sleep 期间可能已停止）
                {
                    let is_running = *running_flag.lock().await;
                    if !is_running {
                        break;
                    }
                }

                let current_ips = get_local_ips();

                // 对比 IP 列表是否发生变化
                if current_ips != last_ips {
                    let now = Instant::now();

                    // 首次检测到变化时，保存变化前的 IP 列表
                    if debounce_first_change.is_none() {
                        debounce_first_change = Some(now);
                        ips_before_debounce = last_ips.clone();
                    }
                    debounce_last_change = Some(now);

                    // 更新 last_ips 用于下次对比
                    last_ips = current_ips;
                }

                // 检查防抖窗口是否应该触发
                if let (Some(first_change), Some(last_change)) =
                    (debounce_first_change, debounce_last_change)
                {
                    let now = Instant::now();
                    let since_last = now.duration_since(last_change);
                    let since_first = now.duration_since(first_change);

                    // 防抖窗口结束（距最后一次变化超过 2 秒）或达到最大等待时间（10 秒）
                    if since_last >= DEBOUNCE_WINDOW || since_first >= DEBOUNCE_MAX_WAIT {
                        // 重新获取当前最新 IP 以确保准确
                        let final_ips = get_local_ips();

                        // 判断网络是否断开（仅有回环地址）
                        let is_disconnected =
                            final_ips.len() == 1 && final_ips[0] == "127.0.0.1";
                        let was_disconnected =
                            ips_before_debounce.len() == 1
                                && ips_before_debounce[0] == "127.0.0.1";

                        let change_type = if is_disconnected {
                            NetworkChangeType::Disconnected
                        } else if was_disconnected {
                            NetworkChangeType::Reconnected
                        } else {
                            NetworkChangeType::IpChanged
                        };

                        let payload = NetworkChangedPayload {
                            change_type,
                            ip_addresses: final_ips.clone(),
                            previous_ip_addresses: ips_before_debounce.clone(),
                        };

                        // 发送 Tauri 事件通知前端
                        let _ = app_handle.emit("network-changed", &payload);

                        // 调用外部回调（mDNS 重启等）
                        {
                            let cb_guard = on_change_callback.lock().await;
                            if let Some(ref callback) = *cb_guard {
                                callback(payload);
                            }
                        }

                        // 重置防抖状态
                        last_ips = final_ips;
                        debounce_first_change = None;
                        debounce_last_change = None;
                        ips_before_debounce = Vec::new();
                    }
                }
            }
        });
    }

    /// 停止网络监视器
    #[allow(dead_code)]
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }
}

impl Default for NetworkWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// 网络监视器状态（用于 Tauri 状态管理）
pub struct NetworkWatcherState {
    /// 网络监视器实例
    pub watcher: Arc<NetworkWatcher>,
}

impl NetworkWatcherState {
    pub fn new() -> Self {
        Self {
            watcher: Arc::new(NetworkWatcher::new()),
        }
    }
}

impl Default for NetworkWatcherState {
    fn default() -> Self {
        Self::new()
    }
}
