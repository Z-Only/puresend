//! 断点续传模块
//!
//! 提供传输中断后的断点信息持久化存储和恢复功能。
//! 断点信息以 JSON 文件形式存储在应用数据目录下，24 小时后自动过期清理。

use crate::error::{TransferError, TransferResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// 断点信息过期时间：24 小时（毫秒）
const RESUME_INFO_EXPIRY_MS: u64 = 24 * 60 * 60 * 1000;

/// 断点信息存储文件名
const RESUME_INFO_FILENAME: &str = "resume_info.json";

/// 单个任务的断点信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeInfo {
    /// 任务 ID
    pub task_id: String,
    /// 文件名
    pub file_name: String,
    /// 文件总大小
    pub file_size: u64,
    /// 文件哈希（用于验证是否为同一文件）
    pub file_hash: String,
    /// 已传输字节数
    pub transferred_bytes: u64,
    /// 最后一个成功传输的分块索引
    pub last_chunk_index: u32,
    /// 中断时间戳（毫秒）
    pub interrupted_at: u64,
    /// 过期时间戳（毫秒）
    pub expires_at: u64,
    /// 目标设备 IP
    pub peer_ip: String,
    /// 目标设备端口
    pub peer_port: u16,
    /// 传输方向（"send" 或 "receive"）
    pub direction: String,
    /// 接收文件的保存路径（仅接收方有效）
    pub save_path: Option<String>,
}

impl ResumeInfo {
    /// 创建新的断点信息
    pub fn new(
        task_id: String,
        file_name: String,
        file_size: u64,
        file_hash: String,
        transferred_bytes: u64,
        last_chunk_index: u32,
        peer_ip: String,
        peer_port: u16,
        direction: String,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            task_id,
            file_name,
            file_size,
            file_hash,
            transferred_bytes,
            last_chunk_index,
            interrupted_at: now,
            expires_at: now + RESUME_INFO_EXPIRY_MS,
            peer_ip,
            peer_port,
            direction,
            save_path: None,
        }
    }

    /// 检查断点信息是否已过期
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now > self.expires_at
    }
}

/// 可恢复任务信息（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumableTaskInfo {
    /// 任务 ID
    pub task_id: String,
    /// 文件名
    pub file_name: String,
    /// 文件总大小
    pub file_size: u64,
    /// 已传输字节数
    pub transferred_bytes: u64,
    /// 中断时间戳（毫秒）
    pub interrupted_at: u64,
    /// 过期时间戳（毫秒）
    pub expires_at: u64,
}

impl From<&ResumeInfo> for ResumableTaskInfo {
    fn from(info: &ResumeInfo) -> Self {
        Self {
            task_id: info.task_id.clone(),
            file_name: info.file_name.clone(),
            file_size: info.file_size,
            transferred_bytes: info.transferred_bytes,
            interrupted_at: info.interrupted_at,
            expires_at: info.expires_at,
        }
    }
}

/// 断点续传管理器
///
/// 负责断点信息的内存缓存、持久化存储和过期清理。
pub struct ResumeManager {
    /// 断点信息缓存（task_id -> ResumeInfo）
    resume_infos: Arc<RwLock<HashMap<String, ResumeInfo>>>,
    /// 存储目录
    storage_dir: PathBuf,
}

