//! 分享相关数据模型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::FileMetadata;

/// 分享链接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkInfo {
    /// 分享链接
    pub link: String,
    /// HTTP 服务器端口
    pub port: u16,
    /// 分享的文件列表
    pub files: Vec<FileMetadata>,
    /// 创建时间戳（毫秒）
    pub created_at: u64,
    /// 是否启用 PIN 保护
    pub pin_enabled: bool,
    /// PIN 码（仅在启用时存在）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    /// 是否自动接受所有访问请求
    pub auto_accept: bool,
    /// 分享状态
    pub status: ShareStatus,
}

impl ShareLinkInfo {
    /// 创建新的分享链接信息
    pub fn new(link: String, port: u16, files: Vec<FileMetadata>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            link,
            port,
            files,
            created_at: now,
            pin_enabled: false,
            pin: None,
            auto_accept: false,
            status: ShareStatus::Active,
        }
    }

    /// 设置 PIN 码
    pub fn with_pin(mut self, pin: String) -> Self {
        self.pin_enabled = true;
        self.pin = Some(pin);
        self
    }

    /// 设置自动接受
    pub fn with_auto_accept(mut self, auto_accept: bool) -> Self {
        self.auto_accept = auto_accept;
        self
    }
}

/// 分享状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShareStatus {
    /// 活跃中
    Active,
    /// 已停止
    Stopped,
    /// 已过期
    Expired,
}

impl Default for ShareStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// 访问请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessRequest {
    /// 请求 ID
    pub id: String,
    /// 访问者 IP 地址
    pub ip: String,
    /// 请求时间戳（毫秒）
    pub requested_at: u64,
    /// 请求状态
    pub status: AccessRequestStatus,
    /// PIN 验证失败次数
    pub pin_attempts: u32,
    /// 是否被锁定（连续三次失败）
    pub locked: bool,
    /// 锁定解除时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_until: Option<u64>,
    /// 用户代理（浏览器/平台信息，如 "Chrome(Android)"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

impl AccessRequest {
    /// 创建新的访问请求
    pub fn new(ip: String, user_agent: Option<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            id: Uuid::new_v4().to_string(),
            ip,
            requested_at: now,
            status: AccessRequestStatus::Pending,
            pin_attempts: 0,
            locked: false,
            locked_until: None,
            user_agent,
        }
    }

    /// 记录 PIN 验证失败
    pub fn record_pin_failure(&mut self) {
        self.pin_attempts += 1;
        if self.pin_attempts >= 3 {
            self.locked = true;
            let locked_until = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + 5 * 60 * 1000; // 锁定 5 分钟
            self.locked_until = Some(locked_until);
        }
    }

    /// 检查是否仍然锁定
    pub fn is_still_locked(&self) -> bool {
        if !self.locked {
            return false;
        }
        if let Some(locked_until) = self.locked_until {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            if now >= locked_until {
                return false;
            }
        }
        true
    }

    /// 重置锁定状态
    pub fn reset_lock(&mut self) {
        self.locked = false;
        self.locked_until = None;
        self.pin_attempts = 0;
    }

    /// 接受请求
    pub fn accept(&mut self) {
        self.status = AccessRequestStatus::Accepted;
    }

    /// 拒绝请求
    pub fn reject(&mut self) {
        self.status = AccessRequestStatus::Rejected;
    }
}

/// 访问请求状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessRequestStatus {
    /// 待处理
    Pending,
    /// 已接受
    Accepted,
    /// 已拒绝
    Rejected,
}

impl Default for AccessRequestStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// 分享设置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareSettings {
    /// 是否启用 PIN 保护
    pub pin_enabled: bool,
    /// PIN 码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    /// 是否自动接受所有访问请求
    pub auto_accept: bool,
}

impl Default for ShareSettings {
    fn default() -> Self {
        Self {
            pin_enabled: false,
            pin: None,
            auto_accept: false,
        }
    }
}

/// PIN 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinVerifyResult {
    /// 是否验证成功
    pub success: bool,
    /// 剩余尝试次数（失败时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_attempts: Option<u32>,
    /// 是否被锁定
    pub locked: bool,
    /// 锁定解除时间（毫秒，锁定时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_until: Option<u64>,
}

