//! 传输压缩模块
//!
//! 提供 zstd 流式压缩/解压功能，支持智能压缩策略（根据文件 MIME 类型自动选择压缩级别）。

use crate::error::{TransferError, TransferResult};

/// 压缩模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    /// 智能压缩：根据文件类型自动选择压缩级别
    Smart,
    /// 手动压缩：使用指定的压缩级别
    Manual(i32),
}

/// 压缩器
///
/// 封装 zstd 压缩/解压操作，支持智能压缩策略。
pub struct Compressor {
    /// 压缩模式
    mode: CompressionMode,
}

impl Compressor {
    /// 创建智能压缩器
    pub fn smart() -> Self {
        Self {
            mode: CompressionMode::Smart,
        }
    }

    /// 创建手动压缩器
    pub fn manual(level: i32) -> Self {
        // zstd 压缩级别范围：1-22，clamp 到有效范围
        let clamped_level = level.clamp(1, 19);
        Self {
            mode: CompressionMode::Manual(clamped_level),
        }
    }

    /// 根据 MIME 类型判断是否应该跳过压缩
    ///
    /// 已压缩的文件格式（如 zip、mp4、jpg）再次压缩效果极差，应跳过。
    pub fn should_skip_compression(mime_type: &str) -> bool {
        matches!(
            mime_type,
            // 压缩文件
            "application/zip"
                | "application/gzip"
                | "application/x-7z-compressed"
                | "application/vnd.rar"
                | "application/x-tar"
                // 已压缩的图片
                | "image/jpeg"
                | "image/webp"
                | "image/gif"
                // 已压缩的视频
                | "video/mp4"
                | "video/x-matroska"
                | "video/webm"
                | "video/quicktime"
                | "video/x-msvideo"
                // 已压缩的音频
                | "audio/mpeg"
                | "audio/ogg"
                | "audio/flac"
                | "audio/aac"
        )
    }

    /// 根据 MIME 类型获取智能压缩级别
    ///
    /// 智能压缩策略：
    /// - 文档类文件（文本、JSON、XML 等）：高压缩级别 9
    /// - 无损图片（PNG、BMP、TIFF）：中等压缩级别 3
    /// - 已压缩文件：返回 None（跳过压缩）
    /// - 其他文件：默认压缩级别 3
    pub fn smart_compression_level(mime_type: &str) -> Option<i32> {
        if Self::should_skip_compression(mime_type) {
            return None;
        }

        let level = if mime_type.starts_with("text/")
            || matches!(
                mime_type,
                "application/json"
                    | "application/xml"
                    | "application/javascript"
                    | "application/typescript"
                    | "application/msword"
                    | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                    | "application/vnd.ms-excel"
                    | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
                    | "application/vnd.ms-powerpoint"
                    | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                    | "application/pdf"
            )
        {
            // 文档类文件：高压缩级别
            9
        } else if matches!(
            mime_type,
            "image/png" | "image/bmp" | "image/x-icon" | "image/svg+xml"
        ) {
            // 无损图片：中等压缩级别
            3
        } else {
            // 其他文件：默认压缩级别
            3
        };

        Some(level)
    }

    /// 获取当前压缩级别（根据 MIME 类型）
    ///
    /// 返回 None 表示应跳过压缩。
    pub fn get_level(&self, mime_type: &str) -> Option<i32> {
        match self.mode {
            CompressionMode::Smart => Self::smart_compression_level(mime_type),
            CompressionMode::Manual(level) => {
                if Self::should_skip_compression(mime_type) {
                    None
                } else {
                    Some(level)
                }
            }
        }
    }

    /// 压缩数据块
    ///
    /// # Arguments
    /// * `data` - 原始数据
    /// * `level` - 压缩级别（1-19）
    ///
    /// # Returns
    /// 压缩后的数据
    pub fn compress(data: &[u8], level: i32) -> TransferResult<Vec<u8>> {
        zstd::encode_all(std::io::Cursor::new(data), level)
            .map_err(|e| TransferError::Compression(format!("zstd 压缩失败: {}", e)))
    }

