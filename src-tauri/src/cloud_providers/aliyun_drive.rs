//! 阿里云盘实现
//!
//! 使用阿里云盘 Open API 实现文件操作。

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tauri::Emitter;

use crate::cloud::{CloudProvider, CloudError, CloudFileItem, CloudUploadProgress, CloudDownloadProgress};

/// 阿里云盘 API 基础 URL
const ALIYUN_DRIVE_API: &str = "https://openapi.alipan.com";

/// 阿里云盘凭证
#[derive(Debug, Clone)]
pub struct DriveCredentials {
    /// Refresh Token
    pub refresh_token: String,
}

/// 阿里云盘 Access Token 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

/// 阿里云盘用户信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct UserInfo {
    user_id: String,
    name: String,
    #[serde(default)]
    avatar: Option<String>,
}

/// 阿里云盘文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DriveFile {
    file_id: String,
    name: String,
    file_type: String,
    size: Option<u64>,
    updated_at: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>,
}

/// 阿里云盘文件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileListResponse {
    items: Vec<DriveFile>,
    next_marker: Option<String>,
}

/// 阿里云盘上传 URL 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadUrlResponse {
    upload_url: String,
    upload_id: String,
}

/// 阿里云盘下载 URL 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DownloadUrlResponse {
    url: String,
    expiration: String,
}

/// 阿里云盘 Provider
pub struct AliyunDriveProvider {
    /// HTTP 客户端
    client: Client,
    /// Refresh Token
    refresh_token: String,
    /// 缓存的 Access Token
    access_token: Option<String>,
}

impl AliyunDriveProvider {
    /// 创建阿里云盘 Provider 实例
    pub fn new(credentials: DriveCredentials) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            client,
            refresh_token: credentials.refresh_token,
            access_token: None,
        }
    }

    /// 刷新并获取 Access Token
    async fn get_access_token(&mut self) -> Result<String, CloudError> {
        // 如果已有 access_token，直接返回
        if let Some(ref token) = self.access_token {
            return Ok(token.clone());
        }

        // 刷新 token
        let url = format!("{}/oauth/access_token", ALIYUN_DRIVE_API);
        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "refresh_token": self.refresh_token,
                "grant_type": "refresh_token"
            }))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("刷新 Token 失败: {}", e)))?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(CloudError::Authentication(format!("认证失败，状态码: {}", status)));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| CloudError::ParseError(format!("解析 Token 响应失败: {}", e)))?;

        // 更新缓存的 token 和 refresh_token
        self.access_token = Some(token_response.access_token.clone());
        self.refresh_token = token_response.refresh_token;

        Ok(token_response.access_token)
    }

    /// 获取根目录 ID（阿里云盘使用 root）
    fn get_root_directory_id() -> &'static str {
        "root"
    }

    /// 根据路径获取文件 ID
    async fn get_file_id_by_path(&mut self, path: &str) -> Result<String, CloudError> {
        if path == "/" || path.is_empty() {
            return Ok(Self::get_root_directory_id().to_string());
        }

        let access_token = self.get_access_token().await?;
        let url = format!("{}/adrive/v1.0/openFile/get_by_path", ALIYUN_DRIVE_API);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&serde_json::json!({
                "file_path": path,
                "drive_id": "" // 使用默认 drive
            }))
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        if status == 404 {
            return Err(CloudError::NotFound(format!("路径不存在: {}", path)));
        }
        if status != 200 {
            return Err(CloudError::Network(format!("获取文件 ID 失败: {}", status)));
        }

        #[derive(Deserialize)]
        struct FileIdResponse {
            file_id: String,
        }

        let result: FileIdResponse = response
            .json()
            .await
            .map_err(|e| CloudError::ParseError(e.to_string()))?;

        Ok(result.file_id)
    }

    /// 将阿里云盘文件转换为统一的 CloudFileItem
    fn convert_file_item(file: DriveFile, parent_path: &str) -> CloudFileItem {
        let is_directory = file.file_type == "folder";
        let path = if parent_path == "/" {
            format!("/{}", file.name)
        } else {
            format!("{}/{}", parent_path, file.name)
        };

        let modified = file.updated_at
            .and_then(|dt| chrono::DateTime::parse_from_rfc3339(&dt).ok())
            .map(|dt| dt.timestamp_millis() as u64);

        CloudFileItem {
            name: file.name,
            path,
            is_directory,
            size: file.size,
            modified,
        }
    }
}

#[async_trait]
impl CloudProvider for AliyunDriveProvider {
    async fn test_connection(&self) -> Result<bool, CloudError> {
        // 通过获取用户信息测试连接
        let mut self_mut = self.clone();
        let access_token = self_mut.get_access_token().await?;

        let url = format!("{}/adrive/v1.0/user/getDriveInfo", ALIYUN_DRIVE_API);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        match status {
            200 => Ok(true),
            401 | 403 => Err(CloudError::Authentication("Refresh Token 无效或已过期".to_string())),
            _ => Err(CloudError::Network(format!("服务器返回状态码: {}", status))),
        }
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<CloudFileItem>, CloudError> {
        let mut self_mut = self.clone();
        let access_token = self_mut.get_access_token().await?;
        
        // 获取父目录 ID
        let parent_file_id = self_mut.get_file_id_by_path(path).await?;

        let url = format!("{}/adrive/v1.0/openFile/list", ALIYUN_DRIVE_API);
        
        let mut all_items = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut request_body = serde_json::json!({
                "parent_file_id": parent_file_id,
                "limit": 100,
                "order_by": "name",
                "order_direction": "ASC"
            });

            if let Some(ref m) = marker {
                request_body["marker"] = serde_json::Value::String(m.clone());
            }

            let response = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", access_token))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| CloudError::Network(e.to_string()))?;

            let status = response.status().as_u16();
            if status != 200 {
                return Err(CloudError::Network(format!("列出文件失败: {}", status)));
            }

            let result: FileListResponse = response
                .json()
                .await
                .map_err(|e| CloudError::ParseError(e.to_string()))?;

            for file in result.items {
                all_items.push(Self::convert_file_item(file, path));
            }

            // 检查是否还有更多数据
            marker = result.next_marker;
            if marker.is_none() {
                break;
            }
        }

        // 排序：目录在前，文件在后
        all_items.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(all_items)
    }

