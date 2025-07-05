use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Request types
#[derive(Serialize, Deserialize, Clone)]
pub struct StartUploadRequest {
    pub filename: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
    pub chunk_size: Option<i32>,
    pub expires_in_hours: Option<i32>,
    pub max_downloads: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CompleteUploadRequest {
    pub upload_id: Uuid,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DownloadRequest {
    pub encryption_key: Option<String>,
}

#[derive(Deserialize)]
pub struct UploadOptions {
    pub expires_in_hours: Option<i32>,
    pub max_downloads: Option<i32>,
}

// Response types
#[derive(Serialize, Deserialize, Clone)]
pub struct UploadResponse {
    pub file_id: Uuid,
    pub download_url: String,
    pub encryption_key: Option<String>,
    pub curl_command: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StartUploadResponse {
    pub upload_id: Uuid,
    pub chunk_size: i32,
    pub total_chunks: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChunkUploadResponse {
    pub chunk_number: i32,
    pub uploaded_chunks: i32,
    pub total_chunks: i32,
    pub progress: f32,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct FileInfo {
    pub file_id: Uuid,
    pub original_size: i64,
    pub encrypted_size: i64,
    pub mime_type: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub download_count: i32,
    pub max_downloads: Option<i32>,
    pub filename: String,
    pub is_encrypted: bool,
}
