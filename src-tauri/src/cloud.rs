//! 云盘中转模块
//!
//! 提供云盘文件传输功能，支持 WebDAV 协议。
//! 架构设计为可扩展模式，后续可添加 OSS、网盘等类型。

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use async_trait::async_trait;
use hkdf::Hkdf;
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::StoreExt;
use tokio::sync::Mutex;

// ============ 数据结构 ============

/// 云盘类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CloudType {
    /// WebDAV 协议（如坚果云、NextCloud）
    WebDAV,
}

/// 云盘账号连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CloudAccountStatus {
    /// 已连接（连接测试通过）
    Connected,
    /// 未连接（未测试或初始状态）
    Disconnected,
    /// 无效（凭证错误或服务不可用）
    Invalid,
}

/// 云盘账号信息（不含密码明文）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccount {
    /// 唯一标识
    pub id: String,
    /// 账号名称（用户自定义）
    pub name: String,
    /// 云盘类型
    pub cloud_type: CloudType,
    /// 默认上传/下载目录
    pub default_directory: String,
    /// 连接状态
    pub status: CloudAccountStatus,
    /// 创建时间（Unix 毫秒时间戳）
    pub created_at: u64,
    /// 更新时间（Unix 毫秒时间戳）
    pub updated_at: u64,
}

/// 云盘账号凭证信息（用于编辑时回显，不含密码）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccountCredentials {
    /// 服务器地址
    pub server_url: String,
    /// 用户名
    pub username: String,
}

/// 云盘文件/目录项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudFileItem {
    /// 文件/目录名称
    pub name: String,
    /// 完整路径
    pub path: String,
    /// 是否为目录
    pub is_directory: bool,
    /// 文件大小（目录为 None）
    pub size: Option<u64>,
    /// 最后修改时间（Unix 毫秒时间戳）
    pub modified: Option<u64>,
}

/// WebDAV 凭证（前端传入）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDAVCredentials {
    /// 服务器地址（如 https://dav.jianguoyun.com/dav/）
    pub server_url: String,
    /// 用户名
    pub username: String,
    /// 密码/应用密码
    pub password: String,
}

/// 添加/更新云盘账号的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccountInput {
    /// 账号名称
    pub name: String,
    /// 云盘类型
    pub cloud_type: CloudType,
    /// 凭证信息
    pub credentials: WebDAVCredentials,
    /// 默认目录
    pub default_directory: String,
}

/// 连接测试输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudConnectionTestInput {
    /// 云盘类型
    pub cloud_type: CloudType,
    /// 凭证信息
    pub credentials: WebDAVCredentials,
}

/// 云盘上传进度事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudUploadProgress {
    /// 账号 ID
    pub account_id: String,
    /// 本地文件路径
    pub local_path: String,
    /// 远程文件路径
    pub remote_path: String,
    /// 已上传字节数
    pub uploaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 进度百分比（0-100）
    pub progress: f64,
    /// 状态
    pub status: String,
}

/// 云盘下载进度事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudDownloadProgress {
    /// 账号 ID
    pub account_id: String,
    /// 远程文件路径
    pub remote_path: String,
    /// 本地保存路径
    pub local_path: String,
    /// 已下载字节数
    pub downloaded_bytes: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 进度百分比（0-100）
    pub progress: f64,
    /// 状态
    pub status: String,
}

/// 存储在 Tauri Store 中的账号数据（含加密后的密码）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAccountData {
    /// 账号元信息
    account: CloudAccount,
    /// WebDAV 服务器地址
    server_url: String,
    /// 用户名
    username: String,
    /// 加密后的密码（Base64 编码）
    encrypted_password: String,
    /// 加密随机数（Base64 编码）
    password_nonce: String,
}

// ============ 云盘操作 Trait ============

