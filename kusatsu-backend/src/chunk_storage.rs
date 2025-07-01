use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

/// Manages temporary storage of file chunks during upload
#[derive(Clone)]
pub struct ChunkStorage {
    chunks_root: PathBuf,
}

impl ChunkStorage {
    /// Create a new chunk storage instance
    pub fn new(storage_root: impl AsRef<Path>) -> Self {
        Self {
            chunks_root: storage_root.as_ref().join("chunks"),
        }
    }

    /// Initialize the chunk storage directory
    pub async fn init(&self) -> Result<()> {
        if !self.chunks_root.exists() {
            fs::create_dir_all(&self.chunks_root).await.map_err(|e| {
                AppError::ServerError(format!("Failed to create chunks directory: {}", e))
            })?;
            tracing::info!(
                "📁 Created chunks directory: {}",
                self.chunks_root.display()
            );
        }
        Ok(())
    }

    /// Generate path for a specific chunk
    fn get_chunk_path(&self, upload_id: Uuid, chunk_number: i32) -> PathBuf {
        let upload_dir = self.chunks_root.join(upload_id.to_string());
        upload_dir.join(format!("chunk_{:06}", chunk_number))
    }

    /// Get directory for all chunks of an upload
    fn get_upload_dir(&self, upload_id: Uuid) -> PathBuf {
        self.chunks_root.join(upload_id.to_string())
    }

    /// Store a chunk to disk
    pub async fn store_chunk(
        &self,
        upload_id: Uuid,
        chunk_number: i32,
        chunk_data: &[u8],
    ) -> Result<()> {
        let chunk_path = self.get_chunk_path(upload_id, chunk_number);

        // Create upload directory if it doesn't exist
        if let Some(parent) = chunk_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                AppError::ServerError(format!("Failed to create upload directory: {}", e))
            })?;
        }

        // Write chunk data
        fs::write(&chunk_path, chunk_data)
            .await
            .map_err(|e| AppError::ServerError(format!("Failed to write chunk: {}", e)))?;

        tracing::debug!(
            "💾 Stored chunk {}/{} ({} bytes)",
            upload_id,
            chunk_number,
            chunk_data.len()
        );

        Ok(())
    }

    /// Check if a specific chunk exists
    pub async fn chunk_exists(&self, upload_id: Uuid, chunk_number: i32) -> bool {
        let chunk_path = self.get_chunk_path(upload_id, chunk_number);
        chunk_path.exists()
    }

    /// Get the size of a specific chunk
    pub async fn get_chunk_size(&self, upload_id: Uuid, chunk_number: i32) -> Result<u64> {
        let chunk_path = self.get_chunk_path(upload_id, chunk_number);
        let metadata = fs::metadata(&chunk_path)
            .await
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => AppError::FileNotFound,
                _ => AppError::ServerError(format!("Failed to get chunk metadata: {}", e)),
            })?;
        Ok(metadata.len())
    }

    /// Assemble all chunks into a single file and return the data
    pub async fn assemble_chunks(&self, upload_id: Uuid, total_chunks: i32) -> Result<Vec<u8>> {
        let mut assembled_data = Vec::new();

        for chunk_number in 0..total_chunks {
            let chunk_path = self.get_chunk_path(upload_id, chunk_number);

            if !chunk_path.exists() {
                return Err(AppError::BadRequest(format!(
                    "Missing chunk {} for upload {}",
                    chunk_number, upload_id
                )));
            }

            let chunk_data = fs::read(&chunk_path).await.map_err(|e| {
                AppError::ServerError(format!("Failed to read chunk {}: {}", chunk_number, e))
            })?;

            assembled_data.extend_from_slice(&chunk_data);
        }

        tracing::info!(
            "🔧 Assembled {} chunks into {} bytes for upload {}",
            total_chunks,
            assembled_data.len(),
            upload_id
        );

        Ok(assembled_data)
    }

    /// Delete all chunks for an upload (cleanup)
    pub async fn cleanup_upload(&self, upload_id: Uuid) -> Result<()> {
        let upload_dir = self.get_upload_dir(upload_id);

        if upload_dir.exists() {
            fs::remove_dir_all(&upload_dir).await.map_err(|e| {
                AppError::ServerError(format!("Failed to cleanup upload chunks: {}", e))
            })?;

            tracing::debug!("🧹 Cleaned up chunks for upload {}", upload_id);
        }

        Ok(())
    }

    /// Get list of uploaded chunks for an upload
    pub async fn get_uploaded_chunks(&self, upload_id: Uuid) -> Result<Vec<i32>> {
        let upload_dir = self.get_upload_dir(upload_id);

        if !upload_dir.exists() {
            return Ok(Vec::new());
        }

        let mut chunks = Vec::new();
        let mut entries = fs::read_dir(&upload_dir).await.map_err(|e| {
            AppError::ServerError(format!("Failed to read upload directory: {}", e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::ServerError(format!("Failed to read directory entry: {}", e)))?
        {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if let Some(chunk_str) = file_name_str.strip_prefix("chunk_") {
                if let Ok(chunk_number) = chunk_str.parse::<i32>() {
                    chunks.push(chunk_number);
                }
            }
        }

        chunks.sort();
        Ok(chunks)
    }

    /// Cleanup expired upload sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<u64> {
        let mut cleanup_count = 0u64;

        if !self.chunks_root.exists() {
            return Ok(0);
        }

        let mut entries = fs::read_dir(&self.chunks_root).await.map_err(|e| {
            AppError::ServerError(format!("Failed to read chunks directory: {}", e))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::ServerError(format!("Failed to read directory entry: {}", e)))?
        {
            if entry
                .file_type()
                .await
                .map_err(|e| AppError::ServerError(format!("Failed to get file type: {}", e)))?
                .is_dir()
            {
                // Check if directory is older than 2 hours (generous cleanup)
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(created) = metadata.created() {
                        let created_time = created
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();

                        if now.as_secs() - created_time.as_secs() > 2 * 3600 {
                            if fs::remove_dir_all(entry.path()).await.is_ok() {
                                cleanup_count += 1;
                                tracing::debug!(
                                    "🧹 Cleaned up expired chunk directory: {:?}",
                                    entry.path()
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(cleanup_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_chunk_storage() {
        let temp_dir = TempDir::new().unwrap();
        let chunk_storage = ChunkStorage::new(temp_dir.path());
        chunk_storage.init().await.unwrap();

        let upload_id = Uuid::new_v4();
        let chunk_data1 = b"Hello, ";
        let chunk_data2 = b"World!";

        // Store chunks
        chunk_storage
            .store_chunk(upload_id, 0, chunk_data1)
            .await
            .unwrap();
        chunk_storage
            .store_chunk(upload_id, 1, chunk_data2)
            .await
            .unwrap();

        // Check chunks exist
        assert!(chunk_storage.chunk_exists(upload_id, 0).await);
        assert!(chunk_storage.chunk_exists(upload_id, 1).await);
        assert!(!chunk_storage.chunk_exists(upload_id, 2).await);

        // Get uploaded chunks
        let uploaded_chunks = chunk_storage.get_uploaded_chunks(upload_id).await.unwrap();
        assert_eq!(uploaded_chunks, vec![0, 1]);

        // Assemble chunks
        let assembled = chunk_storage.assemble_chunks(upload_id, 2).await.unwrap();
        assert_eq!(assembled, b"Hello, World!");

        // Cleanup
        chunk_storage.cleanup_upload(upload_id).await.unwrap();
        assert!(!chunk_storage.chunk_exists(upload_id, 0).await);
    }
}