impl ResumeManager {
    /// 创建新的断点续传管理器
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            resume_infos: Arc::new(RwLock::new(HashMap::new())),
            storage_dir,
        }
    }

    /// 获取存储文件路径
    fn storage_path(&self) -> PathBuf {
        self.storage_dir.join(RESUME_INFO_FILENAME)
    }

    /// 从磁盘加载断点信息
    pub async fn load(&self) -> TransferResult<()> {
        let path = self.storage_path();
        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| TransferError::ResumeFailed(format!("读取断点信息文件失败: {}", e)))?;

        let infos: HashMap<String, ResumeInfo> = serde_json::from_str(&content)
            .map_err(|e| TransferError::ResumeFailed(format!("解析断点信息失败: {}", e)))?;

        // 过滤掉已过期的断点信息
        let valid_infos: HashMap<String, ResumeInfo> = infos
            .into_iter()
            .filter(|(_, info)| !info.is_expired())
            .collect();

        let mut cache = self.resume_infos.write().await;
        *cache = valid_infos;

        Ok(())
    }

    /// 将断点信息持久化到磁盘
    pub async fn save(&self) -> TransferResult<()> {
        // 确保存储目录存在
        if !self.storage_dir.exists() {
            tokio::fs::create_dir_all(&self.storage_dir)
                .await
                .map_err(|e| {
                    TransferError::ResumeFailed(format!("创建存储目录失败: {}", e))
                })?;
        }

        let cache = self.resume_infos.read().await;
        let content = serde_json::to_string_pretty(&*cache)
            .map_err(|e| TransferError::ResumeFailed(format!("序列化断点信息失败: {}", e)))?;

        let path = self.storage_path();
        tokio::fs::write(&path, content)
            .await
            .map_err(|e| TransferError::ResumeFailed(format!("写入断点信息文件失败: {}", e)))?;

        Ok(())
    }

    /// 保存断点信息
    pub async fn save_resume_info(&self, info: ResumeInfo) -> TransferResult<()> {
        {
            let mut cache = self.resume_infos.write().await;
            cache.insert(info.task_id.clone(), info);
        }
        self.save().await
    }

    /// 获取指定任务的断点信息
    pub async fn get_resume_info(&self, task_id: &str) -> Option<ResumeInfo> {
        let cache = self.resume_infos.read().await;
        cache.get(task_id).and_then(|info| {
            if info.is_expired() {
                None
            } else {
                Some(info.clone())
            }
        })
    }

    /// 获取所有可恢复的任务列表
    pub async fn get_resumable_tasks(&self) -> Vec<ResumableTaskInfo> {
        let cache = self.resume_infos.read().await;
        cache
            .values()
            .filter(|info| !info.is_expired())
            .map(ResumableTaskInfo::from)
            .collect()
    }

    /// 删除指定任务的断点信息
    pub async fn remove_resume_info(&self, task_id: &str) -> TransferResult<()> {
        {
            let mut cache = self.resume_infos.write().await;
            cache.remove(task_id);
        }
        self.save().await
    }

    /// 清理所有过期的断点信息
    #[allow(dead_code)]
    pub async fn cleanup_expired(&self) -> TransferResult<usize> {
        let removed_count;
        {
            let mut cache = self.resume_infos.write().await;
            let before_count = cache.len();
            cache.retain(|_, info| !info.is_expired());
            removed_count = before_count - cache.len();
        }

        if removed_count > 0 {
            self.save().await?;
        }

        Ok(removed_count)
    }

    /// 清理所有断点信息
    pub async fn cleanup_all(&self) -> TransferResult<()> {
        {
            let mut cache = self.resume_infos.write().await;
            cache.clear();
        }
        self.save().await
    }
}

/// 获取默认的断点信息存储目录
pub fn default_resume_storage_dir() -> PathBuf {
    // 使用系统临时目录下的 puresend 子目录
    let mut dir = std::env::temp_dir();
    dir.push("puresend");
    dir.push("resume");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resume_info_expiry() {
        let mut info = ResumeInfo::new(
            "test-task".to_string(),
            "test.txt".to_string(),
            1000,
            "abc123".to_string(),
            500,
            4,
            "192.168.1.1".to_string(),
            8080,
            "send".to_string(),
        );

        // 新创建的断点信息不应过期
        assert!(!info.is_expired());

        // 手动设置为过去的时间
        info.expires_at = 0;
        assert!(info.is_expired());
    }

    #[test]
    fn test_resumable_task_info_from() {
        let info = ResumeInfo::new(
            "test-task".to_string(),
            "test.txt".to_string(),
            1000,
            "abc123".to_string(),
            500,
            4,
            "192.168.1.1".to_string(),
            8080,
            "send".to_string(),
        );

        let resumable: ResumableTaskInfo = (&info).into();
        assert_eq!(resumable.task_id, "test-task");
        assert_eq!(resumable.file_name, "test.txt");
        assert_eq!(resumable.file_size, 1000);
        assert_eq!(resumable.transferred_bytes, 500);
    }

    #[tokio::test]
    async fn test_resume_manager_save_and_load() {
        let temp_dir = std::env::temp_dir().join("puresend_test_resume");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let manager = ResumeManager::new(temp_dir.clone());

        let info = ResumeInfo::new(
            "task-1".to_string(),
            "file.txt".to_string(),
            2000,
            "hash123".to_string(),
            1000,
            9,
            "10.0.0.1".to_string(),
            9090,
            "send".to_string(),
        );

        manager.save_resume_info(info).await.unwrap();

        // 创建新的 manager 并加载
        let manager2 = ResumeManager::new(temp_dir.clone());
        manager2.load().await.unwrap();

        let loaded = manager2.get_resume_info("task-1").await;
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().file_name, "file.txt");

        // 清理
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
