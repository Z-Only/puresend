//! 阿里云 OSS 云盘实现
//!
//! 使用 OSS RESTful API 实现文件操作。

use async_trait::async_trait;
use base64::Engine;
use reqwest::Client;
use sha2::Sha256;
use hmac::{Hmac, Mac};
use tauri::Emitter;

use crate::cloud::{CloudProvider, CloudError, CloudFileItem, CloudUploadProgress, CloudDownloadProgress};

type HmacSha256 = Hmac<Sha256>;

/// 阿里云 OSS 凭证
#[derive(Debug, Clone)]
pub struct OSSCredentials {
    /// Bucket 名称
    pub bucket: String,
    /// Region ID（如 oss-cn-hangzhou）
    pub region: String,
    /// AccessKey ID
    pub access_key_id: String,
    /// AccessKey Secret
    pub access_key_secret: String,
    /// 自定义域名（可选）
    pub custom_domain: Option<String>,
}

/// 阿里云 OSS Provider
pub struct AliyunOSSProvider {
    /// HTTP 客户端
    client: Client,
    /// 凭证
    credentials: OSSCredentials,
    /// Endpoint
    endpoint: String,
}

impl AliyunOSSProvider {
    /// 创建 OSS Provider 实例
    pub fn new(credentials: OSSCredentials) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        // 构建Endpoint：优先使用自定义域名
        let endpoint = if let Some(ref custom_domain) = credentials.custom_domain {
            custom_domain.clone()
        } else {
            format!(
                "https://{}.oss-{}.aliyuncs.com",
                credentials.bucket, credentials.region
            )
        };

