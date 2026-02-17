//! 本地网络传输实现
//!
//! 基于 TCP 的本地网络文件传输

use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::error::{TransferError, TransferResult};
use crate::models::{FileMetadata, TransferMode, TransferProgress, TransferTask};
use crate::transfer::{FileChunker, IntegrityChecker, Transport};

/// 传输协议魔数
const PROTOCOL_MAGIC: &[u8; 4] = b"PSEN";

/// 协议版本
const PROTOCOL_VERSION: u8 = 1;

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum MessageType {
    /// 文件传输请求
    FileRequest = 0x01,
    /// 文件传输响应
    FileResponse = 0x02,
    /// 分块数据
    ChunkData = 0x03,
    /// 分块确认
    ChunkAck = 0x04,
    /// 传输取消
    Cancel = 0x05,
    /// 心跳
    Heartbeat = 0x06,
    /// 错误
    Error = 0x07,
}

/// 消息头
#[derive(Debug)]
struct MessageHeader {
    message_type: MessageType,
    payload_length: u32,
}

impl MessageHeader {
    fn new(message_type: MessageType, payload_length: u32) -> Self {
        Self {
            message_type,
            payload_length,
        }
    }

    fn to_bytes(&self) -> [u8; 8] {
        let mut buf = [0u8; 8];
        buf[0..4].copy_from_slice(PROTOCOL_MAGIC);
        buf[4] = PROTOCOL_VERSION;
        buf[5] = self.message_type as u8;
        buf[6..8].copy_from_slice(&(self.payload_length as u16).to_be_bytes());
        buf
    }

    fn from_bytes(bytes: &[u8; 8]) -> TransferResult<Self> {
        if &bytes[0..4] != PROTOCOL_MAGIC {
            return Err(TransferError::Network("无效的协议魔数".to_string()));
        }

        let version = bytes[4];
        if version != PROTOCOL_VERSION {
            return Err(TransferError::Network(format!(
                "不支持的协议版本: {}",
                version
            )));
        }

        let message_type = match bytes[5] {
            0x01 => MessageType::FileRequest,
            0x02 => MessageType::FileResponse,
            0x03 => MessageType::ChunkData,
            0x04 => MessageType::ChunkAck,
            0x05 => MessageType::Cancel,
            0x06 => MessageType::Heartbeat,
            0x07 => MessageType::Error,
            _ => return Err(TransferError::Network("未知的消息类型".to_string())),
        };

        let payload_length = u16::from_be_bytes([bytes[6], bytes[7]]) as u32;

        Ok(Self {
            message_type,
            payload_length,
        })
    }
}

/// 本地传输实现
#[allow(dead_code)]
pub struct LocalTransport {
    /// 监听端口
    listen_port: u16,
    /// 分块器
    chunker: FileChunker,
    /// 校验器
    checker: IntegrityChecker,
    /// 活跃传输任务
    active_tasks: Arc<RwLock<HashMap<String, TransferTaskState>>>,
    /// TCP 监听器
    listener: Arc<Mutex<Option<TcpListener>>>,
    /// 是否已初始化
    initialized: Arc<Mutex<bool>>,
    /// 取消信号发送器
    cancel_senders: Arc<RwLock<HashMap<String, mpsc::Sender<()>>>>,
}

/// 传输任务状态
#[derive(Debug, Clone)]
struct TransferTaskState {
    /// 进度
    progress: TransferProgress,
    /// 是否已取消
    cancelled: bool,
}

