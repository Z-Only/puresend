//! 文件元数据模型

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// 文件唯一标识
    pub id: String,
    /// 文件名
    pub name: String,
    /// 文件大小（字节）
    pub size: u64,
    /// MIME 类型
    pub mime_type: String,
    /// 文件哈希（用于校验）
    pub hash: String,
    /// 分块信息
    pub chunks: Vec<ChunkInfo>,
    /// 文件路径（发送时为源路径，接收时为目标路径）
    pub path: Option<String>,
}

impl FileMetadata {
    /// 创建新的文件元数据
    pub fn new(name: String, size: u64, mime_type: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            size,
            mime_type,
            hash: String::new(),
            chunks: Vec::new(),
            path: None,
        }
    }

    /// 根据文件扩展名推断 MIME 类型
    pub fn infer_mime_type(filename: &str) -> String {
        let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();
        match extension.as_str() {
            // 文本类型
            "txt" => "text/plain".to_string(),
            "md" => "text/markdown".to_string(),
            "json" => "application/json".to_string(),
            "xml" => "application/xml".to_string(),
            "html" | "htm" => "text/html".to_string(),
            "css" => "text/css".to_string(),
            "js" => "application/javascript".to_string(),
            "ts" => "application/typescript".to_string(),
            
            // 图像类型
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "gif" => "image/gif".to_string(),
            "webp" => "image/webp".to_string(),
            "svg" => "image/svg+xml".to_string(),
            "bmp" => "image/bmp".to_string(),
            "ico" => "image/x-icon".to_string(),
            
            // 视频类型
            "mp4" => "video/mp4".to_string(),
            "avi" => "video/x-msvideo".to_string(),
            "mov" => "video/quicktime".to_string(),
            "mkv" => "video/x-matroska".to_string(),
            "webm" => "video/webm".to_string(),
            
            // 音频类型
            "mp3" => "audio/mpeg".to_string(),
            "wav" => "audio/wav".to_string(),
            "ogg" => "audio/ogg".to_string(),
            "flac" => "audio/flac".to_string(),
            
            // 文档类型
            "pdf" => "application/pdf".to_string(),
            "doc" => "application/msword".to_string(),
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document".to_string(),
            "xls" => "application/vnd.ms-excel".to_string(),
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            "ppt" => "application/vnd.ms-powerpoint".to_string(),
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation".to_string(),
            
            // 压缩文件
            "zip" => "application/zip".to_string(),
            "rar" => "application/vnd.rar".to_string(),
            "7z" => "application/x-7z-compressed".to_string(),
            "tar" => "application/x-tar".to_string(),
            "gz" => "application/gzip".to_string(),
            
            // 其他
            _ => "application/octet-stream".to_string(),
        }
    }

    /// 获取文件扩展名
    pub fn extension(&self) -> Option<&str> {
        self.name.rsplit('.').next()
    }

    /// 计算分块数量
    pub fn chunk_count(&self, chunk_size: u64) -> u32 {
        if self.size == 0 {
            return 0;
        }
        ((self.size + chunk_size - 1) / chunk_size) as u32
    }
}

/// 分块信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// 块序号（从 0 开始）
    pub index: u32,
    /// 块大小（字节）
    pub size: u64,
    /// 块偏移量
    pub offset: u64,
    /// 块哈希
    pub hash: String,
}

impl ChunkInfo {
    /// 创建新的分块信息
    pub fn new(index: u32, size: u64, offset: u64) -> Self {
        Self {
            index,
            size,
            offset,
            hash: String::new(),
        }
    }
}

/// 默认分块大小：1MB
pub const DEFAULT_CHUNK_SIZE: u64 = 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_mime_type() {
        assert_eq!(FileMetadata::infer_mime_type("test.txt"), "text/plain");
        assert_eq!(FileMetadata::infer_mime_type("image.jpg"), "image/jpeg");
        assert_eq!(FileMetadata::infer_mime_type("doc.pdf"), "application/pdf");
        assert_eq!(FileMetadata::infer_mime_type("unknown.xyz"), "application/octet-stream");
    }

    #[test]
    fn test_chunk_count() {
        let meta = FileMetadata::new("test.txt".to_string(), 2_500_000, "text/plain".to_string());
        assert_eq!(meta.chunk_count(1_000_000), 3);
    }
}
