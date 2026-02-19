//! 完整性校验模块
//!
//! 提供文件传输前后的数据完整性验证

use crate::error::TransferResult;
use crate::models::{ChunkInfo, FileMetadata};
use crate::transfer::FileChunker;
use std::path::Path;

/// 完整性校验器
pub struct IntegrityChecker {
    chunker: FileChunker,
}

impl IntegrityChecker {
    /// 创建新的校验器
    pub fn new() -> Self {
        Self {
            chunker: FileChunker::default_chunker(),
        }
    }

    /// 使用指定分块大小创建校验器
    #[allow(dead_code)]
    pub fn with_chunk_size(chunk_size: u64) -> Self {
        Self {
            chunker: FileChunker::new(chunk_size),
        }
    }

    /// 验证文件完整性
    ///
    /// 比较文件的实际哈希与期望哈希
    ///
    /// # Arguments
    /// * `file_path` - 文件路径
    /// * `expected_hash` - 期望的 SHA256 哈希值
    ///
    /// # Returns
    /// * `TransferResult<bool>` - 校验结果
    pub fn verify_file(&self, file_path: &Path, expected_hash: &str) -> TransferResult<bool> {
        let actual_hash = self.chunker.compute_file_hash(file_path)?;
        Ok(actual_hash == expected_hash)
    }

    /// 验证指定分块的完整性
    ///
    /// # Arguments
    /// * `file_path` - 文件路径
    /// * `chunk` - 分块信息（包含期望哈希）
    ///
    /// # Returns
    /// * `TransferResult<bool>` - 校验结果
    #[allow(dead_code)]
    pub fn verify_chunk(&self, file_path: &Path, chunk: &ChunkInfo) -> TransferResult<bool> {
        let data = self.chunker.read_chunk(file_path, chunk)?;
        let actual_hash = FileChunker::compute_hash(&data);
        Ok(actual_hash == chunk.hash)
    }

    /// 验证整个文件的所有分块
    ///
    /// # Arguments
    /// * `file_path` - 文件路径
    /// * `chunks` - 所有分块信息
    ///
    /// # Returns
    /// * `TransferResult<Vec<(u32, bool)>>` - 每个分块的校验结果（索引, 是否通过）
    #[allow(dead_code)]
    pub fn verify_all_chunks(
        &self,
        file_path: &Path,
        chunks: &[ChunkInfo],
    ) -> TransferResult<Vec<(u32, bool)>> {
        let mut results = Vec::with_capacity(chunks.len());

        for chunk in chunks {
            let is_valid = if chunk.hash.is_empty() {
                // 如果分块没有哈希值，跳过校验
                true
            } else {
                self.verify_chunk(file_path, chunk)?
            };
            results.push((chunk.index, is_valid));
        }

        Ok(results)
    }

    /// 完整验证文件元数据
    ///
    /// 验证文件大小、总哈希和所有分块哈希
    ///
    /// # Arguments
    /// * `file_path` - 文件路径
    /// * `metadata` - 文件元数据
    ///
    /// # Returns
    /// * `TransferResult<VerificationResult>` - 验证结果详情
    #[allow(dead_code)]
    pub fn verify_metadata(
        &self,
        file_path: &Path,
        metadata: &FileMetadata,
    ) -> TransferResult<VerificationResult> {
        let mut result = VerificationResult::default();

        // 检查文件是否存在
        if !file_path.exists() {
            return Ok(VerificationResult {
                file_exists: false,
                ..Default::default()
            });
        }

        result.file_exists = true;

        // 检查文件大小
        let actual_size = std::fs::metadata(file_path)?.len();
        result.size_matches = actual_size == metadata.size;

        // 检查总哈希
        if !metadata.hash.is_empty() {
            result.hash_matches = self.verify_file(file_path, &metadata.hash)?;
        } else {
            result.hash_matches = true;
        }

        // 检查所有分块
        if !metadata.chunks.is_empty() {
            let chunk_results = self.verify_all_chunks(file_path, &metadata.chunks)?;
            result.failed_chunks = chunk_results
                .iter()
                .filter(|(_, valid)| !valid)
                .map(|(index, _)| *index)
                .collect();
            result.all_chunks_valid = result.failed_chunks.is_empty();
        } else {
            result.all_chunks_valid = true;
        }

        result.is_valid = result.file_exists
            && result.size_matches
            && result.hash_matches
            && result.all_chunks_valid;

        Ok(result)
    }

    /// 快速校验 - 仅检查文件大小和部分分块
    ///
    /// 用于传输过程中的快速验证
    ///
    /// # Arguments
    /// * `file_path` - 文件路径
    /// * `metadata` - 文件元数据
    /// * `sample_count` - 抽样校验的分块数量
    ///
    /// # Returns
    /// * `TransferResult<bool>` - 快速校验结果
    #[allow(dead_code)]
    pub fn quick_verify(
        &self,
        file_path: &Path,
        metadata: &FileMetadata,
        sample_count: usize,
    ) -> TransferResult<bool> {
        // 检查文件大小
        if !file_path.exists() {
            return Ok(false);
        }

        let actual_size = std::fs::metadata(file_path)?.len();
        if actual_size != metadata.size {
            return Ok(false);
        }

        // 如果没有分块信息，仅验证大小
        if metadata.chunks.is_empty() {
            return Ok(true);
        }

        // 抽样校验分块
        let sample_step = (metadata.chunks.len() / sample_count).max(1);
        for i in (0..metadata.chunks.len()).step_by(sample_step) {
            if !self.verify_chunk(file_path, &metadata.chunks[i])? {
                return Ok(false);
            }
        }

        // 总是校验最后一个分块
        if let Some(last_chunk) = metadata.chunks.last() {
            if !self.verify_chunk(file_path, last_chunk)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

impl Default for IntegrityChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 验证结果
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct VerificationResult {
    /// 文件是否存在
    pub file_exists: bool,
    /// 文件大小是否匹配
    pub size_matches: bool,
    /// 总哈希是否匹配
    pub hash_matches: bool,
    /// 所有分块是否有效
    pub all_chunks_valid: bool,
    /// 失败的分块索引列表
    pub failed_chunks: Vec<u32>,
    /// 整体验证结果
    pub is_valid: bool,
}

impl VerificationResult {
    /// 获取失败原因描述
    #[allow(dead_code)]
    pub fn failure_reason(&self) -> Option<String> {
        if !self.file_exists {
            return Some("文件不存在".to_string());
        }
        if !self.size_matches {
            return Some("文件大小不匹配".to_string());
        }
        if !self.hash_matches {
            return Some("文件哈希校验失败".to_string());
        }
        if !self.all_chunks_valid {
            return Some(format!(
                "{} 个分块校验失败: {:?}",
                self.failed_chunks.len(),
                self.failed_chunks
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_verify_file() {
        let checker = IntegrityChecker::new();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test content").unwrap();
        temp_file.flush().unwrap();

        let hash = checker.chunker.compute_file_hash(temp_file.path()).unwrap();
        assert!(checker.verify_file(temp_file.path(), &hash).unwrap());
        assert!(!checker
            .verify_file(temp_file.path(), "invalid_hash")
            .unwrap());
    }
}
