use gloo::net::http::Request;
use uuid::Uuid;
use web_sys::FormData;

// Re-export shared types
pub use kusatsu_types::*;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },
}

// All API types are now defined in kusatsu-types and re-exported above

#[derive(Clone)]
pub struct ApiClient {
    pub base_url: String,
}

impl ApiClient {
    pub fn new() -> Self {
        let base_url = option_env!("KUSATSU_API_URL")
            .unwrap_or("http://localhost:3000")
            .to_string();

        Self { base_url }
    }

    // Get file info with optional encryption key (to handle both encrypted and unencrypted files)
    pub async fn get_file_info(
        &self,
        file_id: &str,
        encryption_key: Option<&str>,
    ) -> Result<FileInfo, ApiError> {
        let url = format!("{}/api/files/{}/info", self.base_url, file_id);

        let request = if let Some(encryption_key) = encryption_key {
            DownloadRequest {
                encryption_key: Some(encryption_key.to_string()),
            }
        } else {
            DownloadRequest {
                encryption_key: None,
            }
        };

        let response = Request::post(&url)
            .json(&request)
            .map_err(|e| ApiError::Network(format!("Failed to create request: {:?}", e)))?
            .send()
            .await
            .map_err(|e| ApiError::Network(format!("Request failed: {:?}", e)))?;

        if !response.ok() {
            let status = response.status();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiError::Server { status, message });
        }

        response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse response: {:?}", e)))
    }

    pub async fn upload_file(
        &self,
        file_data: Vec<u8>,
        filename: String,
        mime_type: Option<String>,
        expires_in_hours: Option<i32>,
        max_downloads: Option<i32>,
    ) -> Result<UploadResponse, ApiError> {
        let form_data = FormData::new()
            .map_err(|e| ApiError::Network(format!("Failed to create form data: {:?}", e)))?;

        // Add file data as blob
        let file_data_array = js_sys::Uint8Array::from(&file_data[..]);

        // Create blob from array
        use web_sys::Blob;
        let file_blob = Blob::new_with_u8_array_sequence(&js_sys::Array::of1(&file_data_array))
            .map_err(|e| ApiError::Network(format!("Failed to create file blob: {:?}", e)))?;

        form_data
            .append_with_blob("file_data", &file_blob)
            .map_err(|e| ApiError::Network(format!("Failed to append file_data: {:?}", e)))?;

        // Add filename
        form_data
            .append_with_str("filename", &filename)
            .map_err(|e| ApiError::Network(format!("Failed to append filename: {:?}", e)))?;

        // Add mime type if provided
        if let Some(mime) = mime_type {
            form_data
                .append_with_str("mime_type", &mime)
                .map_err(|e| ApiError::Network(format!("Failed to append mime_type: {:?}", e)))?;
        }

        // Build URL with query parameters
        let mut url = format!("{}/api/upload", self.base_url);
        let mut params = Vec::new();

        if let Some(hours) = expires_in_hours {
            params.push(format!("expires_in_hours={}", hours));
        }

        if let Some(max_dl) = max_downloads {
            params.push(format!("max_downloads={}", max_dl));
        }

        if !params.is_empty() {
            url.push_str("?");
            url.push_str(&params.join("&"));
        }

        let response = Request::post(&url)
            .body(form_data)
            .map_err(|e| ApiError::Network(format!("Failed to create request: {:?}", e)))?
            .send()
            .await
            .map_err(|e| ApiError::Network(format!("Request failed: {:?}", e)))?;

        if !response.ok() {
            let status = response.status();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiError::Server { status, message });
        }

        response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse response: {:?}", e)))
    }

    // Chunked upload methods
    pub async fn start_chunked_upload(
        &self,
        request: StartUploadRequest,
    ) -> Result<StartUploadResponse, ApiError> {
        let url = format!("{}/api/upload/start", self.base_url);

        let response = Request::post(&url)
            .json(&request)
            .map_err(|e| ApiError::Network(format!("Failed to create request: {:?}", e)))?
            .send()
            .await
            .map_err(|e| ApiError::Network(format!("Request failed: {:?}", e)))?;

        if !response.ok() {
            let status = response.status();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiError::Server { status, message });
        }

        response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse response: {:?}", e)))
    }

    pub async fn upload_chunk(
        &self,
        upload_id: &str,
        chunk_number: i32,
        chunk_data: &[u8],
    ) -> Result<ChunkUploadResponse, ApiError> {
        let url = format!(
            "{}/api/upload/chunk/{}/{}",
            self.base_url, upload_id, chunk_number
        );

        let form_data = FormData::new()
            .map_err(|e| ApiError::Network(format!("Failed to create form data: {:?}", e)))?;

        // Add chunk data as blob
        let chunk_data_array = js_sys::Uint8Array::from(chunk_data);
        use web_sys::Blob;
        let chunk_blob =
            Blob::new_with_u8_array_sequence(&js_sys::Array::of1(&chunk_data_array))
                .map_err(|e| ApiError::Network(format!("Failed to create chunk blob: {:?}", e)))?;

        form_data
            .append_with_blob("chunk", &chunk_blob)
            .map_err(|e| ApiError::Network(format!("Failed to append chunk: {:?}", e)))?;

        let response = Request::post(&url)
            .body(form_data)
            .map_err(|e| ApiError::Network(format!("Failed to create request: {:?}", e)))?
            .send()
            .await
            .map_err(|e| ApiError::Network(format!("Request failed: {:?}", e)))?;

        if !response.ok() {
            let status = response.status();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiError::Server { status, message });
        }

        response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse response: {:?}", e)))
    }

    pub async fn complete_chunked_upload(
        &self,
        upload_id: &str,
    ) -> Result<UploadResponse, ApiError> {
        let url = format!("{}/api/upload/complete", self.base_url);

        let upload_uuid = Uuid::parse_str(upload_id)
            .map_err(|_| ApiError::Network("Invalid upload ID format".to_string()))?;

        let request = CompleteUploadRequest {
            upload_id: upload_uuid,
        };

        let response = Request::post(&url)
            .json(&request)
            .map_err(|e| ApiError::Network(format!("Failed to create request: {:?}", e)))?
            .send()
            .await
            .map_err(|e| ApiError::Network(format!("Request failed: {:?}", e)))?;

        if !response.ok() {
            let status = response.status();
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ApiError::Server { status, message });
        }

        response
            .json()
            .await
            .map_err(|e| ApiError::Parse(format!("Failed to parse response: {:?}", e)))
    }
}