    async fn upload_file(
        &self,
        local_path: &str,
        remote_path: &str,
        app_handle: Option<&tauri::AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError> {
        let mut self_mut = self.clone();
        let access_token = self_mut.get_access_token().await?;

        // 读取本地文件
        let file_content = tokio::fs::read(local_path)
            .await
            .map_err(|e| CloudError::Io(format!("读取本地文件失败: {}", e)))?;

        let total_bytes = file_content.len() as u64;

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

        // 获取父目录 ID
        let parent_path = std::path::Path::new(remote_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        
        let parent_file_id = self_mut.get_file_id_by_path(&parent_path).await?;

        // 获取文件名
        let file_name = std::path::Path::new(remote_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        // 1. 获取上传 URL
        let url = format!("{}/adrive/v1.0/openFile/getUploadUrl", ALIYUN_DRIVE_API);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&serde_json::json!({
                "drive_id": "",
                "name": file_name,
                "parent_file_id": parent_file_id,
                "type": "file",
                "size": total_bytes,
                "check_name_mode": "overwrite"
            }))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("获取上传 URL 失败: {}", e)))?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(CloudError::Network(format!("获取上传 URL 失败: {}", status)));
        }

        let upload_info: UploadUrlResponse = response
            .json()
            .await
            .map_err(|e| CloudError::ParseError(e.to_string()))?;

        // 2. 上传文件内容
        let upload_response = self
            .client
            .put(&upload_info.upload_url)
            .header("Content-Type", "application/octet-stream")
            .body(file_content)
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("上传失败: {}", e)))?;

        let upload_status = upload_response.status().as_u16();

        // 发送上传完成/失败事件
        if let Some(handle) = app_handle {
            let status_str = if (200..300).contains(&upload_status) {
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
                progress: if status_str == "completed" { 100.0 } else { 0.0 },
                status: status_str.to_string(),
            });
        }

        match upload_status {
            200..=299 => Ok(()),
            401 | 403 => Err(CloudError::Authentication("认证失败".to_string())),
            _ => Err(CloudError::Network(format!("上传失败，状态码: {}", upload_status))),
        }
    }

    async fn download_file(
        &self,
        remote_path: &str,
        local_path: &str,
        app_handle: Option<&tauri::AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError> {
        let mut self_mut = self.clone();
        let access_token = self_mut.get_access_token().await?;

        // 获取文件 ID
        let file_id = self_mut.get_file_id_by_path(remote_path).await?;

        // 获取下载 URL
        let url = format!("{}/adrive/v1.0/openFile/getDownloadUrl", ALIYUN_DRIVE_API);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&serde_json::json!({
                "file_id": file_id,
                "drive_id": ""
            }))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("获取下载 URL 失败: {}", e)))?;

        let status = response.status().as_u16();
        if status != 200 {
            return Err(CloudError::Network(format!("获取下载 URL 失败: {}", status)));
        }

        let download_info: DownloadUrlResponse = response
            .json()
            .await
            .map_err(|e| CloudError::ParseError(e.to_string()))?;

        // 下载文件
        let download_response = self
            .client
            .get(&download_info.url)
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("下载请求失败: {}", e)))?;

        let total_bytes = download_response.content_length().unwrap_or(0);

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

        let bytes = download_response
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
        let mut self_mut = self.clone();
        let access_token = self_mut.get_access_token().await?;

        // 获取父目录 ID
        let parent_path = std::path::Path::new(path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        
        let parent_file_id = self_mut.get_file_id_by_path(&parent_path).await?;

        // 获取目录名
        let dir_name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or_else(|| CloudError::Io("无效的目录名".to_string()))?;

        // 创建目录
        let url = format!("{}/adrive/v1.0/openFile/create", ALIYUN_DRIVE_API);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&serde_json::json!({
                "drive_id": "",
                "parent_file_id": parent_file_id,
                "name": dir_name,
                "type": "folder",
                "check_name_mode": "refuse"
            }))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("创建目录失败: {}", e)))?;

        let status = response.status().as_u16();
        match status {
            200..=299 => Ok(()),
            409 => Err(CloudError::AlreadyExists(format!("目录已存在: {}", path))),
            401 | 403 => Err(CloudError::Authentication("认证失败".to_string())),
            _ => Err(CloudError::Network(format!("创建目录失败，状态码: {}", status))),
        }
    }

    async fn exists(&self, path: &str) -> Result<bool, CloudError> {
        let mut self_mut = self.clone();
        match self_mut.get_file_id_by_path(path).await {
            Ok(_) => Ok(true),
            Err(CloudError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

impl Clone for AliyunDriveProvider {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            refresh_token: self.refresh_token.clone(),
            access_token: self.access_token.clone(),
        }
    }
}