/// 云盘操作接口
///
/// 所有云盘类型都需要实现此 trait，提供统一的文件操作接口。
#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// 测试连接是否可用
    async fn test_connection(&self) -> Result<bool, CloudError>;

    /// 列出指定目录下的文件和子目录
    async fn list_directory(&self, path: &str) -> Result<Vec<CloudFileItem>, CloudError>;

    /// 上传文件到云盘
    async fn upload_file(
        &self,
        local_path: &str,
        remote_path: &str,
        app_handle: Option<&AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError>;

    /// 从云盘下载文件到本地
    async fn download_file(
        &self,
        remote_path: &str,
        local_path: &str,
        app_handle: Option<&AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError>;

    /// 创建远程目录
    async fn create_directory(&self, path: &str) -> Result<(), CloudError>;

    /// 检查远程路径是否存在
    #[allow(dead_code)]
    async fn exists(&self, path: &str) -> Result<bool, CloudError>;
}

// ============ 错误类型 ============

/// 云盘操作错误
#[derive(Debug, thiserror::Error, Serialize)]
pub enum CloudError {
    #[error("网络请求失败: {0}")]
    Network(String),

    #[error("认证失败: {0}")]
    Authentication(String),

    #[error("文件未找到: {0}")]
    NotFound(String),

    #[error("目录已存在: {0}")]
    AlreadyExists(String),

    #[error("IO 错误: {0}")]
    Io(String),

    #[error("XML 解析失败: {0}")]
    #[allow(dead_code)]
    ParseError(String),

    #[error("凭证加密/解密失败: {0}")]
    Encryption(String),

    #[error("存储操作失败: {0}")]
    Storage(String),

    #[error("账号未找到: {0}")]
    AccountNotFound(String),

    #[error("不支持的云盘类型: {0}")]
    #[allow(dead_code)]
    UnsupportedType(String),

    #[error("内部错误: {0}")]
    #[allow(dead_code)]
    Internal(String),
}

// ============ WebDAV 实现 ============

/// WebDAV 云盘操作实现
pub struct WebDAVProvider {
    /// HTTP 客户端
    client: Client,
    /// 服务器基础 URL
    server_url: String,
    /// 用户名
    username: String,
    /// 密码
    password: String,
}

impl WebDAVProvider {
    /// 创建 WebDAV 提供者实例
    pub fn new(server_url: &str, username: &str, password: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        // 确保 server_url 以 / 结尾
        let normalized_url = if server_url.ends_with('/') {
            server_url.to_string()
        } else {
            format!("{}/", server_url)
        };

        Self {
            client,
            server_url: normalized_url,
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// 构建完整的 WebDAV URL
    fn build_url(&self, path: &str) -> String {
        let trimmed_path = path.trim_start_matches('/');
        format!("{}{}", self.server_url, trimmed_path)
    }

    /// 解析 PROPFIND XML 响应，提取文件列表
    fn parse_propfind_response(
        &self,
        xml_body: &str,
        request_path: &str,
    ) -> Result<Vec<CloudFileItem>, CloudError> {
        let mut items = Vec::new();

        // 简单的 XML 解析：提取 <D:response> 块中的信息
        // WebDAV PROPFIND 响应格式：
        // <D:multistatus>
        //   <D:response>
        //     <D:href>/path/to/resource</D:href>
        //     <D:propstat>
        //       <D:prop>
        //         <D:getcontentlength>1234</D:getcontentlength>
        //         <D:getlastmodified>Mon, 01 Jan 2024 00:00:00 GMT</D:getlastmodified>
        //         <D:resourcetype><D:collection/></D:resourcetype>
        //       </D:prop>
        //     </D:propstat>
        //   </D:response>
        // </D:multistatus>

        // 规范化请求路径用于比较（跳过自身）
        let normalized_request_path = Self::normalize_path_for_comparison(request_path);

        // 按 response 块分割
        let response_blocks: Vec<&str> = xml_body.split("<D:response>")
            .chain(xml_body.split("<d:response>"))
            .collect();

        for block in response_blocks.iter().skip(1) {
            let end_tag_pos = block.find("</D:response>")
                .or_else(|| block.find("</d:response>"));
            let block_content = match end_tag_pos {
                Some(pos) => &block[..pos],
                None => block,
            };

            // 提取 href
            let href = Self::extract_tag_content(block_content, "href");
            if href.is_empty() {
                continue;
            }

            // URL 解码 href
            let decoded_href = urlencoding::decode(&href).unwrap_or_else(|_| href.clone().into());
            let decoded_href = decoded_href.to_string();

            // 跳过请求路径自身
            let normalized_href = Self::normalize_path_for_comparison(&decoded_href);
            if normalized_href == normalized_request_path {
                continue;
            }

            // 判断是否为目录
            let is_directory = block_content.contains("<D:collection")
                || block_content.contains("<d:collection")
                || block_content.contains("<D:collection/>")
                || block_content.contains("<d:collection/>");

            // 提取文件大小
            let size_str = Self::extract_tag_content(block_content, "getcontentlength");
            let size = if is_directory {
                None
            } else {
                size_str.parse::<u64>().ok()
            };

            // 提取最后修改时间
            let modified_str = Self::extract_tag_content(block_content, "getlastmodified");
            let modified = Self::parse_http_date(&modified_str);

            // 从 href 中提取文件名
            let name = Self::extract_name_from_href(&decoded_href);
            if name.is_empty() {
                continue;
            }

            // 构建路径
            let item_path = if is_directory {
                let trimmed = decoded_href.trim_end_matches('/');
                // 从 server_url 的路径部分之后截取
                Self::extract_relative_path(&self.server_url, trimmed)
            } else {
                Self::extract_relative_path(&self.server_url, &decoded_href)
            };

            items.push(CloudFileItem {
                name,
                path: item_path,
                is_directory,
                size,
                modified,
            });
        }

        // 排序：目录在前，文件在后；同类型按名称排序
        items.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(items)
    }

    /// 从 XML 内容中提取指定标签的文本内容
    /// 支持带命名空间前缀的标签（D: 或 d:）
    fn extract_tag_content(xml: &str, tag_name: &str) -> String {
        // 尝试多种命名空间前缀
        let patterns = [
            format!("<D:{}>", tag_name),
            format!("<d:{}>", tag_name),
            format!("<{}>", tag_name),
        ];
        let end_patterns = [
            format!("</D:{}>", tag_name),
            format!("</d:{}>", tag_name),
            format!("</{}>", tag_name),
        ];

        for (start_pattern, end_pattern) in patterns.iter().zip(end_patterns.iter()) {
            if let Some(start) = xml.find(start_pattern.as_str()) {
                let content_start = start + start_pattern.len();
                if let Some(end) = xml[content_start..].find(end_pattern.as_str()) {
                    return xml[content_start..content_start + end].trim().to_string();
                }
            }
        }

        String::new()
    }

    /// 从 href 中提取文件/目录名称
    fn extract_name_from_href(href: &str) -> String {
        let trimmed = href.trim_end_matches('/');
        trimmed
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string()
    }

    /// 规范化路径用于比较（去除末尾斜杠，提取路径部分）
    fn normalize_path_for_comparison(path: &str) -> String {
        // 如果是完整 URL，提取路径部分
        let path_part = if path.starts_with("http://") || path.starts_with("https://") {
            match url_path_from_full_url(path) {
                Some(p) => p,
                None => path.to_string(),
            }
        } else {
            path.to_string()
        };
        path_part.trim_end_matches('/').to_string()
    }

    /// 从完整 href 中提取相对于 server_url 的路径
    fn extract_relative_path(server_url: &str, href: &str) -> String {
        // 尝试从 server_url 中提取基础路径
        let base_path = url_path_from_full_url(server_url)
            .unwrap_or_default();
        let base_path = base_path.trim_end_matches('/');

        // href 可能是完整 URL 或仅路径
        let href_path = if href.starts_with("http://") || href.starts_with("https://") {
            url_path_from_full_url(href).unwrap_or_else(|| href.to_string())
        } else {
            href.to_string()
        };

        // 去除基础路径前缀
        let relative = if !base_path.is_empty() && href_path.starts_with(base_path) {
            href_path[base_path.len()..].to_string()
        } else {
            href_path
        };

        // 确保以 / 开头
        if relative.starts_with('/') {
            relative
        } else {
            format!("/{}", relative)
        }
    }

    /// 解析 HTTP 日期格式为 Unix 毫秒时间戳
    fn parse_http_date(date_str: &str) -> Option<u64> {
        if date_str.is_empty() {
            return None;
        }
        // 尝试解析 RFC 2822 格式: "Mon, 01 Jan 2024 00:00:00 GMT"
        chrono::DateTime::parse_from_rfc2822(date_str)
            .ok()
            .map(|dt| dt.timestamp_millis() as u64)
            .or_else(|| {
                // 尝试解析 RFC 3339 格式
                chrono::DateTime::parse_from_rfc3339(date_str)
                    .ok()
                    .map(|dt| dt.timestamp_millis() as u64)
            })
    }
}

/// 从完整 URL 中提取路径部分
fn url_path_from_full_url(url: &str) -> Option<String> {
    // 简单解析：找到 :// 后的第一个 /
    let after_scheme = url.find("://").map(|i| i + 3)?;
    let path_start = url[after_scheme..].find('/').map(|i| i + after_scheme)?;
    Some(url[path_start..].to_string())
}

#[async_trait]
impl CloudProvider for WebDAVProvider {
    async fn test_connection(&self) -> Result<bool, CloudError> {
        let url = self.build_url("/");
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "0")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#)
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        match status {
            207 => Ok(true),
            401 | 403 => Err(CloudError::Authentication("认证失败，请检查用户名和密码".to_string())),
            404 => Err(CloudError::NotFound("服务器地址不正确".to_string())),
            _ => Err(CloudError::Network(format!("服务器返回状态码: {}", status))),
        }
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<CloudFileItem>, CloudError> {
        let normalized_path = if path.is_empty() || path == "/" {
            "/".to_string()
        } else if !path.ends_with('/') {
            format!("{}/", path)
        } else {
            path.to_string()
        };

        let url = self.build_url(&normalized_path);
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:resourcetype/>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:displayname/>
  </D:prop>
</D:propfind>"#)
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        match status {
            207 => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| CloudError::Network(e.to_string()))?;
                self.parse_propfind_response(&body, &url)
            }
            401 | 403 => Err(CloudError::Authentication("认证失败".to_string())),
            404 => Err(CloudError::NotFound(format!("目录不存在: {}", path))),
            _ => Err(CloudError::Network(format!("服务器返回状态码: {}", status))),
        }
    }

    async fn upload_file(
        &self,
        local_path: &str,
        remote_path: &str,
        app_handle: Option<&AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError> {
        let file_content = tokio::fs::read(local_path)
            .await
            .map_err(|e| CloudError::Io(format!("读取本地文件失败: {}", e)))?;

        let total_bytes = file_content.len() as u64;
        let url = self.build_url(remote_path);

        // 发送上传开始事件
        if let Some(handle) = app_handle {
            let _ = handle.emit("cloud-upload-progress", CloudUploadProgress {
                account_id: account_id.to_string(),
                local_path: local_path.to_string(),
                remote_path: remote_path.to_string(),
                uploaded_bytes: 0,
                total_bytes,
                progress: 0.0,
                status: "uploading".to_string(),
            });
        }

        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Content-Type", "application/octet-stream")
            .body(file_content)
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("上传失败: {}", e)))?;

        let status = response.status().as_u16();

        // 发送上传完成/失败事件
        if let Some(handle) = app_handle {
            let upload_status = if (200..300).contains(&status) {
                "completed"
            } else {
                "failed"
            };
            let _ = handle.emit("cloud-upload-progress", CloudUploadProgress {
                account_id: account_id.to_string(),
                local_path: local_path.to_string(),
                remote_path: remote_path.to_string(),
                uploaded_bytes: total_bytes,
                total_bytes,
                progress: if upload_status == "completed" { 100.0 } else { 0.0 },
                status: upload_status.to_string(),
            });
        }

        match status {
            200..=299 => Ok(()),
            401 | 403 => Err(CloudError::Authentication("认证失败".to_string())),
            404 => Err(CloudError::NotFound("目标目录不存在".to_string())),
            507 => Err(CloudError::Io("云盘空间不足".to_string())),
            _ => Err(CloudError::Network(format!("上传失败，状态码: {}", status))),
        }
    }

    async fn download_file(
        &self,
        remote_path: &str,
        local_path: &str,
        app_handle: Option<&AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError> {
        let url = self.build_url(remote_path);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("下载请求失败: {}", e)))?;

        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return match status {
                401 | 403 => Err(CloudError::Authentication("认证失败".to_string())),
                404 => Err(CloudError::NotFound(format!("文件不存在: {}", remote_path))),
                _ => Err(CloudError::Network(format!("下载失败，状态码: {}", status))),
            };
        }

        let total_bytes = response
            .content_length()
            .unwrap_or(0);

        // 发送下载开始事件
        if let Some(handle) = app_handle {
            let _ = handle.emit("cloud-download-progress", CloudDownloadProgress {
                account_id: account_id.to_string(),
                remote_path: remote_path.to_string(),
                local_path: local_path.to_string(),
                downloaded_bytes: 0,
                total_bytes,
                progress: 0.0,
                status: "downloading".to_string(),
            });
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| CloudError::Network(format!("读取响应数据失败: {}", e)))?;

        // 确保父目录存在
        if let Some(parent) = std::path::Path::new(local_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| CloudError::Io(format!("创建本地目录失败: {}", e)))?;
        }

        tokio::fs::write(local_path, &bytes)
            .await
            .map_err(|e| CloudError::Io(format!("写入本地文件失败: {}", e)))?;

        // 发送下载完成事件
        if let Some(handle) = app_handle {
            let _ = handle.emit("cloud-download-progress", CloudDownloadProgress {
                account_id: account_id.to_string(),
                remote_path: remote_path.to_string(),
                local_path: local_path.to_string(),
                downloaded_bytes: bytes.len() as u64,
                total_bytes,
                progress: 100.0,
                status: "completed".to_string(),
            });
        }

        Ok(())
    }

    async fn create_directory(&self, path: &str) -> Result<(), CloudError> {
        let normalized_path = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };

        let url = self.build_url(&normalized_path);
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("创建目录失败: {}", e)))?;

        let status = response.status().as_u16();
        match status {
            201 => Ok(()),
            405 => Err(CloudError::AlreadyExists(format!("目录已存在: {}", path))),
            401 | 403 => Err(CloudError::Authentication("认证失败".to_string())),
            409 => Err(CloudError::NotFound("父目录不存在".to_string())),
            507 => Err(CloudError::Io("云盘空间不足".to_string())),
            _ => Err(CloudError::Network(format!("创建目录失败，状态码: {}", status))),
        }
    }

    async fn exists(&self, path: &str) -> Result<bool, CloudError> {
        let url = self.build_url(path);
        let response = self
            .client
            .head(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("检查路径失败: {}", e)))?;

        let status = response.status().as_u16();
        match status {
            200..=299 => Ok(true),
            404 => Ok(false),
            401 | 403 => Err(CloudError::Authentication("认证失败".to_string())),
            _ => Err(CloudError::Network(format!("检查路径失败，状态码: {}", status))),
        }
    }
}

