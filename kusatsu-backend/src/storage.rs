use crate::error::{AppError, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

/// File storage manager that handles storing and retrieving encrypted files
#[derive(Clone)]
pub struct FileStorage {
    storage_root: PathBuf,
}

impl FileStorage {
    /// Create a new file storage instance
    pub fn new(storage_root: impl AsRef<Path>) -> Self {
        Self {
            storage_root: storage_root.as_ref().to_path_buf(),
        }
    }

    /// Initialize the storage directory structure
    pub async fn init(&self) -> Result<()> {
        if !self.storage_root.exists() {
            fs::create_dir_all(&self.storage_root).await.map_err(|e| {
                AppError::ServerError(format!("Failed to create storage directory: {}", e))
            })?;
            tracing::info!(
                "📁 Created storage directory: {}",
                self.storage_root.display()
            );
        }
        Ok(())
    }

    /// Generate a performant file path for a given file ID
    /// Uses a hierarchical structure: storage/ab/cd/abcd1234-5678-9abc-def0-123456789abc.enc
    /// This distributes files across subdirectories to avoid filesystem performance issues
    pub fn generate_file_path(&self, file_id: Uuid) -> PathBuf {
        let file_id_str = file_id.to_string().replace('-', "");

        // Take first 4 characters for two-level directory structure
        // This gives us 16^4 = 65,536 possible directories
        let level1 = &file_id_str[0..2]; // First 2 chars (256 dirs)
        let level2 = &file_id_str[2..4]; // Next 2 chars (256 subdirs each)

        let filename = format!("{}.enc", file_id);

        self.storage_root.join(level1).join(level2).join(filename)
    }

    /// Store encrypted file data to disk
    pub async fn store_file(&self, file_id: Uuid, encrypted_data: &[u8]) -> Result<String> {
        let file_path = self.generate_file_path(file_id);

        // Create parent directories if they don't exist
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| AppError::ServerError(format!("Failed to create directory: {}", e)))?;
        }

        // Write encrypted data to file
        fs::write(&file_path, encrypted_data)
            .await
            .map_err(|e| AppError::ServerError(format!("Failed to write file: {}", e)))?;

        // Return relative path for database storage
        let relative_path = file_path
            .strip_prefix(&self.storage_root)
            .map_err(|e| AppError::ServerError(format!("Failed to get relative path: {}", e)))?
            .to_string_lossy()
            .to_string();

        tracing::debug!("💾 Stored file: {} -> {}", file_id, relative_path);
        Ok(relative_path)
    }

    /// Retrieve encrypted file data from disk
    pub async fn retrieve_file(&self, relative_path: &str) -> Result<Vec<u8>> {
        let file_path = self.storage_root.join(relative_path);

        let data = fs::read(&file_path).await.map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => AppError::FileNotFound,
            _ => AppError::ServerError(format!("Failed to read file: {}", e)),
        })?;

        tracing::debug!(
            "📖 Retrieved file: {} ({} bytes)",
            relative_path,
            data.len()
        );
        Ok(data)
    }

    /// Delete a file from disk
    pub async fn delete_file(&self, relative_path: &str) -> Result<()> {
        let file_path = self.storage_root.join(relative_path);

        match fs::remove_file(&file_path).await {
            Ok(_) => {
                tracing::debug!("🗑️  Deleted file: {}", relative_path);

                // Try to clean up empty parent directories
                Box::pin(self.cleanup_empty_dirs(&file_path)).await;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File already doesn't exist, that's fine
                Ok(())
            }
            Err(e) => Err(AppError::ServerError(format!(
                "Failed to delete file: {}",
                e
            ))),
        }
    }

    /// Clean up empty parent directories after file deletion
    fn cleanup_empty_dirs<'a>(
        &'a self,
        file_path: &'a Path,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            if let Some(parent) = file_path.parent() {
                // Only clean up directories within our storage root
                if parent.starts_with(&self.storage_root) && parent != self.storage_root {
                    if let Ok(mut entries) = fs::read_dir(parent).await {
                        // Check if directory is empty
                        if entries.next_entry().await.unwrap_or(None).is_none() {
                            if fs::remove_dir(parent).await.is_ok() {
                                tracing::debug!(
                                    "🧹 Cleaned up empty directory: {}",
                                    parent.display()
                                );
                                // Recursively clean up parent directories
                                Box::pin(self.cleanup_empty_dirs(parent)).await;
                            }
                        }
                    }
                }
            }
        })
    }

    /// Get storage statistics
    pub async fn get_stats(&self) -> Result<StorageStats> {
        let mut total_files = 0u64;
        let mut total_size = 0u64;

        fn count_files_recursive(
            dir: &Path,
            total_files: &mut u64,
            total_size: &mut u64,
        ) -> std::io::Result<()> {
            let entries = std::fs::read_dir(dir)?;

            for entry in entries {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    count_files_recursive(&path, total_files, total_size)?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("enc") {
                    *total_files += 1;
                    if let Ok(metadata) = entry.metadata() {
                        *total_size += metadata.len();
                    }
                }
            }
            Ok(())
        }

        if self.storage_root.exists() {
            count_files_recursive(&self.storage_root, &mut total_files, &mut total_size).map_err(
                |e| AppError::ServerError(format!("Failed to calculate storage stats: {}", e)),
            )?;
        }

        Ok(StorageStats {
            total_files,
            total_size,
        })
    }
}

#[derive(Debug)]
pub struct StorageStats {
    pub total_files: u64,
    pub total_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path());
        storage.init().await.unwrap();

        let file_id = Uuid::new_v4();
        let test_data = b"Hello, World!";

        // Test store
        let path = storage.store_file(file_id, test_data).await.unwrap();
        assert!(!path.is_empty());

        // Test retrieve
        let retrieved_data = storage.retrieve_file(&path).await.unwrap();
        assert_eq!(test_data, &retrieved_data[..]);

        // Test delete
        storage.delete_file(&path).await.unwrap();

        // Verify file is gone
        assert!(storage.retrieve_file(&path).await.is_err());
    }

    #[test]
    fn test_path_generation() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage::new(temp_dir.path());

        let file_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        let path = storage.generate_file_path(file_id);

        // Should create structure: 55/0e/550e8400-e29b-41d4-a716-446655440000.enc
        let expected_suffix = Path::new("55")
            .join("0e")
            .join("550e8400-e29b-41d4-a716-446655440000.enc");

        assert!(path.ends_with(&expected_suffix));
    }
}
