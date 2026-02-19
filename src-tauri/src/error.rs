//! 错误处理模块
//!
//! 定义文件传输过程中可能出现的所有错误类型

use serde::Serialize;
use std::io;
use thiserror::Error;

/// 传输错误类型
#[derive(Debug, Error, Serialize)]
#[allow(dead_code)]
pub enum TransferError {
    #[error("IO错误: {0}")]
    Io(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("文件不存在: {0}")]
    FileNotFound(String),

    #[error("文件过大: {0}")]
    FileTooLarge(String),

    #[error("传输超时")]
    Timeout,

    #[error("传输已取消")]
    Cancelled,

    #[error("校验失败: {0}")]
    IntegrityCheckFailed(String),

    #[error("设备不可达: {0}")]
    PeerUnreachable(String),

    #[error("无效的文件元数据: {0}")]
    InvalidMetadata(String),

    #[error("存储空间不足")]
    InsufficientStorage,

    #[error("不支持的操作: {0}")]
    UnsupportedOperation(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

impl From<io::Error> for TransferError {
    fn from(err: io::Error) -> Self {
        TransferError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for TransferError {
    fn from(err: serde_json::Error) -> Self {
        TransferError::Internal(err.to_string())
    }
}

/// 设备发现错误类型
#[derive(Debug, Error, Serialize)]
#[allow(dead_code)]
pub enum DiscoveryError {
    #[error("mDNS服务错误: {0}")]
    Mdns(String),

    #[error("没有发现设备")]
    NoPeersFound,

    #[error("设备连接失败: {0}")]
    ConnectionFailed(String),

    #[error("握手失败: {0}")]
    HandshakeFailed(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

/// 传输结果类型别名
pub type TransferResult<T> = Result<T, TransferError>;

/// 发现结果类型别名
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;
