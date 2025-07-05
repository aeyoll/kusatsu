use crate::error::Result;
use kusatsu_migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;
use uuid::Uuid;

// Parameter structs for database operations
#[derive(Debug)]
pub struct CreateFileParams {
    pub file_id: Uuid,
    pub original_size: i64,
    pub encrypted_size: i64,
    pub mime_type: Option<String>,
    pub file_path: String,
    pub nonce: Vec<u8>,
    pub encrypted_filename: Vec<u8>,
    pub filename_nonce: Vec<u8>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub max_downloads: Option<i32>,
}

#[derive(Debug)]
pub struct CreateUnencryptedFileParams {
    pub file_id: Uuid,
    pub original_size: i64,
    pub mime_type: Option<String>,
    pub file_path: String,
    pub filename: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub max_downloads: Option<i32>,
}

#[derive(Debug)]
pub struct CreateUploadSessionParams {
    pub upload_id: Uuid,
    pub filename: String,
    pub mime_type: Option<String>,
    pub total_size: i64,
    pub total_chunks: i32,
    pub chunk_size: i32,
    pub expires_in_hours: Option<i32>,
    pub max_downloads: Option<i32>,
}

pub async fn setup_database(database_url: &str) -> Result<DatabaseConnection> {
    tracing::info!("🔗 Connecting to database: {}", database_url);

    // Configure connection options
    let mut opt = ConnectOptions::new(database_url.to_string());
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(true);

    // Connect to database
    let db = Database::connect(opt).await?;

    // Run migrations
    tracing::info!("🔄 Running database migrations...");
    Migrator::up(&db, None).await?;
    tracing::info!("✅ Migrations completed successfully");

    Ok(db)
}

// Helper functions for file operations
pub mod file_ops {
    use super::*;
    use kusatsu_entity::{file, prelude::*};
    use sea_orm::*;
    use uuid::Uuid;

    pub async fn create_file_record(
        db: &DatabaseConnection,
        params: CreateFileParams,
    ) -> Result<file::Model> {
        let file_model = file::ActiveModel {
            file_id: Set(params.file_id),
            original_size: Set(params.original_size),
            encrypted_size: Set(params.encrypted_size),
            mime_type: Set(params.mime_type),
            file_path: Set(params.file_path),
            nonce: Set(params.nonce),
            encrypted_filename: Set(params.encrypted_filename),
            filename_nonce: Set(params.filename_nonce),
            expires_at: Set(params.expires_at),
            max_downloads: Set(params.max_downloads),
            ..Default::default()
        };

        let file = file_model.insert(db).await?;
        Ok(file)
    }

    // Create file record for unencrypted files (chunked uploads)
    pub async fn create_unencrypted_file_record(
        db: &DatabaseConnection,
        params: CreateUnencryptedFileParams,
    ) -> Result<file::Model> {
        let file_model = file::ActiveModel {
            file_id: Set(params.file_id),
            original_size: Set(params.original_size),
            encrypted_size: Set(params.original_size), // Same as original for unencrypted files
            mime_type: Set(params.mime_type),
            file_path: Set(params.file_path),
            nonce: Set(Vec::new()), // Empty nonce indicates unencrypted file
            encrypted_filename: Set(params.filename.as_bytes().to_vec()), // Store plain filename
            filename_nonce: Set(Vec::new()), // Empty nonce for filename
            expires_at: Set(params.expires_at),
            max_downloads: Set(params.max_downloads),
            ..Default::default()
        };

        let file = file_model.insert(db).await?;
        Ok(file)
    }

    pub async fn get_file_by_id(
        db: &DatabaseConnection,
        file_id: Uuid,
    ) -> Result<Option<file::Model>> {
        let file = File::find()
            .filter(file::Column::FileId.eq(file_id))
            .one(db)
            .await?;

        Ok(file)
    }

    pub async fn increment_download_count(db: &DatabaseConnection, file_id: Uuid) -> Result<()> {
        let file = File::find()
            .filter(file::Column::FileId.eq(file_id))
            .one(db)
            .await?;

        if let Some(file) = file {
            let mut file: file::ActiveModel = file.into();
            file.download_count = Set(file.download_count.unwrap() + 1);
            file.update(db).await?;
        }

        Ok(())
    }