impl LocalTransport {
    /// 创建新的本地传输实例
    pub fn new() -> Self {
        Self {
            listen_port: 0, // 自动分配端口
            chunker: FileChunker::default_chunker(),
            checker: IntegrityChecker::new(),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            listener: Arc::new(Mutex::new(None)),
            initialized: Arc::new(Mutex::new(false)),
            cancel_senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 使用指定端口创建本地传输实例
    pub fn with_port(port: u16) -> Self {
        Self {
            listen_port: port,
            chunker: FileChunker::default_chunker(),
            checker: IntegrityChecker::new(),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            listener: Arc::new(Mutex::new(None)),
            initialized: Arc::new(Mutex::new(false)),
            cancel_senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取监听端口
    pub async fn get_listen_port(&self) -> TransferResult<u16> {
        let listener = self.listener.lock().await;
        if let Some(ref listener) = *listener {
            Ok(listener.local_addr()?.port())
        } else {
            Err(TransferError::Internal("传输未初始化".to_string()))
        }
    }

    /// 获取监听器（用于接收文件）
    #[allow(dead_code)]
    pub async fn get_listener(&self) -> TransferResult<Arc<Mutex<Option<TcpListener>>>> {
        Ok(self.listener.clone())
    }

    /// 发送文件到指定地址
    async fn send_file_to(
        &self,
        task: &TransferTask,
        addr: SocketAddr,
    ) -> TransferResult<TransferProgress> {
        let file_path = task
            .file
            .path
            .as_ref()
            .ok_or_else(|| TransferError::InvalidMetadata("文件路径未设置".to_string()))?;

        let file_path = std::path::Path::new(file_path);
        if !file_path.exists() {
            return Err(TransferError::FileNotFound(file_path.display().to_string()));
        }

        // 创建取消通道
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        self.cancel_senders
            .write()
            .await
            .insert(task.id.clone(), cancel_tx);

        // 连接目标
        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| TransferError::Network(format!("连接失败: {}", e)))?;

        // 发送文件请求
        let metadata_json = serde_json::to_string(&task.file)?;
        let header = MessageHeader::new(MessageType::FileRequest, metadata_json.len() as u32);
        stream.write_all(&header.to_bytes()).await?;
        stream.write_all(metadata_json.as_bytes()).await?;

        // 等待响应
        let mut response_header_buf = [0u8; 8];
        stream.read_exact(&mut response_header_buf).await?;
        let response_header = MessageHeader::from_bytes(&response_header_buf)?;

        if response_header.message_type != MessageType::FileResponse {
            return Err(TransferError::Network("未收到正确的文件响应".to_string()));
        }

        let mut response_buf = vec![0u8; response_header.payload_length as usize];
        stream.read_exact(&mut response_buf).await?;
        let response: FileResponse = serde_json::from_slice(&response_buf)?;

        if !response.accepted {
            return Err(TransferError::Network(format!(
                "对方拒绝接收: {}",
                response.reason.unwrap_or_default()
            )));
        }

        // 发送文件分块
        let chunks = self.chunker.compute_chunks(file_path)?;
        let mut task_state = TransferTaskState {
            progress: TransferProgress::from(task),
            cancelled: false,
        };
        task_state.progress.status = crate::models::TaskStatus::Transferring;

        let start_time = std::time::Instant::now();
        let mut total_transferred: u64 = 0;

        for chunk in &chunks {
            // 检查取消信号
            if cancel_rx.try_recv().is_ok() {
                task_state.progress.status = crate::models::TaskStatus::Cancelled;
                self.active_tasks
                    .write()
                    .await
                    .insert(task.id.clone(), task_state);
                return Err(TransferError::Cancelled);
            }

            // 读取分块数据
            let data = self.chunker.read_chunk(file_path, chunk)?;

            // 发送分块
            let chunk_message = ChunkMessage {
                index: chunk.index,
                data: data.clone(),
            };
            let chunk_json = serde_json::to_vec(&chunk_message)?;
            let header = MessageHeader::new(MessageType::ChunkData, chunk_json.len() as u32);
            stream.write_all(&header.to_bytes()).await?;
            stream.write_all(&chunk_json).await?;

            // 等待确认
            let mut ack_header_buf = [0u8; 8];
            tokio::select! {
                result = stream.read_exact(&mut ack_header_buf) => {
                    result?;
                }
                _ = cancel_rx.recv() => {
                    task_state.progress.status = crate::models::TaskStatus::Cancelled;
                    self.active_tasks.write().await.insert(task.id.clone(), task_state);
                    return Err(TransferError::Cancelled);
                }
            }

            total_transferred += chunk.size;
            let elapsed = start_time.elapsed().as_secs_f64();
            let speed = if elapsed > 0.0 {
                (total_transferred as f64 / elapsed) as u64
            } else {
                0
            };

            task_state.progress.transferred_bytes = total_transferred;
            task_state.progress.speed = speed;
            task_state.progress.progress =
                (total_transferred as f64 / task.file.size as f64) * 100.0;

            // 更新活跃任务状态
            self.active_tasks
                .write()
                .await
                .insert(task.id.clone(), task_state.clone());
        }

        // 传输完成
        task_state.progress.status = crate::models::TaskStatus::Completed;
        task_state.progress.progress = 100.0;
        self.active_tasks
            .write()
            .await
            .insert(task.id.clone(), task_state.clone());

        Ok(task_state.progress)
    }

    /// 处理接收连接
    #[allow(dead_code)]
    async fn handle_connection(&self, mut stream: TcpStream) -> TransferResult<()> {
        // 读取消息头
        let mut header_buf = [0u8; 8];
        stream.read_exact(&mut header_buf).await?;
        let header = MessageHeader::from_bytes(&header_buf)?;

        match header.message_type {
            MessageType::FileRequest => {
                // 读取文件元数据
                let mut metadata_buf = vec![0u8; header.payload_length as usize];
                stream.read_exact(&mut metadata_buf).await?;
                let metadata: FileMetadata = serde_json::from_slice(&metadata_buf)?;

                // 发送响应（这里简化处理，总是接受）
                let response = FileResponse {
                    accepted: true,
                    reason: None,
                };
                let response_json = serde_json::to_vec(&response)?;
                let response_header =
                    MessageHeader::new(MessageType::FileResponse, response_json.len() as u32);
                stream.write_all(&response_header.to_bytes()).await?;
                stream.write_all(&response_json).await?;

                // 接收文件分块
                self.receive_file_chunks(&mut stream, &metadata).await?;
            }
            _ => {
                return Err(TransferError::Network("未预期的消息类型".to_string()));
            }
        }

        Ok(())
    }

    /// 接收文件分块
    #[allow(dead_code)]
    async fn receive_file_chunks(
        &self,
        stream: &mut TcpStream,
        metadata: &FileMetadata,
    ) -> TransferResult<()> {
        let save_path = std::env::temp_dir().join(&metadata.name);

        for _ in 0..metadata.chunks.len() {
            // 读取分块消息头
            let mut header_buf = [0u8; 8];
            stream.read_exact(&mut header_buf).await?;
            let header = MessageHeader::from_bytes(&header_buf)?;

            if header.message_type != MessageType::ChunkData {
                return Err(TransferError::Network("期望分块数据".to_string()));
            }

            // 读取分块数据
            let mut chunk_buf = vec![0u8; header.payload_length as usize];
            stream.read_exact(&mut chunk_buf).await?;
            let chunk_message: ChunkMessage = serde_json::from_slice(&chunk_buf)?;

            // 写入文件
            let chunk_info = &metadata.chunks[chunk_message.index as usize];
            self.chunker
                .write_chunk(&save_path, chunk_info, &chunk_message.data)?;

            // 发送确认
            let ack = ChunkAck {
                index: chunk_message.index,
                success: true,
            };
            let ack_json = serde_json::to_vec(&ack)?;
            let ack_header = MessageHeader::new(MessageType::ChunkAck, ack_json.len() as u32);
            stream.write_all(&ack_header.to_bytes()).await?;
            stream.write_all(&ack_json).await?;
        }

        // 验证文件
        if !self.checker.verify_file(&save_path, &metadata.hash)? {
            return Err(TransferError::IntegrityCheckFailed(
                "文件校验失败".to_string(),
            ));
        }

        Ok(())
    }
}

/// 文件传输请求响应
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct FileResponse {
    /// 是否接受
    accepted: bool,
    /// 拒绝原因
    reason: Option<String>,
}

/// 分块消息
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ChunkMessage {
    /// 分块索引
    index: u32,
    /// 分块数据
    data: Vec<u8>,
}

/// 分块确认
#[allow(dead_code)]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ChunkAck {
    /// 分块索引
    index: u32,
    /// 是否成功
    success: bool,
}

#[async_trait]
impl Transport for LocalTransport {
    async fn initialize(&self) -> TransferResult<()> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Ok(());
        }

        // 创建 TCP 监听器
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.listen_port)).await?;

        let mut listener_guard = self.listener.lock().await;
        *listener_guard = Some(listener);

        *initialized = true;
        Ok(())
    }

    async fn send(&self, task: &TransferTask) -> TransferResult<TransferProgress> {
        if task.mode != TransferMode::Local {
            return Err(TransferError::UnsupportedOperation(
                "仅支持本地网络传输".to_string(),
            ));
        }

        let peer = task
            .peer
            .as_ref()
            .ok_or_else(|| TransferError::PeerUnreachable("未指定目标设备".to_string()))?;

        let addr: SocketAddr = format!("{}:{}", peer.ip, peer.port)
            .parse()
            .map_err(|e| TransferError::PeerUnreachable(format!("无效的地址: {}", e)))?;

        self.send_file_to(task, addr).await
    }

    async fn receive(&self, _task: &TransferTask) -> TransferResult<TransferProgress> {
        // 接收逻辑在 handle_connection 中处理
        Err(TransferError::UnsupportedOperation(
            "请使用监听模式接收文件".to_string(),
        ))
    }

    async fn cancel(&self, task_id: &str) -> TransferResult<()> {
        if let Some(sender) = self.cancel_senders.write().await.remove(task_id) {
            let _ = sender.send(()).await;
        }
        if let Some(task_state) = self.active_tasks.write().await.get_mut(task_id) {
            task_state.cancelled = true;
            task_state.progress.status = crate::models::TaskStatus::Cancelled;
        }
        Ok(())
    }

    async fn progress(&self, task_id: &str) -> TransferResult<TransferProgress> {
        let tasks = self.active_tasks.read().await;
        tasks
            .get(task_id)
            .map(|state| state.progress.clone())
            .ok_or_else(|| TransferError::Internal(format!("任务不存在: {}", task_id)))
    }

    async fn shutdown(&self) -> TransferResult<()> {
        // 清理资源
        self.active_tasks.write().await.clear();
        self.cancel_senders.write().await.clear();
        *self.listener.lock().await = None;
        *self.initialized.lock().await = false;
        Ok(())
    }

    fn mode(&self) -> &'static str {
        "local"
    }
}

impl Default for LocalTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_header() {
        let header = MessageHeader::new(MessageType::FileRequest, 100);
        let bytes = header.to_bytes();
        let parsed = MessageHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.message_type, MessageType::FileRequest);
        assert_eq!(parsed.payload_length, 100);
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(b"XXXX");
        let result = MessageHeader::from_bytes(&bytes);
        assert!(result.is_err());
    }
}
