//! 传输任务模型

use crate::models::{FileMetadata, PeerInfo};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// 传输任务
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferTask {
    /// 任务 ID
    pub id: String,
    /// 文件元数据
    pub file: FileMetadata,
    /// 传输模式
    pub mode: TransferMode,
    /// 目标设备（P2P 模式）
    pub peer: Option<PeerInfo>,
    /// 任务状态
    pub status: TaskStatus,
    /// 进度百分比（0-100）
    pub progress: f64,
    /// 已传输字节数
    pub transferred_bytes: u64,
    /// 传输速度（字节/秒）
    pub speed: u64,
    /// 创建时间戳（毫秒）
    pub created_at: u64,
    /// 完成时间戳（毫秒）
    pub completed_at: Option<u64>,
    /// 错误信息
    pub error: Option<String>,
    /// 传输方向
    pub direction: TransferDirection,
    /// 是否可恢复（断点续传）
    #[serde(default)]
    pub resumable: bool,
    /// 续传偏移量（已传输字节数）
    #[serde(default)]
    pub resume_offset: u64,
    /// 是否为恢复的传输
    #[serde(default)]
    pub resumed: bool,
    /// 是否使用加密传输
    #[serde(default)]
    pub encrypted: bool,
    /// 压缩率（百分比，0 表示未压缩）
    #[serde(default)]
    pub compression_ratio: f64,
}

impl TransferTask {
    /// 创建新的传输任务
    pub fn new(file: FileMetadata, mode: TransferMode, direction: TransferDirection) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            id: Uuid::new_v4().to_string(),
            file,
            mode,
            peer: None,
            status: TaskStatus::Pending,
            progress: 0.0,
            transferred_bytes: 0,
            speed: 0,
            created_at: now,
            completed_at: None,
            error: None,
            direction,
            resumable: false,
            resume_offset: 0,
            resumed: false,
            encrypted: false,
            compression_ratio: 0.0,
        }
    }

    /// 设置目标设备
    pub fn with_peer(mut self, peer: PeerInfo) -> Self {
        self.peer = Some(peer);
        self
    }

    /// 更新进度
    #[allow(dead_code)]
    pub fn update_progress(&mut self, transferred_bytes: u64, speed: u64) {
        self.transferred_bytes = transferred_bytes;
        self.speed = speed;
        if self.file.size > 0 {
            self.progress = (transferred_bytes as f64 / self.file.size as f64) * 100.0;
        }
    }

    /// 标记为传输中
    pub fn start(&mut self) {
        self.status = TaskStatus::Transferring;
    }

    /// 标记为完成
    #[allow(dead_code)]
    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.progress = 100.0;
        self.transferred_bytes = self.file.size;
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    /// 标记为失败
    pub fn fail(&mut self, error: String) {
        self.status = TaskStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    /// 标记为取消
    #[allow(dead_code)]
    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.completed_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }

    /// 标记为中断（可恢复）
    #[allow(dead_code)]
    pub fn interrupt(&mut self) {
        self.status = TaskStatus::Interrupted;
        self.resumable = true;
    }

    /// 计算预估剩余时间（秒）
    pub fn estimated_time_remaining(&self) -> Option<u64> {
        if self.speed == 0 {
            return None;
        }
        let remaining_bytes = self.file.size.saturating_sub(self.transferred_bytes);
        Some(remaining_bytes / self.speed)
    }
}

/// 传输模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferMode {
    /// 本地网络直连
    Local,
    /// 云盘中转
    Cloud,
}

impl Default for TransferMode {
    fn default() -> Self {
        Self::Local
    }
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// 等待中
    Pending,
    /// 传输中
    Transferring,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 已中断（可恢复）
    Interrupted,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// 传输方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    /// 发送
    Send,
    /// 接收
    Receive,
}

/// 传输进度事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    /// 任务 ID
    pub task_id: String,
    /// 状态
    pub status: TaskStatus,
    /// 进度百分比
    pub progress: f64,
    /// 已传输字节数
    pub transferred_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 传输速度（字节/秒）
    pub speed: u64,
    /// 预估剩余时间（秒）
    pub estimated_time_remaining: Option<u64>,
    /// 错误信息
    pub error: Option<String>,
}

impl From<&TransferTask> for TransferProgress {
    fn from(task: &TransferTask) -> Self {
        Self {
            task_id: task.id.clone(),
            status: task.status,
            progress: task.progress,
            transferred_bytes: task.transferred_bytes,
            total_bytes: task.file.size,
            speed: task.speed,
            estimated_time_remaining: task.estimated_time_remaining(),
            error: task.error.clone(),
        }
    }
}
