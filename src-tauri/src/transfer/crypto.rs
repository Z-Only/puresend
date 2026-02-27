//! 传输加密模块
//!
//! 提供 X25519 ECDH 密钥交换和 AES-256-GCM 加密/解密功能，
//! 用于保护 P2P 直连模式下的文件传输数据。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret, PublicKey, SharedSecret};

use crate::error::{TransferError, TransferResult};

/// AES-256-GCM nonce 大小（12 字节）
const NONCE_SIZE: usize = 12;

/// 密钥交换公钥载荷（用于握手阶段传输）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct KeyExchangePayload {
    /// X25519 公钥（32 字节，base64 编码）
    pub public_key: Vec<u8>,
}

/// 加密会话
///
/// 封装一次传输会话中的加密状态，包括共享密钥和 AES-256-GCM 密码实例。
pub struct CryptoSession {
    /// AES-256-GCM 密码实例
    cipher: Aes256Gcm,
    /// nonce 计数器（每次加密递增，防止 nonce 重用）
    nonce_counter: u64,
}

/// 密钥交换发起方
///
/// 生成临时密钥对，发送公钥给对方，接收对方公钥后派生共享密钥。
pub struct KeyExchangeInitiator {
    secret: EphemeralSecret,
    public_key: PublicKey,
}

impl KeyExchangeInitiator {
    /// 创建新的密钥交换发起方
    pub fn new() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public_key = PublicKey::from(&secret);
        Self { secret, public_key }
    }

    /// 获取本方公钥（发送给对方）
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.as_bytes().to_vec()
    }

    /// 使用对方公钥完成密钥交换，生成加密会话
    pub fn complete(self, peer_public_key: &[u8]) -> TransferResult<CryptoSession> {
        let peer_key_bytes: [u8; 32] = peer_public_key.try_into().map_err(|_| {
            TransferError::KeyExchange("对方公钥长度无效，期望 32 字节".to_string())
        })?;

        let peer_public = PublicKey::from(peer_key_bytes);
        let shared_secret: SharedSecret = self.secret.diffie_hellman(&peer_public);

        CryptoSession::from_shared_secret(shared_secret.as_bytes())
    }
}

/// 密钥交换响应方
///
/// 接收对方公钥后生成自己的临时密钥对，派生共享密钥。
pub struct KeyExchangeResponder {
    secret: EphemeralSecret,
    public_key: PublicKey,
}

impl KeyExchangeResponder {
    /// 创建新的密钥交换响应方
    pub fn new() -> Self {
        let secret = EphemeralSecret::random_from_rng(OsRng);
        let public_key = PublicKey::from(&secret);
        Self { secret, public_key }
    }

    /// 获取本方公钥（发送给对方）
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.as_bytes().to_vec()
    }

    /// 使用对方公钥完成密钥交换，生成加密会话
    pub fn complete(self, peer_public_key: &[u8]) -> TransferResult<CryptoSession> {
        let peer_key_bytes: [u8; 32] = peer_public_key.try_into().map_err(|_| {
            TransferError::KeyExchange("对方公钥长度无效，期望 32 字节".to_string())
        })?;

        let peer_public = PublicKey::from(peer_key_bytes);
        let shared_secret: SharedSecret = self.secret.diffie_hellman(&peer_public);

        CryptoSession::from_shared_secret(shared_secret.as_bytes())
    }
}

impl CryptoSession {
    /// 从共享密钥创建加密会话
    fn from_shared_secret(shared_secret: &[u8; 32]) -> TransferResult<Self> {
        let cipher = Aes256Gcm::new_from_slice(shared_secret)
            .map_err(|e| TransferError::Encryption(format!("创建 AES-256-GCM 实例失败: {}", e)))?;

        Ok(Self {
            cipher,
            nonce_counter: 0,
        })
    }

    /// 加密数据
    ///
    /// 使用递增 nonce 加密数据，返回 nonce + 密文。
    /// 输出格式：[12 字节 nonce][密文 + 16 字节 tag]
    pub fn encrypt(&mut self, plaintext: &[u8]) -> TransferResult<Vec<u8>> {
        let nonce_bytes = self.next_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| TransferError::Encryption(format!("加密失败: {}", e)))?;

        // 输出格式：nonce + ciphertext
        let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);

        Ok(output)
    }

    /// 解密数据
    ///
    /// 输入格式：[12 字节 nonce][密文 + 16 字节 tag]
    pub fn decrypt(&self, encrypted_data: &[u8]) -> TransferResult<Vec<u8>> {
        if encrypted_data.len() < NONCE_SIZE {
            return Err(TransferError::Decryption(
                "加密数据太短，无法提取 nonce".to_string(),
            ));
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| TransferError::Decryption(format!("解密失败: {}", e)))
    }

    /// 生成下一个 nonce（基于计数器）
    fn next_nonce(&mut self) -> [u8; NONCE_SIZE] {
        let mut nonce = [0u8; NONCE_SIZE];
        // 前 8 字节使用计数器，后 4 字节使用随机数
        nonce[..8].copy_from_slice(&self.nonce_counter.to_le_bytes());
        let mut random_part = [0u8; 4];
        OsRng.fill_bytes(&mut random_part);
        nonce[8..].copy_from_slice(&random_part);
        self.nonce_counter += 1;
        nonce
    }
}

/// 加密设置状态（由前端同步到后端）
static ENCRYPTION_ENABLED: std::sync::OnceLock<std::sync::RwLock<bool>> =
    std::sync::OnceLock::new();

fn get_encryption_lock() -> &'static std::sync::RwLock<bool> {
    ENCRYPTION_ENABLED.get_or_init(|| std::sync::RwLock::new(true))
}

/// 获取加密是否启用
pub fn is_encryption_enabled() -> bool {
    get_encryption_lock().read().map(|v| *v).unwrap_or(true)
}

/// 设置加密启用状态
pub fn set_encryption_enabled_internal(enabled: bool) {
    if let Ok(mut lock) = get_encryption_lock().write() {
        *lock = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_exchange_and_encrypt_decrypt() {
        // 模拟双方密钥交换
        let initiator = KeyExchangeInitiator::new();
        let responder = KeyExchangeResponder::new();

        let initiator_pub = initiator.public_key_bytes();
        let responder_pub = responder.public_key_bytes();

        let mut session_a = initiator.complete(&responder_pub).unwrap();
        let session_b = responder.complete(&initiator_pub).unwrap();

        // A 加密，B 解密
        let plaintext = b"Hello, PureSend!";
        let encrypted = session_a.encrypt(plaintext).unwrap();
        let decrypted = session_b.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let initiator = KeyExchangeInitiator::new();
        let responder = KeyExchangeResponder::new();

        let initiator_pub = initiator.public_key_bytes();
        let responder_pub = responder.public_key_bytes();

        let mut session_a = initiator.complete(&responder_pub).unwrap();
        let session_b = responder.complete(&initiator_pub).unwrap();

        // 测试 1MB 数据
        let plaintext = vec![0xABu8; 1024 * 1024];
        let encrypted = session_a.encrypt(&plaintext).unwrap();
        let decrypted = session_b.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_invalid_key_length() {
        let initiator = KeyExchangeInitiator::new();
        let result = initiator.complete(&[0u8; 16]);
        assert!(result.is_err());
    }
}
