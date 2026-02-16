//! 传输层抽象接口
//! 
//! 定义所有传输后端必须实现的 Trait

use async_trait::async_trait;
use crate::error::TransferResult;
use crate::models::{TransferTask, TransferProgress};

/// 传输后端抽象接口
/// 
/// 所有传输实现（本地、云盘等）都需要实现此接口
#[async_trait]
pub trait Transport: Send + Sync {
    /// 初始化传输通道
    /// 
    /// 在开始传输前调用，用于建立连接、分配资源等
    async fn initialize(&self) -> TransferResult<()>;

    /// 发送文件
    /// 
    /// # Arguments
    /// * `task` - 传输任务信息
    /// 
    /// # Returns
    /// * `TransferResult<TransferProgress>` - 传输进度结果
    async fn send(&self, task: &TransferTask) -> TransferResult<TransferProgress>;

    /// 接收文件
    /// 
    /// # Arguments
    /// * `task` - 传输任务信息
    /// 
    /// # Returns
    /// * `TransferResult<TransferProgress>` - 传输进度结果
    async fn receive(&self, task: &TransferTask) -> TransferResult<TransferProgress>;

    /// 取消传输
    /// 
    /// # Arguments
    /// * `task_id` - 要取消的任务 ID
    async fn cancel(&self, task_id: &str) -> TransferResult<()>;

    /// 获取传输进度
    /// 
    /// # Arguments
    /// * `task_id` - 任务 ID
    /// 
    /// # Returns
    /// * `TransferResult<TransferProgress>` - 当前传输进度
    async fn progress(&self, task_id: &str) -> TransferResult<TransferProgress>;

    /// 关闭传输通道
    /// 
    /// 释放资源、关闭连接等
    async fn shutdown(&self) -> TransferResult<()>;

    /// 获取传输模式名称
    fn mode(&self) -> &'static str;
}

/// 传输工厂 Trait
/// 
/// 用于创建不同类型的传输实例
pub trait TransportFactory: Send + Sync {
    /// 创建传输实例
    fn create(&self) -> Box<dyn Transport>;
    
    /// 获取支持的传输模式
    fn mode(&self) -> &'static str;
}