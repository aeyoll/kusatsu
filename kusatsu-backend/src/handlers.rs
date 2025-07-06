use axum::{
    body::Body,
    extract::{Form, Multipart, Path, Query, State},
    http::{Response, StatusCode},
    response::{IntoResponse, Json},
};
use kusatsu_encrypt::{Encryption, EncryptionKey};
use uuid::Uuid;

use crate::{
    database::{file_ops, upload_session_ops},
    error::{AppError, Result},
    AppState, ChunkUploadResponse, CompleteUploadRequest, DownloadRequest, FileInfo,
    StartUploadRequest, StartUploadResponse, UploadOptions, UploadResponse,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct DownloadFormData {
    pub encryption_key: String,
}

#[derive(Serialize)]
pub struct CleanupResponse {
    pub items_cleaned: u64,
    pub cleanup_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// Default chunk size: 5MB
const DEFAULT_CHUNK_SIZE: i32 = 5 * 1024 * 1024;

// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "kusatsu-backend",
        "timestamp": chrono::Utc::now()
    }))
}

// File upload endpoint - receives plaintext file data and encrypts server-side
pub async fn upload_file(
    State(state): State<AppState>,
    Query(options): Query<UploadOptions>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename: Option<String> = None;
    let mut mime_type: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart data".to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "file" | "file_data" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::BadRequest("Failed to read file data".to_string()))?;

                if data.len() > state.config.max_file_size {
                    return Err(AppError::FileTooLarge);
                }

                file_data = Some(data.to_vec());
            }
            "filename" => {
                let data = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("Failed to read filename".to_string()))?;
                if !data.is_empty() {
                    filename = Some(data);
                }
            }
            "mime_type" => {
                let data = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("Failed to read mime type".to_string()))?;
                if !data.is_empty() {
                    mime_type = Some(data);
                }
            }
            _ => {
                // Skip unknown fields
                let _ = field.bytes().await;
            }
        }
    }

    // Validate required fields
    let file_data =
        file_data.ok_or_else(|| AppError::BadRequest("Missing file data".to_string()))?;
    let filename = filename.ok_or_else(|| AppError::BadRequest("Missing filename".to_string()))?;

    let original_size = file_data.len() as i64;

    // Generate encryption key and encrypt the file server-side
    let encryption_key = EncryptionKey::generate();

    // Encrypt file content
    let encrypted_file_data = Encryption::encrypt(&file_data, &encryption_key)
        .map_err(|e| AppError::ServerError(format!("Failed to encrypt file: {}", e)))?;

    // Encrypt filename
    let encrypted_filename_data = Encryption::encrypt(filename.as_bytes(), &encryption_key)
        .map_err(|e| AppError::ServerError(format!("Failed to encrypt filename: {}", e)))?;

    // Calculate expiration time
    let expires_at = options
        .expires_in_hours
        .map(|hours| chrono::Utc::now() + chrono::Duration::hours(hours as i64));

    // Generate file ID
    let file_id = Uuid::new_v4();
    let encrypted_size = encrypted_file_data.ciphertext.len() as i64;

    // Store encrypted file to disk
    let file_path = state
        .storage
        .store_file(file_id, &encrypted_file_data.ciphertext)
        .await?;

    // Store file metadata in database
    let _file_record = file_ops::create_file_record(
        &state.db,
        crate::database::CreateFileParams {
            file_id,
            original_size,
            encrypted_size,
            mime_type,
            file_path,
            nonce: encrypted_file_data.nonce,
            encrypted_filename: encrypted_filename_data.ciphertext,
            filename_nonce: encrypted_filename_data.nonce,
            expires_at,
            max_downloads: options.max_downloads,
        },
    )
    .await?;

    // Encode encryption key for return to client
    let encoded_key = encryption_key.to_base64();

    // Generate download URL
    let download_url = format!(
        "{}/download/{}#{}",
        state.config.base_url, file_id, encoded_key
    );

    // Generate curl command
    let curl_command = format!(
        "curl -X POST -JLO --fail -d 'encryption_key={}' {}/api/files/{}/form",
        encoded_key, state.config.api_url, file_id
    );

    tracing::info!(
        "📁 File uploaded and encrypted server-side: {} ({} bytes -> {} bytes encrypted)",
        file_id,
        original_size,
        encrypted_size
    );

    Ok(Json(UploadResponse {
        file_id,
        download_url,
        encryption_key: Some(encoded_key),
        curl_command,
    }))
}

