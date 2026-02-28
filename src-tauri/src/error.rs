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
    #[error("IO error: {0}")]
    Io(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("File too large: {0}")]
    FileTooLarge(String),

    #[error("Transfer timeout")]
    Timeout,

    #[error("Transfer cancelled")]
    Cancelled,

    #[error("Integrity check failed: {0}")]
    IntegrityCheckFailed(String),

    #[error("Peer unreachable: {0}")]
    PeerUnreachable(String),

    #[error("Invalid file metadata: {0}")]
    InvalidMetadata(String),

    #[error("Insufficient storage")]
    InsufficientStorage,

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Key exchange failed: {0}")]
    KeyExchange(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Resume failed: {0}")]
    ResumeFailed(String),

    #[error("Resume info expired")]
    ResumeInfoExpired,

    #[error("Chunk verification failed: {0}")]
    ChunkVerificationFailed(String),

    #[error("Protocol version mismatch: {0}")]
    ProtocolVersionMismatch(String),
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
    #[error("mDNS service error: {0}")]
    Mdns(String),

    #[error("No peers found")]
    NoPeersFound,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// 传输结果类型别名
pub type TransferResult<T> = Result<T, TransferError>;

/// 发现结果类型别名
pub type DiscoveryResult<T> = Result<T, DiscoveryError>;