// ============ 凭证加密 ============

/// 加密上下文信息（用于 HKDF 密钥派生）
const ENCRYPTION_INFO: &[u8] = b"puresend-cloud-credential-encryption";

/// 从设备标识派生加密密钥
///
/// 使用 HKDF-SHA256 从设备唯一标识派生 AES-256 密钥，
/// 确保不同设备间密文不可互换。
fn derive_encryption_key() -> Result<[u8; 32], CloudError> {
    // 使用机器 hostname 作为设备标识的一部分
    let device_id = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "puresend-default-device".to_string());

    // 使用固定盐值 + 设备标识进行密钥派生
    let salt = format!("puresend-cloud-salt-{}", device_id);
    let ikm = format!("puresend-cloud-ikm-{}", device_id);

    let hk = Hkdf::<Sha256>::new(Some(salt.as_bytes()), ikm.as_bytes());
    let mut key = [0u8; 32];
    hk.expand(ENCRYPTION_INFO, &mut key)
        .map_err(|e| CloudError::Encryption(format!("密钥派生失败: {}", e)))?;

    Ok(key)
}

/// 加密密码
fn encrypt_password(password: &str) -> Result<(String, String), CloudError> {
    let key = derive_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CloudError::Encryption(format!("创建加密器失败: {}", e)))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, password.as_bytes())
        .map_err(|e| CloudError::Encryption(format!("加密失败: {}", e)))?;

    let encrypted_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &ciphertext,
    );
    let nonce_base64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &nonce_bytes,
    );

    Ok((encrypted_base64, nonce_base64))
}

