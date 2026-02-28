//! 完整性校验模块
//!
//! 提供文件传输前后的数据完整性验证

use crate::error::TransferResult;
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
}

impl Default for IntegrityChecker {
    fn default() -> Self {
        Self::new()
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