    /// 解压数据块
    ///
    /// # Arguments
    /// * `compressed_data` - 压缩后的数据
    ///
    /// # Returns
    /// 解压后的原始数据
    pub fn decompress(compressed_data: &[u8]) -> TransferResult<Vec<u8>> {
        zstd::decode_all(std::io::Cursor::new(compressed_data))
            .map_err(|e| TransferError::Decompression(format!("zstd 解压失败: {}", e)))
    }
}

/// 压缩设置状态（由前端同步到后端）
static COMPRESSION_SETTINGS: std::sync::OnceLock<std::sync::RwLock<CompressionConfig>> =
    std::sync::OnceLock::new();

/// 压缩配置
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// 是否启用压缩
    pub enabled: bool,
    /// 压缩模式（"smart" 或 "manual"）
    pub mode: String,
    /// 手动压缩级别（1-19）
    pub level: i32,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "smart".to_string(),
            level: 3,
        }
    }
}

fn get_compression_lock() -> &'static std::sync::RwLock<CompressionConfig> {
    COMPRESSION_SETTINGS.get_or_init(|| std::sync::RwLock::new(CompressionConfig::default()))
}

/// 获取当前压缩配置
pub fn get_compression_config() -> CompressionConfig {
    get_compression_lock()
        .read()
        .map(|v| v.clone())
        .unwrap_or_default()
}

/// 设置压缩启用状态
pub fn set_compression_enabled_internal(enabled: bool) {
    if let Ok(mut lock) = get_compression_lock().write() {
        lock.enabled = enabled;
    }
}

/// 设置压缩模式
pub fn set_compression_mode_internal(mode: String) {
    if let Ok(mut lock) = get_compression_lock().write() {
        lock.mode = mode;
    }
}

/// 设置压缩级别
pub fn set_compression_level_internal(level: i32) {
    if let Ok(mut lock) = get_compression_lock().write() {
        lock.level = level.clamp(1, 19);
    }
}

/// 根据当前配置创建压缩器
pub fn create_compressor_from_config() -> Option<Compressor> {
    let config = get_compression_config();
    if !config.enabled {
        return None;
    }

    match config.mode.as_str() {
        "smart" => Some(Compressor::smart()),
        "manual" => Some(Compressor::manual(config.level)),
        _ => Some(Compressor::smart()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let data = b"Hello, PureSend! This is a test for zstd compression.".repeat(100);
        let compressed = Compressor::compress(&data, 3).unwrap();
        let decompressed = Compressor::decompress(&compressed).unwrap();
        assert_eq!(data, decompressed);
        assert!(compressed.len() < data.len());
    }

    #[test]
    fn test_smart_compression_level() {
        // 文档类文件应使用高压缩级别
        assert_eq!(Compressor::smart_compression_level("text/plain"), Some(9));
        assert_eq!(
            Compressor::smart_compression_level("application/json"),
            Some(9)
        );

        // 无损图片应使用中等压缩级别
        assert_eq!(Compressor::smart_compression_level("image/png"), Some(3));

        // 已压缩文件应跳过
        assert_eq!(Compressor::smart_compression_level("image/jpeg"), None);
        assert_eq!(Compressor::smart_compression_level("video/mp4"), None);
        assert_eq!(
            Compressor::smart_compression_level("application/zip"),
            None
        );

        // 其他文件使用默认级别
        assert_eq!(
            Compressor::smart_compression_level("application/octet-stream"),
            Some(3)
        );
    }

    #[test]
    fn test_should_skip_compression() {
        assert!(Compressor::should_skip_compression("application/zip"));
        assert!(Compressor::should_skip_compression("image/jpeg"));
        assert!(Compressor::should_skip_compression("video/mp4"));
        assert!(!Compressor::should_skip_compression("text/plain"));
        assert!(!Compressor::should_skip_compression("image/png"));
    }
}
