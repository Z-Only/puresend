//! 文件分块处理模块
//! 
//! 负责将大文件分割成固定大小的块，便于传输和断点续传

use crate::error::TransferResult;
use crate::models::{ChunkInfo, FileMetadata, DEFAULT_CHUNK_SIZE};
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// 文件分块器
pub struct FileChunker {
    /// 分块大小（字节）
    chunk_size: u64,
}

impl FileChunker {
    /// 创建新的分块器
    pub fn new(chunk_size: u64) -> Self {
        Self { chunk_size }
    }

    /// 使用默认分块大小创建分块器
    pub fn default_chunker() -> Self {
        Self::new(DEFAULT_CHUNK_SIZE)
    }

    /// 计算文件的分块信息
    /// 
    /// # Arguments
    /// * `file_path` - 文件路径
    /// 
    /// # Returns
    /// * `TransferResult<Vec<ChunkInfo>>` - 分块信息列表
    pub fn compute_chunks(&self, file_path: &Path) -> TransferResult<Vec<ChunkInfo>> {
        let metadata = std::fs::metadata(file_path)?;
        let file_size = metadata.len();
        
        if file_size == 0 {
            return Ok(Vec::new());
        }

        let mut chunks = Vec::new();
        let mut offset: u64 = 0;
        let mut index: u32 = 0;

        while offset < file_size {
            let chunk_size = self.chunk_size.min(file_size - offset);
            chunks.push(ChunkInfo::new(index, chunk_size, offset));
            offset += chunk_size;
            index += 1;
        }

        Ok(chunks)
    }

    /// 读取指定分块的数据
    /// 
    /// # Arguments
    /// * `file_path` - 文件路径
    /// * `chunk` - 分块信息
    /// 
    /// # Returns
    /// * `TransferResult<Vec<u8>>` - 分块数据
    pub fn read_chunk(&self, file_path: &Path, chunk: &ChunkInfo) -> TransferResult<Vec<u8>> {
        let mut file = File::open(file_path)?;
        file.seek(SeekFrom::Start(chunk.offset))?;
        
        let mut buffer = vec![0u8; chunk.size as usize];
        file.read_exact(&mut buffer)?;
        
        Ok(buffer)
    }

    /// 写入分块数据到文件
    /// 
    /// # Arguments
    /// * `file_path` - 目标文件路径
    /// * `chunk` - 分块信息
    /// * `data` - 分块数据
    /// 
    /// # Returns
    /// * `TransferResult<()>` - 操作结果
    pub fn write_chunk(&self, file_path: &Path, chunk: &ChunkInfo, data: &[u8]) -> TransferResult<()> {
        // 确保父目录存在
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = if file_path.exists() {
            File::options().write(true).open(file_path)?
        } else {
            File::create(file_path)?
        };

        file.seek(SeekFrom::Start(chunk.offset))?;
        file.write_all(data)?;
        file.sync_data()?;

        Ok(())
    }

    /// 计算数据的 SHA256 哈希值
    /// 
    /// # Arguments
    /// * `data` - 要计算哈希的数据
    /// 
    /// # Returns
    /// * `String` - 十六进制格式的哈希值
    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// 计算整个文件的哈希值
    /// 
    /// # Arguments
    /// * `file_path` - 文件路径
    /// 
    /// # Returns
    /// * `TransferResult<String>` - 十六进制格式的哈希值
    pub fn compute_file_hash(&self, file_path: &Path) -> TransferResult<String> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// 为文件元数据计算并设置所有分块的哈希值
    /// 
    /// # Arguments
    /// * `metadata` - 文件元数据（会被修改）
    /// * `file_path` - 文件路径
    /// 
    /// # Returns
    /// * `TransferResult<FileMetadata>` - 包含哈希值的元数据
    pub fn compute_metadata_with_hashes(
        &self,
        mut metadata: FileMetadata,
        file_path: &Path,
    ) -> TransferResult<FileMetadata> {
        // 计算文件总哈希
        metadata.hash = self.compute_file_hash(file_path)?;
        
        // 计算每个分块的哈希
        metadata.chunks = self.compute_chunks(file_path)?;
        for chunk in &mut metadata.chunks {
            let data = self.read_chunk(file_path, chunk)?;
            chunk.hash = Self::compute_hash(&data);
        }

        Ok(metadata)
    }

    /// 获取分块大小
    pub fn chunk_size(&self) -> u64 {
        self.chunk_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_chunks() {
        let chunker = FileChunker::new(100);
        let mut temp_file = NamedTempFile::new().unwrap();
        
        // 写入 250 字节
        temp_file.write_all(&[0u8; 250]).unwrap();
        temp_file.flush().unwrap();

        let chunks = chunker.compute_chunks(temp_file.path()).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].size, 100);
        assert_eq!(chunks[1].size, 100);
        assert_eq!(chunks[2].size, 50);
    }

    #[test]
    fn test_compute_hash() {
        let data = b"hello world";
        let hash = FileChunker::compute_hash(data);
        assert_eq!(hash.len(), 64); // SHA256 产生 64 个十六进制字符
    }
}