    pub async fn cleanup_expired_files(
        db: &DatabaseConnection,
        storage: &crate::storage::FileStorage,
    ) -> Result<u64> {
        let now = chrono::Utc::now();

        // Get expired files first so we can delete them from storage
        let expired_files = File::find()
            .filter(file::Column::ExpiresAt.lt(now))
            .all(db)
            .await?;

        // Delete files from storage
        for file in &expired_files {
            if let Err(e) = storage.delete_file(&file.file_path).await {
                tracing::warn!(
                    "Failed to delete file from storage: {} - {}",
                    file.file_path,
                    e
                );
            }
        }

        // Delete from database
        let result = File::delete_many()
            .filter(file::Column::ExpiresAt.lt(now))
            .exec(db)
            .await?;

        Ok(result.rows_affected)
    }

    pub async fn delete_file_by_id(
        db: &DatabaseConnection,
        storage: &crate::storage::FileStorage,
        file_id: Uuid,
    ) -> Result<bool> {
        let file = File::find()
            .filter(file::Column::FileId.eq(file_id))
            .one(db)
            .await?;

        if let Some(file) = file {
            // Delete from storage first
            if let Err(e) = storage.delete_file(&file.file_path).await {
                tracing::warn!(
                    "Failed to delete file from storage: {} - {}",
                    file.file_path,
                    e
                );
            }

            // Delete from database
            File::delete_by_id(file.id).exec(db).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

// Helper functions for upload session operations
pub mod upload_session_ops {
    use super::*;
    use kusatsu_entity::{prelude::*, upload_session};
    use sea_orm::*;
    use uuid::Uuid;

    pub async fn create_upload_session(
        db: &DatabaseConnection,
        params: CreateUploadSessionParams,
    ) -> Result<upload_session::Model> {
        let session_model = upload_session::ActiveModel {
            upload_id: Set(params.upload_id),
            filename: Set(params.filename),
            mime_type: Set(params.mime_type),
            total_size: Set(params.total_size),
            total_chunks: Set(params.total_chunks),
            chunk_size: Set(params.chunk_size),
            expires_in_hours: Set(params.expires_in_hours),
            max_downloads: Set(params.max_downloads),
            ..Default::default()
        };

        let session = session_model.insert(db).await?;
        Ok(session)
    }

    pub async fn get_upload_session_by_id(
        db: &DatabaseConnection,
        upload_id: Uuid,
    ) -> Result<Option<upload_session::Model>> {
        let session = UploadSession::find()
            .filter(upload_session::Column::UploadId.eq(upload_id))
            .one(db)
            .await?;

        Ok(session)
    }

    pub async fn increment_uploaded_chunks(
        db: &DatabaseConnection,
        upload_id: Uuid,
    ) -> Result<upload_session::Model> {
        let session = UploadSession::find()
            .filter(upload_session::Column::UploadId.eq(upload_id))
            .one(db)
            .await?
            .ok_or_else(|| {
                crate::error::AppError::BadRequest("Upload session not found".to_string())
            })?;

        let mut session: upload_session::ActiveModel = session.into();
        session.uploaded_chunks = Set(session.uploaded_chunks.unwrap() + 1);
        let updated_session = session.update(db).await?;

        Ok(updated_session)
    }

    pub async fn delete_upload_session(db: &DatabaseConnection, upload_id: Uuid) -> Result<bool> {
        let result = UploadSession::delete_many()
            .filter(upload_session::Column::UploadId.eq(upload_id))
            .exec(db)
            .await?;

        Ok(result.rows_affected > 0)
    }

    pub async fn cleanup_expired_upload_sessions(
        db: &DatabaseConnection,
        chunk_storage: &crate::chunk_storage::ChunkStorage,
    ) -> Result<u64> {
        let now = chrono::Utc::now();

        // Get expired sessions first so we can clean up their chunks
        let expired_sessions = UploadSession::find()
            .filter(upload_session::Column::ExpiresAt.lt(now))
            .all(db)
            .await?;

        // Clean up chunks for expired sessions
        for session in &expired_sessions {
            if let Err(e) = chunk_storage.cleanup_upload(session.upload_id).await {
                tracing::warn!(
                    "Failed to cleanup chunks for upload {}: {}",
                    session.upload_id,
                    e
                );
            }
        }

        // Delete from database
        let result = UploadSession::delete_many()
            .filter(upload_session::Column::ExpiresAt.lt(now))
            .exec(db)
            .await?;

        Ok(result.rows_affected)
    }
}