/// 解密密码
fn decrypt_password(encrypted_base64: &str, nonce_base64: &str) -> Result<String, CloudError> {
    let key = derive_encryption_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CloudError::Encryption(format!("创建解密器失败: {}", e)))?;

    let ciphertext = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        encrypted_base64,
    )
    .map_err(|e| CloudError::Encryption(format!("Base64 解码密文失败: {}", e)))?;

    let nonce_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        nonce_base64,
    )
    .map_err(|e| CloudError::Encryption(format!("Base64 解码 nonce 失败: {}", e)))?;

    if nonce_bytes.len() != 12 {
        return Err(CloudError::Encryption("无效的 nonce 长度".to_string()));
    }

    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| CloudError::Encryption(format!("解密失败: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| CloudError::Encryption(format!("UTF-8 解码失败: {}", e)))
}

// ============ 存储管理 ============

/// 云盘账号存储文件名
const CLOUD_STORE_FILE: &str = "cloud-accounts.json";
/// 云盘账号存储键名
const CLOUD_STORE_KEY: &str = "accounts";

/// 云盘状态（用于 Tauri 状态管理）
pub struct CloudState {
    /// 缓存的账号列表
    accounts: Arc<Mutex<Vec<StoredAccountData>>>,
    /// 是否已初始化
    initialized: Arc<Mutex<bool>>,
}

impl CloudState {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(Mutex::new(Vec::new())),
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    /// 确保已从存储加载账号数据
    async fn ensure_initialized(&self, app_handle: &AppHandle) -> Result<(), CloudError> {
        let mut initialized = self.initialized.lock().await;
        if *initialized {
            return Ok(());
        }

        let stored = load_accounts_from_store(app_handle).await?;
        let mut accounts = self.accounts.lock().await;
        *accounts = stored;
        *initialized = true;
        Ok(())
    }

    /// 获取所有账号（不含密码）
    async fn list_accounts(&self, app_handle: &AppHandle) -> Result<Vec<CloudAccount>, CloudError> {
        self.ensure_initialized(app_handle).await?;
        let accounts = self.accounts.lock().await;
        Ok(accounts.iter().map(|a| a.account.clone()).collect())
    }

    /// 添加账号
    async fn add_account(
        &self,
        app_handle: &AppHandle,
        input: CloudAccountInput,
    ) -> Result<CloudAccount, CloudError> {
        self.ensure_initialized(app_handle).await?;

        let (encrypted_password, password_nonce) =
            encrypt_password(&input.credentials.password)?;

        let now = chrono::Utc::now().timestamp_millis() as u64;
        let account = CloudAccount {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            cloud_type: input.cloud_type,
            default_directory: input.default_directory,
            status: CloudAccountStatus::Disconnected,
            created_at: now,
            updated_at: now,
        };

        let stored = StoredAccountData {
            account: account.clone(),
            server_url: input.credentials.server_url,
            username: input.credentials.username,
            encrypted_password,
            password_nonce,
        };

        {
            let mut accounts = self.accounts.lock().await;
            accounts.push(stored);
        }

        self.save_to_store(app_handle).await?;
        Ok(account)
    }

    /// 更新账号
    async fn update_account(
        &self,
        app_handle: &AppHandle,
        account_id: &str,
        input: CloudAccountInput,
    ) -> Result<CloudAccount, CloudError> {
        self.ensure_initialized(app_handle).await?;

        let (encrypted_password, password_nonce) =
            encrypt_password(&input.credentials.password)?;

        let now = chrono::Utc::now().timestamp_millis() as u64;

        let mut accounts = self.accounts.lock().await;
        let stored = accounts
            .iter_mut()
            .find(|a| a.account.id == account_id)
            .ok_or_else(|| CloudError::AccountNotFound(account_id.to_string()))?;

        stored.account.name = input.name;
        stored.account.cloud_type = input.cloud_type;
        stored.account.default_directory = input.default_directory;
        stored.account.status = CloudAccountStatus::Disconnected;
        stored.account.updated_at = now;
        stored.server_url = input.credentials.server_url;
        stored.username = input.credentials.username;
        stored.encrypted_password = encrypted_password;
        stored.password_nonce = password_nonce;

        let updated_account = stored.account.clone();
        drop(accounts);

        self.save_to_store(app_handle).await?;
        Ok(updated_account)
    }

    /// 删除账号
    async fn delete_account(
        &self,
        app_handle: &AppHandle,
        account_id: &str,
    ) -> Result<(), CloudError> {
        self.ensure_initialized(app_handle).await?;

        let mut accounts = self.accounts.lock().await;
        let original_len = accounts.len();
        accounts.retain(|a| a.account.id != account_id);

        if accounts.len() == original_len {
            return Err(CloudError::AccountNotFound(account_id.to_string()));
        }

        drop(accounts);
        self.save_to_store(app_handle).await?;
        Ok(())
    }

    /// 获取指定账号的 WebDAV Provider
    async fn get_provider(
        &self,
        app_handle: &AppHandle,
        account_id: &str,
    ) -> Result<WebDAVProvider, CloudError> {
        self.ensure_initialized(app_handle).await?;

        let accounts = self.accounts.lock().await;
        let stored = accounts
            .iter()
            .find(|a| a.account.id == account_id)
            .ok_or_else(|| CloudError::AccountNotFound(account_id.to_string()))?;

        let password = decrypt_password(&stored.encrypted_password, &stored.password_nonce)?;

        Ok(WebDAVProvider::new(
            &stored.server_url,
            &stored.username,
            &password,
        ))
    }

    /// 获取账号凭证信息（不含密码，用于编辑时回显）
    async fn get_credentials(
        &self,
        app_handle: &AppHandle,
        account_id: &str,
    ) -> Result<CloudAccountCredentials, CloudError> {
        self.ensure_initialized(app_handle).await?;

        let accounts = self.accounts.lock().await;
        let stored = accounts
            .iter()
            .find(|a| a.account.id == account_id)
            .ok_or_else(|| CloudError::AccountNotFound(account_id.to_string()))?;

        Ok(CloudAccountCredentials {
            server_url: stored.server_url.clone(),
            username: stored.username.clone(),
        })
    }

    /// 更新账号连接状态
    async fn update_status(
        &self,
        app_handle: &AppHandle,
        account_id: &str,
        status: CloudAccountStatus,
    ) -> Result<(), CloudError> {
        let mut accounts = self.accounts.lock().await;
        if let Some(stored) = accounts.iter_mut().find(|a| a.account.id == account_id) {
            stored.account.status = status;
            stored.account.updated_at = chrono::Utc::now().timestamp_millis() as u64;
        }
        drop(accounts);
        self.save_to_store(app_handle).await
    }

    /// 保存到 Tauri Store
    async fn save_to_store(&self, app_handle: &AppHandle) -> Result<(), CloudError> {
        let accounts = self.accounts.lock().await;
        save_accounts_to_store(app_handle, &accounts).await
    }
}

impl Default for CloudState {
    fn default() -> Self {
        Self::new()
    }
}

/// 从 Tauri Store 加载账号数据
async fn load_accounts_from_store(
    app_handle: &AppHandle,
) -> Result<Vec<StoredAccountData>, CloudError> {
    let store = app_handle
        .store(CLOUD_STORE_FILE)
        .map_err(|e| CloudError::Storage(format!("打开存储失败: {}", e)))?;

    match store.get(CLOUD_STORE_KEY) {
        Some(value) => {
            let accounts: Vec<StoredAccountData> = serde_json::from_value(value)
                .map_err(|e| CloudError::Storage(format!("解析存储数据失败: {}", e)))?;
            Ok(accounts)
        }
        None => Ok(Vec::new()),
    }
}

/// 保存账号数据到 Tauri Store
async fn save_accounts_to_store(
    app_handle: &AppHandle,
    accounts: &[StoredAccountData],
) -> Result<(), CloudError> {
    let store = app_handle
        .store(CLOUD_STORE_FILE)
        .map_err(|e| CloudError::Storage(format!("打开存储失败: {}", e)))?;

    let value = serde_json::to_value(accounts)
        .map_err(|e| CloudError::Storage(format!("序列化数据失败: {}", e)))?;

    store.set(CLOUD_STORE_KEY, value);
    store
        .save()
        .map_err(|e| CloudError::Storage(format!("保存存储失败: {}", e)))?;

    Ok(())
}

// ============ Tauri Commands ============

/// 获取所有云盘账号列表
#[tauri::command]
pub async fn list_cloud_accounts(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
) -> Result<Vec<CloudAccount>, String> {
    state
        .list_accounts(&app_handle)
        .await
        .map_err(|e| e.to_string())
}

/// 添加云盘账号
#[tauri::command]
pub async fn add_cloud_account(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    input: CloudAccountInput,
) -> Result<CloudAccount, String> {
    state
        .add_account(&app_handle, input)
        .await
        .map_err(|e| e.to_string())
}

/// 更新云盘账号
#[tauri::command]
pub async fn update_cloud_account(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
    input: CloudAccountInput,
) -> Result<CloudAccount, String> {
    state
        .update_account(&app_handle, &account_id, input)
        .await
        .map_err(|e| e.to_string())
}

/// 删除云盘账号
#[tauri::command]
pub async fn delete_cloud_account(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
) -> Result<(), String> {
    state
        .delete_account(&app_handle, &account_id)
        .await
        .map_err(|e| e.to_string())
}

/// 测试已保存账号的连接
#[tauri::command]
pub async fn test_cloud_connection(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
) -> Result<bool, String> {
    let provider = state
        .get_provider(&app_handle, &account_id)
        .await
        .map_err(|e| e.to_string())?;

    match provider.test_connection().await {
        Ok(result) => {
            let status = if result {
                CloudAccountStatus::Connected
            } else {
                CloudAccountStatus::Invalid
            };
            let _ = state.update_status(&app_handle, &account_id, status).await;
            Ok(result)
        }
        Err(e) => {
            let _ = state
                .update_status(&app_handle, &account_id, CloudAccountStatus::Invalid)
                .await;
            Err(e.to_string())
        }
    }
}

/// 使用临时凭证测试连接（用于添加账号前的验证）
#[tauri::command]
pub async fn test_cloud_connection_with_credentials(
    input: CloudConnectionTestInput,
) -> Result<bool, String> {
    match input.cloud_type {
        CloudType::WebDAV => {
            let provider = WebDAVProvider::new(
                &input.credentials.server_url,
                &input.credentials.username,
                &input.credentials.password,
            );
            provider
                .test_connection()
                .await
                .map_err(|e| e.to_string())
        }
    }
}

/// 浏览云盘目录
#[tauri::command]
pub async fn browse_cloud_directory(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
    path: String,
) -> Result<Vec<CloudFileItem>, String> {
    let provider = state
        .get_provider(&app_handle, &account_id)
        .await
        .map_err(|e| e.to_string())?;

    provider
        .list_directory(&path)
        .await
        .map_err(|e| e.to_string())
}

/// 创建云盘目录
#[tauri::command]
pub async fn create_cloud_directory(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
    path: String,
) -> Result<(), String> {
    let provider = state
        .get_provider(&app_handle, &account_id)
        .await
        .map_err(|e| e.to_string())?;

    provider
        .create_directory(&path)
        .await
        .map_err(|e| e.to_string())
}

/// 上传文件到云盘
#[tauri::command]
pub async fn upload_to_cloud(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
    local_path: String,
    remote_path: String,
) -> Result<(), String> {
    let provider = state
        .get_provider(&app_handle, &account_id)
        .await
        .map_err(|e| e.to_string())?;

    provider
        .upload_file(&local_path, &remote_path, Some(&app_handle), &account_id)
        .await
        .map_err(|e| e.to_string())
}

/// 从云盘下载文件到本地
#[tauri::command]
pub async fn download_from_cloud(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
    remote_path: String,
    local_path: String,
) -> Result<(), String> {
    let provider = state
        .get_provider(&app_handle, &account_id)
        .await
        .map_err(|e| e.to_string())?;

    provider
        .download_file(&remote_path, &local_path, Some(&app_handle), &account_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取账号凭证信息（不含密码，用于编辑时回显）
#[tauri::command]
pub async fn get_cloud_account_credentials(
    app_handle: AppHandle,
    state: tauri::State<'_, CloudState>,
    account_id: String,
) -> Result<CloudAccountCredentials, String> {
    state
        .get_credentials(&app_handle, &account_id)
        .await
        .map_err(|e| e.to_string())
}