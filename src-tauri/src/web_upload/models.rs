//! Web 上传相关数据模型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Web 上传请求状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploadRequestStatus {
    /// 等待审批
    Pending,
    /// 已同意
    Accepted,
    /// 已拒绝
    Rejected,
    /// 已过期
    Expired,
}

impl Default for UploadRequestStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// 上传文件记录
///
/// 记录单个文件的上传状态和进度信息，
/// 同一 IP 的所有上传文件记录聚合在对应的 UploadRequest 下。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadRecord {
    /// 记录唯一 ID
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
    /// 状态：transferring / completed / failed
    pub status: String,
    /// 开始时间戳（毫秒）
    pub started_at: u64,
    /// 完成时间戳（毫秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<u64>,
}

#[allow(dead_code)]
impl UploadRecord {
    /// 创建新的上传记录
    pub fn new(file_name: String, total_bytes: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            id: Uuid::new_v4().to_string(),
            file_name,
            uploaded_bytes: 0,
            total_bytes,
            progress: 0.0,
            speed: 0,
            status: "transferring".to_string(),
            started_at: now,
            completed_at: None,
        }
    }
}

/// Web 上传请求（按 IP 审批的接收任务）
///
/// 每个客户端 IP 对应一条 UploadRequest，
/// 审批通过后该 IP 在整个会话期间都有上传权限，
/// 所有上传的文件记录聚合在 upload_records 中。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadRequest {
    /// 请求唯一 ID
    pub id: String,
    /// 上传方 IP 地址
    pub client_ip: String,
    /// 请求状态
    pub status: UploadRequestStatus,
    /// 请求时间戳（毫秒）
    pub created_at: u64,
    /// 上传方 User-Agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// 该 IP 下的所有上传文件记录
    pub upload_records: Vec<UploadRecord>,
}

impl UploadRequest {
    /// 创建新的上传请求（按 IP 创建，无需文件信息）
    pub fn new(client_ip: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            id: Uuid::new_v4().to_string(),
            client_ip,
            status: UploadRequestStatus::Pending,
            created_at: now,
            user_agent: None,
            upload_records: Vec::new(),
        }
    }
}

/// Web 上传服务器状态
#[derive(Debug)]
pub struct WebUploadState {
    /// 上传请求列表（请求 ID -> 请求）
    pub requests: HashMap<String, UploadRequest>,
    /// 已授权的 IP 地址列表
    pub allowed_ips: Vec<String>,
    /// 是否自动接收
    pub auto_receive: bool,
    /// 文件覆盖策略
    pub file_overwrite: bool,
    /// 接收目录
    pub receive_directory: String,
}

impl WebUploadState {
    /// 创建新的 Web 上传状态
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
            allowed_ips: Vec::new(),
            auto_receive: false,
            file_overwrite: false,
            receive_directory: String::new(),
        }
    }

    /// 检查 IP 是否已被拒绝
    pub fn is_ip_rejected(&self, ip: &str) -> bool {
        self.requests
            .values()
            .any(|r| r.client_ip == ip && r.status == UploadRequestStatus::Rejected)
    }

    /// 检查 IP 是否已被授权
    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        self.allowed_ips.contains(&ip.to_string())
    }
}

impl Default for WebUploadState {
    fn default() -> Self {
        Self::new()
    }
}
