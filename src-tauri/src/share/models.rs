//! 分享相关数据模型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::FileMetadata;

/// PIN 验证失败后的锁定时间（毫秒）：5 分钟
const PIN_LOCK_DURATION_MS: u64 = 5 * 60 * 1000;
/// PIN 验证最大失败次数
const MAX_PIN_ATTEMPTS: u32 = 3;

/// 获取当前时间戳（毫秒），如果系统时钟异常则返回 0
fn current_timestamp_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// 分享链接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareLinkInfo {
    /// 分享链接列表
    pub links: Vec<String>,
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
    pub fn new(links: Vec<String>, port: u16, files: Vec<FileMetadata>) -> Self {
        let now = current_timestamp_millis();

        Self {
            links,
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

/// 传输状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    /// 空闲
    Idle,
    /// 传输中
    Transferring,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 失败
    Failed,
}

impl Default for TransferStatus {
    fn default() -> Self {
        Self::Idle
    }
}

/// 上传记录
///
/// 从分享者（应用）视角来看，接收者通过链接获取文件时，
/// 应用作为文件提供方，实际上是在上传文件给接收者。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareUploadRecord {
    /// 上传记录唯一 ID
    pub id: String,
    /// 文件名
    pub file_name: String,
    /// 已上传字节数
    pub uploaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 进度百分比（0-100）
    pub progress: f64,
    /// 上传速度（字节/秒）
    pub speed: u64,
    /// 上传状态
    pub status: TransferStatus,
    /// 开始时间（毫秒）
    pub started_at: u64,
    /// 完成时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
}

impl ShareUploadRecord {
    /// 创建新的上传记录
    pub fn new(file_name: String, total_bytes: u64) -> Self {
        let now = current_timestamp_millis();

        Self {
            id: Uuid::new_v4().to_string(),
            file_name,
            uploaded_bytes: 0,
            total_bytes,
            progress: 0.0,
            speed: 0,
            status: TransferStatus::Transferring,
            started_at: now,
            completed_at: None,
        }
    }
}

/// PIN 尝试状态（用于 PIN 验证前的锁定机制）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PinAttemptState {
    /// 访问者 IP 地址
    pub ip: String,
    /// PIN 验证失败次数
    pub attempts: u32,
    /// 是否被锁定
    pub locked: bool,
    /// 锁定解除时间（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_until: Option<u64>,
}

impl PinAttemptState {
    /// 创建新的 PIN 尝试状态
    pub fn new(ip: String) -> Self {
        Self {
            ip,
            attempts: 0,
            locked: false,
            locked_until: None,
        }
    }

    /// 记录 PIN 验证失败
    pub fn record_failure(&mut self) {
        self.attempts += 1;
        if self.attempts >= MAX_PIN_ATTEMPTS {
            self.locked = true;
            let locked_until = current_timestamp_millis() + PIN_LOCK_DURATION_MS;
            self.locked_until = Some(locked_until);
        }
    }

    /// 检查是否仍然锁定
    pub fn is_still_locked(&self) -> bool {
        if !self.locked {
            return false;
        }
        if let Some(locked_until) = self.locked_until {
            let now = current_timestamp_millis();
            if now >= locked_until {
                return false;
            }
        }
        true
    }

    /// 获取剩余锁定时间（毫秒）
    pub fn remaining_lock_time(&self) -> u64 {
        if !self.locked {
            return 0;
        }
        if let Some(locked_until) = self.locked_until {
            let now = current_timestamp_millis();
            if now >= locked_until {
                0
            } else {
                locked_until - now
            }
        } else {
            0
        }
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
    /// 上传记录列表
    pub upload_records: Vec<ShareUploadRecord>,
}

impl AccessRequest {
    /// 创建新的访问请求
    pub fn new(ip: String, user_agent: Option<String>) -> Self {
        let now = current_timestamp_millis();

        Self {
            id: Uuid::new_v4().to_string(),
            ip,
            requested_at: now,
            status: AccessRequestStatus::Pending,
            pin_attempts: 0,
            locked: false,
            locked_until: None,
            user_agent,
            upload_records: Vec::new(),
        }
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

/// 上传进度
///
/// 从分享者视角，文件被接收者获取时的传输进度。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgress {
    /// 上传 ID
    pub upload_id: String,
    /// 文件名
    pub file_name: String,
    /// 进度百分比（0-100）
    pub progress: f64,
    /// 已上传字节数
    pub uploaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 上传速度（字节/秒）
    pub speed: u64,
    /// 接收者 IP
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
    /// PIN 尝试状态（IP -> PinAttemptState）
    pub pin_attempts: HashMap<String, PinAttemptState>,
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
            pin_attempts: HashMap::new(),
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
        self.pin_attempts.clear();
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

    /// 移除单个访问请求
    pub fn remove_request(&mut self, request_id: &str) -> Option<AccessRequest> {
        self.access_requests.remove(request_id)
    }
}

impl Default for ShareState {
    fn default() -> Self {
        Self::new()
    }
}