/// 下载进度
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DownloadProgress {
    /// 下载 ID
    pub download_id: String,
    /// 文件名
    pub file_name: String,
    /// 进度百分比（0-100）
    pub progress: f64,
    /// 已下载字节数
    pub downloaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 下载速度（字节/秒）
    pub speed: u64,
    /// 访问者 IP
    pub client_ip: String,
}

/// 分享状态管理
#[derive(Debug, Clone)]
pub struct ShareState {
    /// 当前分享信息
    pub share_info: Option<ShareLinkInfo>,
    /// 访问请求列表
    pub access_requests: HashMap<String, AccessRequest>,
    /// 分享设置
    pub settings: ShareSettings,
    /// 已验证的 IP 地址（PIN 验证通过后）
    pub verified_ips: Vec<String>,
    /// 被拒绝的 IP 地址
    pub rejected_ips: Vec<String>,
}

impl ShareState {
    /// 创建新的分享状态
    pub fn new() -> Self {
        Self {
            share_info: None,
            access_requests: HashMap::new(),
            settings: ShareSettings::default(),
            verified_ips: Vec::new(),
            rejected_ips: Vec::new(),
        }
    }

    /// 开始分享
    pub fn start_share(&mut self, info: ShareLinkInfo, settings: ShareSettings) {
        self.share_info = Some(info);
        self.settings = settings;
        self.access_requests.clear();
        self.verified_ips.clear();
        self.rejected_ips.clear();
    }

    /// 停止分享
    pub fn stop_share(&mut self) {
        if let Some(info) = &mut self.share_info {
            info.status = ShareStatus::Stopped;
        }
        self.share_info = None;
        self.access_requests.clear();
        self.verified_ips.clear();
        self.rejected_ips.clear();
    }

    /// 添加访问请求
    #[allow(dead_code)]
    pub fn add_access_request(&mut self, request: AccessRequest) {
        self.access_requests.insert(request.id.clone(), request);
    }

    /// 接受访问请求
    pub fn accept_request(&mut self, request_id: &str) -> Option<&AccessRequest> {
        if let Some(request) = self.access_requests.get_mut(request_id) {
            request.accept();
            if !self.verified_ips.contains(&request.ip) {
                self.verified_ips.push(request.ip.clone());
            }
            // 从拒绝列表中移除（如果存在）
            self.rejected_ips.retain(|ip| ip != &request.ip);
            Some(request)
        } else {
            None
        }
    }

    /// 拒绝访问请求
    pub fn reject_request(&mut self, request_id: &str) -> Option<&AccessRequest> {
        if let Some(request) = self.access_requests.get_mut(request_id) {
            request.reject();
            if !self.rejected_ips.contains(&request.ip) {
                self.rejected_ips.push(request.ip.clone());
            }
            // 从验证列表中移除（如果存在）
            self.verified_ips.retain(|ip| ip != &request.ip);
            Some(request)
        } else {
            None
        }
    }

    /// 检查 IP 是否已被验证
    pub fn is_ip_verified(&self, ip: &str) -> bool {
        self.verified_ips.contains(&ip.to_string())
    }

    /// 检查 IP 是否已被拒绝
    pub fn is_ip_rejected(&self, ip: &str) -> bool {
        self.rejected_ips.contains(&ip.to_string())
    }

