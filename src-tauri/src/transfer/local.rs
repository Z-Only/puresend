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
const PROTOCOL_VERSION: u8 = 2;

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
    /// 握手请求（v2）
    Handshake = 0x08,
    /// 握手响应（v2）
    HandshakeAck = 0x09,
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

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(10);
        buf.extend_from_slice(PROTOCOL_MAGIC);
        buf.push(PROTOCOL_VERSION);
        buf.push(self.message_type as u8);
        buf.extend_from_slice(&self.payload_length.to_be_bytes());
        buf
    }

    /// 从 TCP 流中读取消息头（自动检测 v1/v2 版本）
    async fn read_from_stream(stream: &mut TcpStream) -> TransferResult<Self> {
        // 先读取 6 字节公共部分：magic(4) + version(1) + type(1)
        let mut common_buf = [0u8; 6];
        stream.read_exact(&mut common_buf).await?;

        if &common_buf[0..4] != PROTOCOL_MAGIC {
            return Err(TransferError::Network("无效的协议魔数".to_string()));
        }

        let version = common_buf[4];
        let message_type = match common_buf[5] {
            0x01 => MessageType::FileRequest,
            0x02 => MessageType::FileResponse,
            0x03 => MessageType::ChunkData,
            0x04 => MessageType::ChunkAck,
            0x05 => MessageType::Cancel,
            0x06 => MessageType::Heartbeat,
            0x07 => MessageType::Error,
            0x08 => MessageType::Handshake,
            0x09 => MessageType::HandshakeAck,
            _ => return Err(TransferError::Network("未知的消息类型".to_string())),
        };

        let payload_length = if version >= 2 {
            // v2: 4 字节 payload_length
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).await?;
            u32::from_be_bytes(len_buf)
        } else {
            // v1: 2 字节 payload_length
            let mut len_buf = [0u8; 2];
            stream.read_exact(&mut len_buf).await?;
            u16::from_be_bytes(len_buf) as u32
        };

        Ok(Self {
            message_type,
            payload_length,
        })
    }

    #[allow(dead_code)]
    fn from_bytes(bytes: &[u8]) -> TransferResult<Self> {
        if bytes.len() < 6 {
            return Err(TransferError::Network("消息头数据不足".to_string()));
        }
        if &bytes[0..4] != PROTOCOL_MAGIC {
            return Err(TransferError::Network("无效的协议魔数".to_string()));
        }

        let version = bytes[4];
        let message_type = match bytes[5] {
            0x01 => MessageType::FileRequest,
            0x02 => MessageType::FileResponse,
            0x03 => MessageType::ChunkData,
            0x04 => MessageType::ChunkAck,
            0x05 => MessageType::Cancel,
            0x06 => MessageType::Heartbeat,
            0x07 => MessageType::Error,
            0x08 => MessageType::Handshake,
            0x09 => MessageType::HandshakeAck,
            _ => return Err(TransferError::Network("未知的消息类型".to_string())),
        };

        let payload_length = if version >= 2 {
            if bytes.len() < 10 {
                return Err(TransferError::Network("v2 消息头数据不足".to_string()));
            }
            u32::from_be_bytes([bytes[6], bytes[7], bytes[8], bytes[9]])
        } else {
            if bytes.len() < 8 {
                return Err(TransferError::Network("v1 消息头数据不足".to_string()));
            }
            u16::from_be_bytes([bytes[6], bytes[7]]) as u32
        };

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
    ///
    /// 传输流程：连接 → 握手协商（v2） → 文件请求/响应 → 分块传输（可选加密+压缩） → 完成
    /// 支持断点续传：传输中断时保存断点信息，恢复时跳过已传输的分块
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

        // === 阶段 1：握手协商（v2 特性协商） ===
        let encryption_enabled = crate::transfer::crypto::is_encryption_enabled();
        let compression_config = crate::transfer::compression::get_compression_config();

        // 创建密钥交换发起方（如果启用加密）
        let key_exchange_initiator = if encryption_enabled {
            Some(crate::transfer::crypto::KeyExchangeInitiator::new())
        } else {
            None
        };

        let handshake = HandshakePayload {
            protocol_version: PROTOCOL_VERSION,
            supports_encryption: encryption_enabled,
            supports_compression: compression_config.enabled,
            supports_resume: true,
            public_key: key_exchange_initiator
                .as_ref()
                .map(|k| k.public_key_bytes()),
        };

        let handshake_json = serde_json::to_vec(&handshake)?;
        let handshake_header =
            MessageHeader::new(MessageType::Handshake, handshake_json.len() as u32);
        stream.write_all(&handshake_header.to_bytes()).await?;
        stream.write_all(&handshake_json).await?;

        // 等待握手响应
        let ack_header = MessageHeader::read_from_stream(&mut stream).await?;
        if ack_header.message_type != MessageType::HandshakeAck {
            return Err(TransferError::Network("未收到握手响应".to_string()));
        }

        let mut ack_buf = vec![0u8; ack_header.payload_length as usize];
        stream.read_exact(&mut ack_buf).await?;
        let handshake_ack: HandshakeAckPayload = serde_json::from_slice(&ack_buf)?;

        // 协商最终特性
        let negotiated = NegotiatedFeatures {
            encryption: handshake.supports_encryption && handshake_ack.use_encryption,
            compression: handshake.supports_compression && handshake_ack.use_compression,
            resume: handshake_ack.use_resume,
        };

        // 完成密钥交换（如果双方都同意加密）
        let mut crypto_session = if negotiated.encryption {
            let initiator = key_exchange_initiator.ok_or_else(|| {
                TransferError::KeyExchange("加密已协商但密钥交换发起方缺失".to_string())
            })?;
            let peer_public_key = handshake_ack.public_key.ok_or_else(|| {
                TransferError::KeyExchange("对方未提供加密公钥".to_string())
            })?;
            Some(initiator.complete(&peer_public_key)?)
        } else {
            None
        };

        // 创建压缩器（如果双方都同意压缩）
        let compressor = if negotiated.compression {
            crate::transfer::compression::create_compressor_from_config()
        } else {
            None
        };

        // === 阶段 2：文件请求/响应 ===
        let metadata_json = serde_json::to_string(&task.file)?;
        let header = MessageHeader::new(MessageType::FileRequest, metadata_json.len() as u32);
        stream.write_all(&header.to_bytes()).await?;
        stream.write_all(metadata_json.as_bytes()).await?;

        // 等待响应
        let response_header = MessageHeader::read_from_stream(&mut stream).await?;

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

        // === 阶段 3：检查断点续传信息 ===
        let resume_manager = crate::transfer::resume::ResumeManager::new(
            crate::transfer::resume::default_resume_storage_dir(),
        );
        let _ = resume_manager.load().await;

        let resume_from_chunk: u32 = if negotiated.resume {
            if let Some(resume_info) = resume_manager.get_resume_info(&task.id).await {
                if resume_info.file_name == task.file.name
                    && resume_info.file_size == task.file.size
                {
                    resume_info.last_chunk_index + 1
                } else {
                    0
                }
            } else {
                0
            }
        } else {
            0
        };

        // === 阶段 4：分块传输 ===
        let chunks = self.chunker.compute_chunks(file_path)?;
        let mut task_state = TransferTaskState {
            progress: TransferProgress::from(task),
            cancelled: false,
        };
        task_state.progress.status = crate::models::TaskStatus::Transferring;

        let start_time = std::time::Instant::now();
        // 断点续传时，已传输的字节数从断点处开始计算
        let mut total_transferred: u64 = chunks
            .iter()
            .filter(|c| c.index < resume_from_chunk)
            .map(|c| c.size)
            .sum();
        let mut last_successful_chunk_index: u32 = if resume_from_chunk > 0 {
            resume_from_chunk - 1
        } else {
            0
        };

        let mime_type = &task.file.mime_type;

        for chunk in &chunks {
            // 跳过已传输的分块（断点续传）
            if chunk.index < resume_from_chunk {
                continue;
            }

            // 检查取消信号
            if cancel_rx.try_recv().is_ok() {
                // 保存断点信息
                self.save_resume_info_on_interrupt(
                    &resume_manager,
                    task,
                    last_successful_chunk_index,
                    total_transferred,
                    &addr,
                    "send",
                )
                .await;

                task_state.progress.status = crate::models::TaskStatus::Cancelled;
                self.active_tasks
                    .write()
                    .await
                    .insert(task.id.clone(), task_state);
                return Err(TransferError::Cancelled);
            }

            // 读取分块数据
            let raw_data = self.chunker.read_chunk(file_path, chunk)?;

            // 可选压缩
            let (chunk_data, is_compressed) =
                if let Some(ref comp) = compressor {
                    if let Some(level) = comp.get_level(mime_type) {
                        let compressed = crate::transfer::compression::Compressor::compress(
                            &raw_data, level,
                        )?;
                        // 仅当压缩后更小时才使用压缩数据
                        if compressed.len() < raw_data.len() {
                            (compressed, true)
                        } else {
                            (raw_data, false)
                        }
                    } else {
                        (raw_data, false)
                    }
                } else {
                    (raw_data, false)
                };

            // 可选加密
            let final_data = if let Some(ref mut session) = crypto_session {
                session.encrypt(&chunk_data)?
            } else {
                chunk_data
            };

            // 发送分块
            let chunk_message = ChunkMessage {
                index: chunk.index,
                data: final_data,
                compressed: is_compressed,
            };
            let chunk_json = serde_json::to_vec(&chunk_message)?;
            let header = MessageHeader::new(MessageType::ChunkData, chunk_json.len() as u32);

            let send_result = async {
                stream.write_all(&header.to_bytes()).await?;
                stream.write_all(&chunk_json).await?;
                Ok::<(), std::io::Error>(())
            }
            .await;

            if let Err(send_err) = send_result {
                // 网络错误，保存断点信息
                self.save_resume_info_on_interrupt(
                    &resume_manager,
                    task,
                    last_successful_chunk_index,
                    total_transferred,
                    &addr,
                    "send",
                )
                .await;

                task_state.progress.status = crate::models::TaskStatus::Interrupted;
                self.active_tasks
                    .write()
                    .await
                    .insert(task.id.clone(), task_state);
                return Err(TransferError::Network(format!("发送数据失败: {}", send_err)));
            }

            // 等待确认
            let ack_result = tokio::select! {
                result = MessageHeader::read_from_stream(&mut stream) => {
                    result
                }
                _ = cancel_rx.recv() => {
                    // 取消时保存断点信息
                    self.save_resume_info_on_interrupt(
                        &resume_manager,
                        task,
                        last_successful_chunk_index,
                        total_transferred,
                        &addr,
                        "send",
                    ).await;

                    task_state.progress.status = crate::models::TaskStatus::Cancelled;
                    self.active_tasks.write().await.insert(task.id.clone(), task_state);
                    return Err(TransferError::Cancelled);
                }
            };

            if let Err(ack_err) = ack_result {
                // 等待确认时网络错误，保存断点信息
                self.save_resume_info_on_interrupt(
                    &resume_manager,
                    task,
                    last_successful_chunk_index,
                    total_transferred,
                    &addr,
                    "send",
                )
                .await;

                task_state.progress.status = crate::models::TaskStatus::Interrupted;
                self.active_tasks
                    .write()
                    .await
                    .insert(task.id.clone(), task_state);
                return Err(ack_err);
            }

            last_successful_chunk_index = chunk.index;
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

        // 传输完成，清理断点信息
        let _ = resume_manager.remove_resume_info(&task.id).await;

        task_state.progress.status = crate::models::TaskStatus::Completed;
        task_state.progress.progress = 100.0;
        self.active_tasks
            .write()
            .await
            .insert(task.id.clone(), task_state.clone());

        Ok(task_state.progress)
    }

    /// 传输中断时保存断点信息
    async fn save_resume_info_on_interrupt(
        &self,
        resume_manager: &crate::transfer::resume::ResumeManager,
        task: &TransferTask,
        last_chunk_index: u32,
        transferred_bytes: u64,
        addr: &SocketAddr,
        direction: &str,
    ) {
        let resume_info = crate::transfer::resume::ResumeInfo::new(
            task.id.clone(),
            task.file.name.clone(),
            task.file.size,
            task.file.hash.clone(),
            transferred_bytes,
            last_chunk_index,
            addr.ip().to_string(),
            addr.port(),
            direction.to_string(),
        );
        let _ = resume_manager.save_resume_info(resume_info).await;
    }

    /// 处理接收连接
    ///
    /// 接收流程：握手协商（v2） → 文件请求/响应 → 分块接收（可选解密+解压） → 完成
    #[allow(dead_code)]
    async fn handle_connection(&self, mut stream: TcpStream) -> TransferResult<()> {
        // 读取第一条消息头
        let header = MessageHeader::read_from_stream(&mut stream).await?;

        // === 阶段 1：握手协商（v2） ===
        let (crypto_session, negotiated) = if header.message_type == MessageType::Handshake {
            // 读取握手载荷
            let mut handshake_buf = vec![0u8; header.payload_length as usize];
            stream.read_exact(&mut handshake_buf).await?;
            let handshake: HandshakePayload = serde_json::from_slice(&handshake_buf)?;

            // 接收方根据自身配置和对方能力决定是否启用特性
            let local_encryption_enabled = crate::transfer::crypto::is_encryption_enabled();
            let local_compression_config = crate::transfer::compression::get_compression_config();

            let use_encryption = handshake.supports_encryption && local_encryption_enabled;
            let use_compression = handshake.supports_compression && local_compression_config.enabled;
            let use_resume = handshake.supports_resume;

            // 创建密钥交换响应方（如果双方都同意加密）
            let key_exchange_responder = if use_encryption {
                Some(crate::transfer::crypto::KeyExchangeResponder::new())
            } else {
                None
            };

            // 发送握手响应
            let ack = HandshakeAckPayload {
                protocol_version: PROTOCOL_VERSION,
                use_encryption,
                use_compression,
                use_resume,
                public_key: key_exchange_responder
                    .as_ref()
                    .map(|r| r.public_key_bytes()),
            };

            let ack_json = serde_json::to_vec(&ack)?;
            let ack_header =
                MessageHeader::new(MessageType::HandshakeAck, ack_json.len() as u32);
            stream.write_all(&ack_header.to_bytes()).await?;
            stream.write_all(&ack_json).await?;

            // 完成密钥交换
            let session = if use_encryption {
                let responder = key_exchange_responder.ok_or_else(|| {
                    TransferError::KeyExchange("加密已协商但密钥交换响应方缺失".to_string())
                })?;
                let peer_public_key = handshake.public_key.ok_or_else(|| {
                    TransferError::KeyExchange("对方未提供加密公钥".to_string())
                })?;
                Some(responder.complete(&peer_public_key)?)
            } else {
                None
            };

            let features = NegotiatedFeatures {
                encryption: use_encryption,
                compression: use_compression,
                resume: use_resume,
            };

            // 读取下一条消息（应该是 FileRequest）
            let next_header = MessageHeader::read_from_stream(&mut stream).await?;
            if next_header.message_type != MessageType::FileRequest {
                return Err(TransferError::Network(
                    "握手后期望收到文件请求".to_string(),
                ));
            }

            // 读取文件元数据
            let mut metadata_buf = vec![0u8; next_header.payload_length as usize];
            stream.read_exact(&mut metadata_buf).await?;
            let metadata: FileMetadata = serde_json::from_slice(&metadata_buf)?;

            self.handle_file_request_with_features(
                &mut stream,
                metadata,
                session,
                features,
            )
            .await?;

            return Ok(());
        } else if header.message_type == MessageType::FileRequest {
            // v1 兼容：没有握手，直接处理文件请求
            (None, NegotiatedFeatures::default())
        } else {
            return Err(TransferError::Network("未预期的消息类型".to_string()));
        };

        // v1 兼容路径：直接处理文件请求（无加密/压缩）
        let mut metadata_buf = vec![0u8; header.payload_length as usize];
        stream.read_exact(&mut metadata_buf).await?;
        let metadata: FileMetadata = serde_json::from_slice(&metadata_buf)?;

        self.handle_file_request_with_features(
            &mut stream,
            metadata,
            crypto_session,
            negotiated,
        )
        .await
    }

    /// 处理文件请求（带特性协商结果）
    #[allow(dead_code)]
    async fn handle_file_request_with_features(
        &self,
        stream: &mut TcpStream,
        metadata: FileMetadata,
        crypto_session: Option<crate::transfer::crypto::CryptoSession>,
        negotiated: NegotiatedFeatures,
    ) -> TransferResult<()> {
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
            self.receive_file_chunks_with_features(
                stream,
                &metadata,
                &receive_directory,
                file_overwrite,
                crypto_session,
                &negotiated,
            )
            .await?;
        }

        Ok(())
    }

    /// 接收文件分块（带加密/压缩/断点续传支持）
    #[allow(dead_code)]
    async fn receive_file_chunks_with_features(
        &self,
        stream: &mut TcpStream,
        metadata: &FileMetadata,
        receive_directory: &PathBuf,
        file_overwrite: bool,
        crypto_session: Option<crate::transfer::crypto::CryptoSession>,
        negotiated: &NegotiatedFeatures,
    ) -> TransferResult<()> {
        // 确保接收目录存在
        if !receive_directory.exists() {
            std::fs::create_dir_all(receive_directory)
                .map_err(|e| TransferError::Internal(format!("无法创建接收目录: {}", e)))?;
        }

        // 根据 file_overwrite 设置决定保存路径
        let save_path = if file_overwrite {
            receive_directory.join(&metadata.name)
        } else {
            self.get_unique_file_path(receive_directory, &metadata.name)?
        };

        let peer_addr = stream
            .peer_addr()
            .map(|a| a.to_string())
            .unwrap_or_default();

        // 断点续传管理器
        let resume_manager = crate::transfer::resume::ResumeManager::new(
            crate::transfer::resume::default_resume_storage_dir(),
        );
        let _ = resume_manager.load().await;

        let mut last_successful_chunk_index: u32 = 0;
        let mut total_received: u64 = 0;

        for _ in 0..metadata.chunks.len() {
            // 读取分块消息头
            let header_result = MessageHeader::read_from_stream(stream).await;
            let header = match header_result {
                Ok(h) => h,
                Err(err) => {
                    // 网络中断，保存断点信息
                    if negotiated.resume {
                        let mut resume_info = crate::transfer::resume::ResumeInfo::new(
                            format!("recv-{}", metadata.hash),
                            metadata.name.clone(),
                            metadata.size,
                            metadata.hash.clone(),
                            total_received,
                            last_successful_chunk_index,
                            peer_addr.clone(),
                            0,
                            "receive".to_string(),
                        );
                        resume_info.save_path = Some(save_path.to_string_lossy().to_string());
                        let _ = resume_manager.save_resume_info(resume_info).await;
                    }
                    return Err(err);
                }
            };

            if header.message_type != MessageType::ChunkData {
                return Err(TransferError::Network("期望分块数据".to_string()));
            }

            // 读取分块数据
            let mut chunk_buf = vec![0u8; header.payload_length as usize];
            stream.read_exact(&mut chunk_buf).await?;
            let chunk_message: ChunkMessage = serde_json::from_slice(&chunk_buf)?;

            // 可选解密
            let decrypted_data = if let Some(ref session) = crypto_session {
                session.decrypt(&chunk_message.data)?
            } else {
                chunk_message.data
            };

            // 可选解压
            let final_data = if chunk_message.compressed {
                crate::transfer::compression::Compressor::decompress(&decrypted_data)?
            } else {
                decrypted_data
            };

            // 写入文件
            let chunk_info = &metadata.chunks[chunk_message.index as usize];
            self.chunker
                .write_chunk(&save_path, chunk_info, &final_data)?;

            last_successful_chunk_index = chunk_message.index;
            total_received += chunk_info.size;

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

        // 传输完成，清理断点信息
        let _ = resume_manager
            .remove_resume_info(&format!("recv-{}", metadata.hash))
            .await;

        Ok(())
    }

    /// 接收文件分块（使用指定配置，v1 兼容方法）
    #[allow(dead_code)]
    async fn receive_file_chunks_with_config(
        &self,
        stream: &mut TcpStream,
        metadata: &FileMetadata,
        receive_directory: &PathBuf,
        file_overwrite: bool,
    ) -> TransferResult<()> {
        self.receive_file_chunks_with_features(
            stream,
            metadata,
            receive_directory,
            file_overwrite,
            None,
            &NegotiatedFeatures::default(),
        )
        .await
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

/// 握手请求载荷
///
/// 在传输开始前交换双方支持的特性标志
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandshakePayload {
    /// 协议版本
    protocol_version: u8,
    /// 是否支持加密
    supports_encryption: bool,
    /// 是否支持压缩
    supports_compression: bool,
    /// 是否支持断点续传
    supports_resume: bool,
    /// 加密公钥（X25519，仅在支持加密时有值）
    public_key: Option<Vec<u8>>,
}

/// 握手响应载荷
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandshakeAckPayload {
    /// 协议版本
    protocol_version: u8,
    /// 是否同意使用加密
    use_encryption: bool,
    /// 是否同意使用压缩
    use_compression: bool,
    /// 是否同意使用断点续传
    use_resume: bool,
    /// 加密公钥（X25519，仅在同意加密时有值）
    public_key: Option<Vec<u8>>,
}

/// 协商后的传输特性
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
struct NegotiatedFeatures {
    /// 是否使用加密
    encryption: bool,
    /// 是否使用压缩
    compression: bool,
    /// 是否使用断点续传
    resume: bool,
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
    /// 数据是否经过压缩
    #[serde(default)]
    compressed: bool,
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
        assert_eq!(bytes.len(), 10);
        let parsed = MessageHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.message_type, MessageType::FileRequest);
        assert_eq!(parsed.payload_length, 100);
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = [0u8; 10];
        bytes[0..4].copy_from_slice(b"XXXX");
        let result = MessageHeader::from_bytes(&bytes);
        assert!(result.is_err());
    }
}