        Self {
            client,
            credentials,
            endpoint,
        }
    }

    /// 构建对象 URL
    fn build_url(&self, path: &str) -> String {
        let normalized_path = if path.starts_with('/') {
            path
        } else {
            &format!("/{}", path)
        };
        format!("{}{}", self.endpoint, normalized_path)
    }

    /// 生成 OSS 签名
    fn generate_signature(
        &self,
        method: &str,
        resource: &str,
        date: &str,
        content_type: &str,
        headers: &[(&str, &str)],
    ) -> String {
        let mut string_to_sign = format!("{}\n\n{}\n{}\n", method, content_type, date);
        
        // 添加 CanonicalizedOSSHeaders
        let mut oss_headers: Vec<(&str, &str)> = headers.to_vec();
        oss_headers.sort_by(|a, b| a.0.cmp(b.0));
        
        let mut canonicalized_headers = String::new();
        for (key, value) in oss_headers {
            if key.starts_with("x-oss-") {
                canonicalized_headers.push_str(&format!("{}:{}\n", key.to_lowercase(), value));
            }
        }
        
        if !canonicalized_headers.is_empty() {
            string_to_sign.push_str(&canonicalized_headers);
        }
        
        // 添加 CanonicalizedResource
        string_to_sign.push_str(resource);

        // HMAC-SHA1 签名
        let mut mac = HmacSha256::new_from_slice(self.credentials.access_key_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        
        base64::engine::general_purpose::STANDARD.encode(result.into_bytes())
    }

    /// 解析 ListBucketResult XML 响应
    fn parse_list_response(&self, xml_body: &str) -> Result<Vec<CloudFileItem>, CloudError> {
        let mut items = Vec::new();

        // 简单的 XML 解析
        let contents: Vec<&str> = xml_body.split("<Contents>").collect();
        for content in contents.iter().skip(1) {
            let key = Self::extract_tag_content(content, "Key");
            let size_str = Self::extract_tag_content(content, "Size");
            let last_modified = Self::extract_tag_content(content, "LastModified");

            if key.is_empty() {
                continue;
            }

            let name = key.rsplit('/').next().unwrap_or(&key).to_string();
            let size = size_str.parse::<u64>().ok();
            let modified = chrono::DateTime::parse_from_rfc3339(&last_modified)
                .ok()
                .map(|dt| dt.timestamp_millis() as u64);

            items.push(CloudFileItem {
                name,
                path: format!("/{}", key),
                is_directory: false,
                size,
                modified,
            });
        }

        // 解析 CommonPrefixes（目录）
        let prefixes: Vec<&str> = xml_body.split("<CommonPrefixes>").collect();
        for prefix in prefixes.iter().skip(1) {
            let prefix_path = Self::extract_tag_content(prefix, "Prefix");
            if prefix_path.is_empty() {
                continue;
            }

            let name = prefix_path.trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or(&prefix_path)
                .to_string();

            items.push(CloudFileItem {
                name,
                path: format!("/{}", prefix_path),
                is_directory: true,
                size: None,
                modified: None,
            });
        }

        // 排序：目录在前，文件在后
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
    fn extract_tag_content(xml: &str, tag_name: &str) -> String {
        let start_pattern = format!("<{}>", tag_name);
        let end_pattern = format!("</{}>", tag_name);

        if let Some(start) = xml.find(&start_pattern) {
            let content_start = start + start_pattern.len();
            if let Some(end) = xml[content_start..].find(&end_pattern) {
                return xml[content_start..content_start + end].trim().to_string();
            }
        }
        String::new()
    }

    /// 获取 GMT 格式的当前时间
    fn get_gmt_date() -> String {
        chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }
}

#[async_trait]
impl CloudProvider for AliyunOSSProvider {
    async fn test_connection(&self) -> Result<bool, CloudError> {
        let date = Self::get_gmt_date();
        let resource = format!("/{}/", self.credentials.bucket);
        let signature = self.generate_signature("GET", &resource, &date, "", &[]);

        let url = self.build_url("/?max-keys=1");

        let response = self
            .client
            .get(&url)
            .header("Date", &date)
            .header("Authorization", format!("OSS {}:{}", self.credentials.access_key_id, signature))
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        match status {
            200 => Ok(true),
            403 => Err(CloudError::Authentication("认证失败，请检查 AccessKey".to_string())),
            404 => Err(CloudError::NotFound("Bucket 不存在".to_string())),
            _ => Err(CloudError::Network(format!("服务器返回状态码: {}", status))),
        }
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<CloudFileItem>, CloudError> {
        let normalized_path = path.trim_start_matches('/');
        let prefix = if normalized_path.is_empty() {
            String::new()
        } else if normalized_path.ends_with('/') {
            normalized_path.to_string()
        } else {
            format!("{}/", normalized_path)
        };

        let date = Self::get_gmt_date();
        let resource = format!("/{}/?prefix={}&delimiter=/", self.credentials.bucket, prefix);
        let signature = self.generate_signature("GET", &resource, &date, "", &[]);

        let url = format!(
            "{}?prefix={}&delimiter=/",
            self.endpoint,
            urlencoding::encode(&prefix)
        );

        let response = self
            .client
            .get(&url)
            .header("Date", &date)
            .header("Authorization", format!("OSS {}:{}", self.credentials.access_key_id, signature))
            .send()
            .await
            .map_err(|e| CloudError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        match status {
            200 => {
                let body = response
                    .text()
                    .await
                    .map_err(|e| CloudError::Network(e.to_string()))?;
                self.parse_list_response(&body)
            }
            403 => Err(CloudError::Authentication("认证失败".to_string())),
            404 => Err(CloudError::NotFound(format!("目录不存在: {}", path))),
            _ => Err(CloudError::Network(format!("服务器返回状态码: {}", status))),
        }
    }

    async fn upload_file(
        &self,
        local_path: &str,
        remote_path: &str,
        app_handle: Option<&tauri::AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError> {
        let file_content = tokio::fs::read(local_path)
            .await
            .map_err(|e| CloudError::Io(format!("读取本地文件失败: {}", e)))?;

        let total_bytes = file_content.len() as u64;
        let object_key = remote_path.trim_start_matches('/');
        let url = self.build_url(&format!("/{}", object_key));

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

        let date = Self::get_gmt_date();
        let content_type = "application/octet-stream";
        let resource = format!("/{}/{}", self.credentials.bucket, object_key);
        let signature = self.generate_signature("PUT", &resource, &date, content_type, &[]);

        let response = self
            .client
            .put(&url)
            .header("Date", &date)
            .header("Content-Type", content_type)
            .header("Authorization", format!("OSS {}:{}", self.credentials.access_key_id, signature))
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
            403 => Err(CloudError::Authentication("认证失败".to_string())),
            404 => Err(CloudError::NotFound("目标 Bucket 不存在".to_string())),
            _ => Err(CloudError::Network(format!("上传失败，状态码: {}", status))),
        }
    }

    async fn download_file(
        &self,
        remote_path: &str,
        local_path: &str,
        app_handle: Option<&tauri::AppHandle>,
        account_id: &str,
    ) -> Result<(), CloudError> {
        let object_key = remote_path.trim_start_matches('/');
        let url = self.build_url(&format!("/{}", object_key));

        let date = Self::get_gmt_date();
        let resource = format!("/{}/{}", self.credentials.bucket, object_key);
        let signature = self.generate_signature("GET", &resource, &date, "", &[]);

        let response = self
            .client
            .get(&url)
            .header("Date", &date)
            .header("Authorization", format!("OSS {}:{}", self.credentials.access_key_id, signature))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("下载请求失败: {}", e)))?;

        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            return match status {
                403 => Err(CloudError::Authentication("认证失败".to_string())),
                404 => Err(CloudError::NotFound(format!("文件不存在: {}", remote_path))),
                _ => Err(CloudError::Network(format!("下载失败，状态码: {}", status))),
            };
        }

        let total_bytes = response.content_length().unwrap_or(0);

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
        // OSS 没有真正的目录概念，通过创建一个以 / 结尾的空对象来模拟目录
        let dir_path = if path.ends_with('/') {
            path.to_string()
        } else {
            format!("{}/", path)
        };

        let object_key = dir_path.trim_start_matches('/');
        let url = self.build_url(&format!("/{}", object_key));

        let date = Self::get_gmt_date();
        let resource = format!("/{}/{}", self.credentials.bucket, object_key);
        let signature = self.generate_signature("PUT", &resource, &date, "application/x-directory", &[]);

        let response = self
            .client
            .put(&url)
            .header("Date", &date)
            .header("Content-Type", "application/x-directory")
            .header("Authorization", format!("OSS {}:{}", self.credentials.access_key_id, signature))
            .body(Vec::new())
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("创建目录失败: {}", e)))?;

        let status = response.status().as_u16();
        match status {
            200..=299 => Ok(()),
            403 => Err(CloudError::Authentication("认证失败".to_string())),
            _ => Err(CloudError::Network(format!("创建目录失败，状态码: {}", status))),
        }
    }

    async fn exists(&self, path: &str) -> Result<bool, CloudError> {
        let object_key = path.trim_start_matches('/');
        let url = self.build_url(&format!("/{}", object_key));

        let date = Self::get_gmt_date();
        let resource = format!("/{}/{}", self.credentials.bucket, object_key);
        let signature = self.generate_signature("HEAD", &resource, &date, "", &[]);

        let response = self
            .client
            .head(&url)
            .header("Date", &date)
            .header("Authorization", format!("OSS {}:{}", self.credentials.access_key_id, signature))
            .send()
            .await
            .map_err(|e| CloudError::Network(format!("检查路径失败: {}", e)))?;

        let status = response.status().as_u16();
        match status {
            200..=299 => Ok(true),
            404 => Ok(false),
            403 => Err(CloudError::Authentication("认证失败".to_string())),
            _ => Err(CloudError::Network(format!("检查路径失败，状态码: {}", status))),
        }
    }
}