    /// 检查 IP 是否有访问权限（请求已被接受）
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        // 检查是否有已接受的访问请求
        self.access_requests
            .values()
            .any(|r| r.ip == ip && r.status == AccessRequestStatus::Accepted)
    }

    /// 根据 IP 获取访问请求
    #[allow(dead_code)]
    pub fn get_request_by_ip(&mut self, ip: &str) -> Option<&mut AccessRequest> {
        self.access_requests.values_mut().find(|r| r.ip == ip)
    }

    /// 验证 PIN 码
    pub fn verify_pin(&mut self, ip: &str, pin: &str) -> PinVerifyResult {
        // 检查是否被拒绝
        if self.is_ip_rejected(ip) {
            return PinVerifyResult {
                success: false,
                remaining_attempts: None,
                locked: false,
                locked_until: None,
            };
        }

        // 获取或创建访问请求
        let existing_request = self.access_requests.values_mut().find(|r| r.ip == ip);

        if let Some(request) = existing_request {
            // 检查是否锁定
            if request.is_still_locked() {
                return PinVerifyResult {
                    success: false,
                    remaining_attempts: Some(0),
                    locked: true,
                    locked_until: request.locked_until,
                };
            }

            // 如果请求已被接受，直接通过
            if request.status == AccessRequestStatus::Accepted {
                return PinVerifyResult {
                    success: true,
                    remaining_attempts: None,
                    locked: false,
                    locked_until: None,
                };
            }

            // 验证 PIN
            if let Some(ref correct_pin) = self.settings.pin {
                if pin == correct_pin {
                    request.reset_lock();
                    // 根据 auto_accept 设置决定是否自动接受
                    if self.settings.auto_accept {
                        // 自动接受：添加到已验证 IP 列表
                        if !self.verified_ips.contains(&ip.to_string()) {
                            self.verified_ips.push(ip.to_string());
                        }
                        request.accept();
                    }
                    // 如果 auto_accept=false，保持 Pending 状态，不添加到 verified_ips
                    return PinVerifyResult {
                        success: true,
                        remaining_attempts: None,
                        locked: false,
                        locked_until: None,
                    };
                } else {
                    request.record_pin_failure();
                    let remaining = 3u32.saturating_sub(request.pin_attempts);
                    return PinVerifyResult {
                        success: false,
                        remaining_attempts: Some(remaining),
                        locked: request.locked,
                        locked_until: request.locked_until,
                    };
                }
            }
        } else {
            // 创建新的访问请求
            let mut new_request = AccessRequest::new(ip.to_string(), None);

            // 验证 PIN
            if let Some(ref correct_pin) = self.settings.pin {
                if pin == correct_pin {
                    // 根据 auto_accept 设置决定是否自动接受
                    if self.settings.auto_accept {
                        // 自动接受：添加到已验证 IP 列表
                        if !self.verified_ips.contains(&ip.to_string()) {
                            self.verified_ips.push(ip.to_string());
                        }
                        new_request.status = AccessRequestStatus::Accepted;
                    }
                    // 如果 auto_accept=false，保持 Pending 状态，不添加到 verified_ips
                    // 无论是否自动接受，都添加到请求列表
                    self.access_requests
                        .insert(new_request.id.clone(), new_request);
                    return PinVerifyResult {
                        success: true,
                        remaining_attempts: None,
                        locked: false,
                        locked_until: None,
                    };
                } else {
                    new_request.record_pin_failure();
                    let remaining = 3u32.saturating_sub(new_request.pin_attempts);
                    let locked_until = new_request.locked_until;
                    let locked = new_request.locked;
                    self.access_requests
                        .insert(new_request.id.clone(), new_request);
                    return PinVerifyResult {
                        success: false,
                        remaining_attempts: Some(remaining),
                        locked,
                        locked_until,
                    };
                }
            } else {
                // 没有设置 PIN，根据 auto_accept 设置决定是否自动接受
                if self.settings.auto_accept {
                    new_request.status = AccessRequestStatus::Accepted;
                    if !self.verified_ips.contains(&ip.to_string()) {
                        self.verified_ips.push(ip.to_string());
                    }
                }
                // 添加到请求列表
                self.access_requests
                    .insert(new_request.id.clone(), new_request);
                return PinVerifyResult {
                    success: true,
                    remaining_attempts: None,
                    locked: false,
                    locked_until: None,
                };
            }
        }

        // 没有设置 PIN 且没有 auto_accept，创建 Pending 状态的请求
        let new_request = AccessRequest::new(ip.to_string(), None);
        self.access_requests
            .insert(new_request.id.clone(), new_request);
        PinVerifyResult {
            success: true,
            remaining_attempts: None,
            locked: false,
            locked_until: None,
        }
    }
}

impl Default for ShareState {
    fn default() -> Self {
        Self::new()
    }
}