// Start chunked upload
pub async fn start_chunked_upload(
    State(state): State<AppState>,
    Json(request): Json<StartUploadRequest>,
) -> Result<Json<StartUploadResponse>> {
    // Validate file size
    if request.file_size > state.config.max_file_size as i64 {
        return Err(AppError::FileTooLarge);
    }

    if request.file_size <= 0 {
        return Err(AppError::BadRequest("Invalid file size".to_string()));
    }

    // Determine chunk size
    let chunk_size = request.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE);
    if chunk_size <= 0 || chunk_size > 50 * 1024 * 1024 {
        return Err(AppError::BadRequest("Invalid chunk size".to_string()));
    }

    // Calculate total chunks
    let total_chunks = ((request.file_size as f64) / (chunk_size as f64)).ceil() as i32;

    // Generate upload ID
    let upload_id = Uuid::new_v4();

    // Create upload session in database
    let _session = upload_session_ops::create_upload_session(
        &state.db,
        crate::database::CreateUploadSessionParams {
            upload_id,
            filename: request.filename,
            mime_type: request.mime_type,
            total_size: request.file_size,
            total_chunks,
            chunk_size,
            expires_in_hours: request.expires_in_hours,
            max_downloads: request.max_downloads,
        },
    )
    .await?;

    tracing::info!(
        "🚀 Started chunked upload: {} ({} bytes, {} chunks of {} bytes each)",
        upload_id,
        request.file_size,
        total_chunks,
        chunk_size
    );

    Ok(Json(StartUploadResponse {
        upload_id,
        chunk_size,
        total_chunks,
    }))
}

