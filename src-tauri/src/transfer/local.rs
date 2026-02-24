//! 本地网络传输实现
//!
//! 基于 TCP 的本地网络文件传输

use async_trait::async_trait;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::error::{TransferError, TransferResult};
use crate::models::{FileMetadata, TransferMode, TransferProgress, TransferTask};
use crate::transfer::{FileChunker, IntegrityChecker, Transport};

/// 接收配置
#[derive(Debug, Clone, Default)]
pub struct ReceiveConfig {
    /// 是否自动接收（无需确认）
    pub auto_receive: bool,
    /// 是否覆盖同名文件
    pub file_overwrite: bool,
    /// 接收目录
    pub receive_directory: PathBuf,
}

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
    /// 接收配置
    receive_config: Arc<RwLock<Option<ReceiveConfig>>>,
}

/// 传输任务状态
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
            receive_config: Arc::new(RwLock::new(None)),
        }
    }

    /// 使用指定端口创建本地传输实例
    #[allow(dead_code)]
    pub fn with_port(port: u16) -> Self {
        Self {
            listen_port: port,
            chunker: FileChunker::default_chunker(),
            checker: IntegrityChecker::new(),
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            listener: Arc::new(Mutex::new(None)),
            initialized: Arc::new(Mutex::new(false)),
            cancel_senders: Arc::new(RwLock::new(HashMap::new())),
            receive_config: Arc::new(RwLock::new(None)),
        }
    }

    /// 设置接收配置
    pub async fn set_receive_config(&self, config: ReceiveConfig) {
        let mut receive_config = self.receive_config.write().await;
        *receive_config = Some(config);
    }

    /// 获取接收配置
    pub async fn get_receive_config(&self) -> Option<ReceiveConfig> {
        self.receive_config.read().await.clone()
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

                // 获取接收配置
                let config = self.get_receive_config().await;
                let auto_receive = config.as_ref().map(|c| c.auto_receive).unwrap_or(false);
                let file_overwrite = config.as_ref().map(|c| c.file_overwrite).unwrap_or(false);
                let receive_directory = config
                    .as_ref()
                    .map(|c| c.receive_directory.clone())
                    .unwrap_or_else(std::env::temp_dir);

                // 根据 auto_receive 设置决定是否自动接受
                let (accepted, reason) = if auto_receive {
                    (true, None)
                } else {
                    // 非自动接收模式下，拒绝请求，发送方需要等待接收方手动确认
                    // 注意：实际的文件接收确认流程需要通过事件通知前端，
                    // 前端用户确认后再建立新的连接。这里直接拒绝当前请求。
                    (false, Some("需要接收方确认".to_string()))
                };

                // 发送响应
                let response = FileResponse {
                    accepted,
                    reason: reason.clone(),
                };
                let response_json = serde_json::to_vec(&response)?;
                let response_header =
                    MessageHeader::new(MessageType::FileResponse, response_json.len() as u32);
                stream.write_all(&response_header.to_bytes()).await?;
                stream.write_all(&response_json).await?;

                if accepted {
                    // 接收文件分块（使用配置）
                    self.receive_file_chunks_with_config(
                        &mut stream,
                        &metadata,
                        &receive_directory,
                        file_overwrite,
                    )
                    .await?;
                }
            }
            _ => {
                return Err(TransferError::Network("未预期的消息类型".to_string()));
            }
        }

        Ok(())
    }

    /// 接收文件分块（使用指定配置）
    #[allow(dead_code)]
    async fn receive_file_chunks_with_config(
        &self,
        stream: &mut TcpStream,
        metadata: &FileMetadata,
        receive_directory: &PathBuf,
        file_overwrite: bool,
    ) -> TransferResult<()> {
        // 确保接收目录存在
        if !receive_directory.exists() {
            std::fs::create_dir_all(receive_directory).map_err(|e| {
                TransferError::Internal(format!("无法创建接收目录: {}", e))
            })?;
        }

        // 根据 file_overwrite 设置决定保存路径
        let save_path = if file_overwrite {
            receive_directory.join(&metadata.name)
        } else {
            // 生成唯一文件名避免覆盖
            self.get_unique_file_path(receive_directory, &metadata.name)?
        };

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

    /// 生成不冲突的文件路径
    fn get_unique_file_path(
        &self,
        directory: &PathBuf,
        original_name: &str,
    ) -> TransferResult<PathBuf> {
        let path = directory.join(original_name);

        // 如果文件不存在，直接使用原文件名
        if !path.exists() {
            return Ok(path);
        }

        // 解析文件名和扩展名
        let (stem, extension) = Self::parse_filename(original_name);

        // 尝试找到可用的文件名
        let mut counter = 1u32;
        loop {
            let new_name = if extension.is_empty() {
                format!("{} ({})", stem, counter)
            } else {
                format!("{} ({}).{}", stem, counter, extension)
            };

            let new_path = directory.join(&new_name);
            if !new_path.exists() {
                return Ok(new_path);
            }

            counter += 1;

            // 防止无限循环（最多尝试 10000 次）
            if counter > 10000 {
                return Err(TransferError::Internal(format!(
                    "无法生成唯一文件名：{}",
                    original_name
                )));
            }
        }
    }

    /// 解析文件名为（主文件名，扩展名）
    fn parse_filename(filename: &str) -> (String, String) {
        // 特殊情况：以点开头的隐藏文件（只有一个点）
        if filename.starts_with('.') && filename.matches('.').count() == 1 {
            return (filename.to_string(), String::new());
        }

        // 查找最后一个点
        if let Some(dot_pos) = filename.rfind('.') {
            let stem = &filename[..dot_pos];
            let ext = &filename[dot_pos + 1..];

            // 检查是否为复合扩展名（如 .tar.gz）
            if let Some(inner_dot) = stem.rfind('.') {
                let inner_ext = &stem[inner_dot + 1..];
                // 常见复合扩展名
                const COMPOUND_EXTENSIONS: &[&str] = &["tar", "zip"];
                if COMPOUND_EXTENSIONS.contains(&inner_ext) {
                    return (
                        stem[..inner_dot].to_string(),
                        format!("{}.{}", inner_ext, ext),
                    );
                }
            }

            (stem.to_string(), ext.to_string())
        } else {
            (filename.to_string(), String::new())
        }
    }
}

/// 文件传输请求响应

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
