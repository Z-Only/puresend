//! HTTP 传输加密模块
//!
//! 提供 P-256 ECDH 密钥交换和 AES-256-GCM 加密/解密功能，
//! 用于保护 HTTP 传输模式下的文件数据。
//!
//! 与 P2P 模式使用 X25519 不同，HTTP 模式使用 P-256 ECDH
//! 以兼容浏览器 Web Crypto API。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use hkdf::Hkdf;
use p256::ecdh::EphemeralSecret;
use p256::PublicKey;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const NONCE_SIZE: usize = 12;
const SESSION_EXPIRY: Duration = Duration::from_secs(3600);
const HKDF_INFO: &[u8] = b"puresend-http-encryption";

pub struct HttpCryptoSession {
    cipher: Aes256Gcm,
    nonce_counter: u64,
    #[allow(dead_code)]
    pub client_ip: String,
    created_at: Instant,
}

impl HttpCryptoSession {
    fn new(shared_secret: &[u8], client_ip: String) -> Result<Self, String> {
        let hk = Hkdf::<Sha256>::new(None, shared_secret);
        let mut key = [0u8; 32];
        hk.expand(HKDF_INFO, &mut key)
            .map_err(|e| format!("HKDF 密钥派生失败: {}", e))?;

        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| format!("创建 AES-256-GCM 实例失败: {}", e))?;

        Ok(Self {
            cipher,
            nonce_counter: 0,
            client_ip,
            created_at: Instant::now(),
        })
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > SESSION_EXPIRY
    }

    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let nonce_bytes = self.next_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("加密失败: {}", e))?;

        let mut output = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    pub fn decrypt(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
        if encrypted_data.len() < NONCE_SIZE {
            return Err("加密数据太短".to_string());
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("解密失败: {}", e))
    }

    fn next_nonce(&mut self) -> [u8; NONCE_SIZE] {
        let mut nonce = [0u8; NONCE_SIZE];
        nonce[..8].copy_from_slice(&self.nonce_counter.to_le_bytes());
        let mut random_part = [0u8; 4];
        OsRng.fill_bytes(&mut random_part);
        nonce[8..].copy_from_slice(&random_part);
        self.nonce_counter += 1;
        nonce
    }
}

#[derive(Debug, Deserialize)]
pub struct HandshakeRequest {
    pub client_public_key: String,
}

#[derive(Debug, Serialize)]
pub struct HandshakeResponse {
    pub encryption: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

pub struct HttpCryptoSessionManager {
    sessions: HashMap<String, HttpCryptoSession>,
}

impl std::fmt::Debug for HttpCryptoSessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpCryptoSessionManager")
            .field("sessions_count", &self.sessions.len())
            .finish()
    }
}

impl HttpCryptoSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub fn handshake(
        &mut self,
        client_public_key_b64: &str,
        client_ip: String,
    ) -> Result<(String, String), String> {
        let b64 = base64::engine::general_purpose::STANDARD;

        let client_pub_bytes = b64
            .decode(client_public_key_b64)
            .map_err(|e| format!("Base64 解码失败: {}", e))?;

        let client_public = PublicKey::from_sec1_bytes(&client_pub_bytes)
            .map_err(|e| format!("无效的 P-256 公钥: {}", e))?;

        let server_secret = EphemeralSecret::random(&mut OsRng);
        let server_public = server_secret.public_key();

        let shared_secret = server_secret.diffie_hellman(&client_public);

        let session =
            HttpCryptoSession::new(shared_secret.raw_secret_bytes().as_slice(), client_ip)?;

        let session_id = uuid::Uuid::new_v4().to_string();
        let server_pub_b64 = b64.encode(server_public.to_sec1_bytes());

        self.sessions.insert(session_id.clone(), session);

        Ok((session_id, server_pub_b64))
    }

    pub fn get_session(&self, session_id: &str) -> Option<&HttpCryptoSession> {
        self.sessions.get(session_id).filter(|s| !s.is_expired())
    }

    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut HttpCryptoSession> {
        match self.sessions.get_mut(session_id) {
            Some(session) if !session.is_expired() => Some(session),
            _ => None,
        }
    }

    pub fn cleanup_expired(&mut self) {
        self.sessions.retain(|_, s| !s.is_expired());
    }
}