// Upload a chunk
pub async fn upload_chunk(
    State(state): State<AppState>,
    Path((upload_id, chunk_number)): Path<(Uuid, i32)>,
    mut multipart: Multipart,
) -> Result<Json<ChunkUploadResponse>> {
    // Get upload session
    let session = upload_session_ops::get_upload_session_by_id(&state.db, upload_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Upload session not found".to_string()))?;

    // Check if session has expired
    if session.is_expired() {
        return Err(AppError::BadRequest(
            "Upload session has expired".to_string(),
        ));
    }

    // Check if upload is already complete
    if session.is_complete() {
        return Err(AppError::BadRequest(
            "Upload is already complete".to_string(),
        ));
    }

    // Validate chunk number
    if chunk_number < 0 || chunk_number >= session.total_chunks {
        return Err(AppError::BadRequest("Invalid chunk number".to_string()));
    }

    // Check if this chunk was already uploaded
    if state
        .chunk_storage
        .chunk_exists(upload_id, chunk_number)
        .await
    {
        tracing::warn!(
            "Chunk {} for upload {} already exists, skipping",
            chunk_number,
            upload_id
        );

        return Ok(Json(ChunkUploadResponse {
            chunk_number,
            uploaded_chunks: session.uploaded_chunks,
            total_chunks: session.total_chunks,
            progress: session.progress(),
        }));
    }

    // Extract chunk data from multipart
    let mut chunk_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("Invalid multipart data".to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "chunk" {
            let data = field
                .bytes()
                .await
                .map_err(|_| AppError::BadRequest("Failed to read chunk data".to_string()))?;

            chunk_data = Some(data.to_vec());
            break;
        }
    }

    let chunk_data =
        chunk_data.ok_or_else(|| AppError::BadRequest("Missing chunk data".to_string()))?;

    // Validate chunk size (last chunk can be smaller)
    let expected_size = if chunk_number == session.total_chunks - 1 {
        // Last chunk: calculate remaining bytes
        let remaining = session.total_size - (chunk_number as i64 * session.chunk_size as i64);
        remaining as usize
    } else {
        session.chunk_size as usize
    };

    if chunk_data.len() != expected_size {
        return Err(AppError::BadRequest(format!(
            "Invalid chunk size: expected {}, got {}",
            expected_size,
            chunk_data.len()
        )));
    }

    // Store chunk
    state
        .chunk_storage
        .store_chunk(upload_id, chunk_number, &chunk_data)
        .await?;

    // Update session (increment uploaded chunks)
    let updated_session =
        upload_session_ops::increment_uploaded_chunks(&state.db, upload_id).await?;

    tracing::debug!(
        "📦 Uploaded chunk {}/{} for upload {} ({}/{} chunks complete)",
        chunk_number,
        session.total_chunks - 1,
        upload_id,
        updated_session.uploaded_chunks,
        updated_session.total_chunks
    );

    Ok(Json(ChunkUploadResponse {
        chunk_number,
        uploaded_chunks: updated_session.uploaded_chunks,
        total_chunks: updated_session.total_chunks,
        progress: updated_session.progress(),
    }))
}

// Complete chunked upload
pub async fn complete_chunked_upload(
    State(state): State<AppState>,
    Json(request): Json<CompleteUploadRequest>,
) -> Result<Json<UploadResponse>> {
    // Get upload session
    let session = upload_session_ops::get_upload_session_by_id(&state.db, request.upload_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Upload session not found".to_string()))?;

    // Check if session has expired
    if session.is_expired() {
        return Err(AppError::BadRequest(
            "Upload session has expired".to_string(),
        ));
    }

    // Check if all chunks have been uploaded
    if !session.is_complete() {
        return Err(AppError::BadRequest(format!(
            "Upload incomplete: {}/{} chunks uploaded",
            session.uploaded_chunks, session.total_chunks
        )));
    }

    // Assemble chunks into complete file
    let assembled_data = state
        .chunk_storage
        .assemble_chunks(request.upload_id, session.total_chunks)
        .await?;

    // Verify assembled file size matches expected size
    if assembled_data.len() != session.total_size as usize {
        return Err(AppError::ServerError(format!(
            "Assembled file size mismatch: expected {}, got {}",
            session.total_size,
            assembled_data.len()
        )));
    }

    // Calculate expiration time
    let expires_at = session
        .expires_in_hours
        .map(|hours| chrono::Utc::now() + chrono::Duration::hours(hours as i64));

    // Generate file ID
    let file_id = Uuid::new_v4();

    // Store unencrypted file to disk (chunked uploads are not encrypted)
    let file_path = state.storage.store_file(file_id, &assembled_data).await?;

    // Store file metadata in database (unencrypted)
    let _file_record = file_ops::create_unencrypted_file_record(
        &state.db,
        crate::database::CreateUnencryptedFileParams {
            file_id,
            original_size: session.total_size,
            mime_type: session.mime_type,
            file_path,
            filename: session.filename.clone(),
            expires_at,
            max_downloads: session.max_downloads,
        },
    )
    .await?;

    // Clean up chunks and upload session
    if let Err(e) = state.chunk_storage.cleanup_upload(request.upload_id).await {
        tracing::warn!(
            "Failed to cleanup chunks for upload {}: {}",
            request.upload_id,
            e
        );
    }

    if let Err(e) = upload_session_ops::delete_upload_session(&state.db, request.upload_id).await {
        tracing::warn!(
            "Failed to delete upload session {}: {}",
            request.upload_id,
            e
        );
    }

    // Generate download URL (no encryption key needed for chunked uploads)
    let download_url = format!("{}/download/{}", state.config.base_url, file_id);

    tracing::info!(
        "✅ Completed chunked upload: {} -> {} ({} bytes unencrypted)",
        request.upload_id,
        file_id,
        session.total_size
    );

    let curl_command = format!(
        "curl -X POST -JLO --fail -d \"encryption_key=\" {}/api/files/{}/form",
        state.config.api_url, file_id
    );

    Ok(Json(UploadResponse {
        file_id,
        download_url,
        encryption_key: None,
        curl_command,
    }))
}

// Get upload status
pub async fn get_upload_status(
    State(state): State<AppState>,
    Path(upload_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    // Get upload session
    let session = upload_session_ops::get_upload_session_by_id(&state.db, upload_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("Upload session not found".to_string()))?;

    // Get list of uploaded chunks
    let uploaded_chunk_numbers = state.chunk_storage.get_uploaded_chunks(upload_id).await?;

    Ok(Json(serde_json::json!({
        "upload_id": upload_id,
        "filename": session.filename,
        "total_size": session.total_size,
        "total_chunks": session.total_chunks,
        "uploaded_chunks": session.uploaded_chunks,
        "uploaded_chunk_numbers": uploaded_chunk_numbers,
        "progress": session.progress(),
        "is_complete": session.is_complete(),
        "is_expired": session.is_expired(),
        "created_at": session.created_at,
        "expires_at": session.expires_at
    })))
}

// Form-based file download endpoint - accepts form data with encryption key and streams file download
pub async fn download_file_form(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    Form(form_data): Form<DownloadFormData>,
) -> Result<impl IntoResponse> {
    // Get file from database
    let file = file_ops::get_file_by_id(&state.db, file_id)
        .await?
        .ok_or(AppError::FileNotFound)?;

    // Check if file is accessible
    if !file.is_accessible() {
        if file.is_expired() {
            return Err(AppError::FileExpired);
        } else if file.is_download_limit_reached() {
            return Err(AppError::DownloadLimitExceeded);
        }
    }

    // Check if file is encrypted (nonce is empty for unencrypted files)
    let is_encrypted = !file.nonce.is_empty();

    let (file_data, original_filename) = if is_encrypted {
        // Handle encrypted file (direct upload)
        let encryption_key = EncryptionKey::from_base64(&form_data.encryption_key)
            .map_err(|_| AppError::BadRequest("Invalid encryption key".to_string()))?;

        // Read encrypted file from disk
        let encrypted_file_data_bytes = state.storage.retrieve_file(&file.file_path).await?;

        // Reconstruct encrypted data structures
        let encrypted_file_data = kusatsu_encrypt::EncryptedData {
            ciphertext: encrypted_file_data_bytes,
            nonce: file.nonce.clone(),
        };

        let encrypted_filename_data = kusatsu_encrypt::EncryptedData {
            ciphertext: file.encrypted_filename.clone(),
            nonce: file.filename_nonce.clone(),
        };

        // Decrypt the file content
        let decrypted_data =
            Encryption::decrypt(&encrypted_file_data, &encryption_key).map_err(|_| {
                AppError::BadRequest("Failed to decrypt file - invalid encryption key".to_string())
            })?;

        // Decrypt the filename
        let decrypted_filename_bytes =
            Encryption::decrypt(&encrypted_filename_data, &encryption_key).map_err(|_| {
                AppError::BadRequest(
                    "Failed to decrypt filename - invalid encryption key".to_string(),
                )
            })?;

        let filename = String::from_utf8(decrypted_filename_bytes)
            .map_err(|_| AppError::ServerError("Invalid filename encoding".to_string()))?;

        (decrypted_data, filename)
    } else {
        // Handle unencrypted file (chunked upload)
        if !form_data.encryption_key.is_empty() {
            return Err(AppError::BadRequest(
                "This file is unencrypted and does not require an encryption key".to_string(),
            ));
        }

        // Read unencrypted file from disk
        let file_data = state.storage.retrieve_file(&file.file_path).await?;

        // Get plain filename (stored as bytes in encrypted_filename field)
        let filename = String::from_utf8(file.encrypted_filename.clone())
            .map_err(|_| AppError::ServerError("Invalid filename encoding".to_string()))?;

        (file_data, filename)
    };

    // Increment download count
    file_ops::increment_download_count(&state.db, file_id).await?;

    // Sanitize filename for Content-Disposition header
    let sanitized_filename = original_filename
        .replace('\"', "\\\"")
        .replace(['\n', '\r'], " ");

    // Build streaming response with proper headers for direct download
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(
            "Content-Type",
            file.mime_type
                .as_deref()
                .unwrap_or("application/octet-stream"),
        )
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", sanitized_filename),
        )
        .header("Content-Length", file_data.len().to_string())
        .header("X-File-ID", file_id.to_string())
        .header("X-Original-Size", file.original_size.to_string())
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        .header("Access-Control-Expose-Headers", "Content-Disposition")
        .body(Body::from(file_data))
        .map_err(|e| AppError::ServerError(format!("Failed to build streaming response: {}", e)))?;

    tracing::info!(
        "📥 File downloaded via form {}: {} -> {} (download #{})",
        if is_encrypted {
            "and decrypted"
        } else {
            "(unencrypted)"
        },
        file_id,
        original_filename,
        file.download_count + 1
    );

    Ok(response)
}

// Get file info endpoint - returns decrypted filename
pub async fn get_file_info(
    State(state): State<AppState>,
    Path(file_id): Path<Uuid>,
    Json(download_request): Json<DownloadRequest>,
) -> Result<Json<FileInfo>> {
    tracing::info!("Getting file info for file: {}", file_id);

    // Get file from database
    let file = file_ops::get_file_by_id(&state.db, file_id)
        .await?
        .ok_or(AppError::FileNotFound)?;

    tracing::info!("File found: {:?}", file);

    // Check if file is encrypted (nonce is empty for unencrypted files)
    let is_encrypted = !file.nonce.is_empty();

    tracing::info!("File is encrypted: {}", is_encrypted);

    let decrypted_filename = if is_encrypted {
        tracing::info!("Decrypting filename for encrypted file {}", file_id);
        // Handle encrypted file (direct upload)
        let encryption_key_str = download_request.encryption_key.as_ref().ok_or_else(|| {
            AppError::BadRequest("Encryption key required for encrypted file".to_string())
        })?;

        let encryption_key = EncryptionKey::from_url_encoded(encryption_key_str)
            .map_err(|_| AppError::BadRequest("Invalid encryption key".to_string()))?;

        // Decrypt the filename
        let encrypted_filename_data = kusatsu_encrypt::EncryptedData {
            ciphertext: file.encrypted_filename,
            nonce: file.filename_nonce,
        };

        let decrypted_filename_bytes =
            Encryption::decrypt(&encrypted_filename_data, &encryption_key).map_err(|_| {
                AppError::BadRequest(
                    "Failed to decrypt filename - invalid encryption key".to_string(),
                )
            })?;

        String::from_utf8(decrypted_filename_bytes)
            .map_err(|_| AppError::ServerError("Invalid filename encoding".to_string()))?
    } else {
        tracing::info!("Decrypting filename for unencrypted file {}", file_id);

        // Handle unencrypted file (chunked upload)
        if download_request.encryption_key.is_some() {
            return Err(AppError::BadRequest(
                "This file is unencrypted and does not require an encryption key".to_string(),
            ));
        }

        // Get plain filename (stored as bytes in encrypted_filename field)
        String::from_utf8(file.encrypted_filename)
            .map_err(|_| AppError::ServerError("Invalid filename encoding".to_string()))?
    };

    Ok(Json(FileInfo {
        file_id: file.file_id,
        original_size: file.original_size,
        encrypted_size: file.encrypted_size,
        mime_type: file.mime_type,
        created_at: file.created_at,
        expires_at: file.expires_at,
        download_count: file.download_count,
        max_downloads: file.max_downloads,
        filename: decrypted_filename,
        is_encrypted,
    }))
}

// Cleanup expired files endpoint
pub async fn cleanup_expired_files(State(state): State<AppState>) -> Result<Json<CleanupResponse>> {
    tracing::info!("🧹 Starting cleanup of expired files");

    let cleaned_count = file_ops::cleanup_expired_files(&state.db, &state.storage).await?;

    tracing::info!("✅ Cleaned up {} expired files", cleaned_count);

    Ok(Json(CleanupResponse {
        items_cleaned: cleaned_count,
        cleanup_type: "expired_files".to_string(),
        timestamp: chrono::Utc::now(),
    }))
}

// Cleanup expired upload sessions endpoint
pub async fn cleanup_expired_upload_sessions(
    State(state): State<AppState>,
) -> Result<Json<CleanupResponse>> {
    tracing::info!("🧹 Starting cleanup of expired upload sessions");

    let cleaned_count =
        upload_session_ops::cleanup_expired_upload_sessions(&state.db, &state.chunk_storage)
            .await?;

    tracing::info!("✅ Cleaned up {} expired upload sessions", cleaned_count);

    Ok(Json(CleanupResponse {
        items_cleaned: cleaned_count,
        cleanup_type: "expired_upload_sessions".to_string(),
        timestamp: chrono::Utc::now(),
    }))
}
