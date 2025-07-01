use sea_orm::entity::prelude::*;
use sea_orm::Set;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "upload_sessions")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i32,

    /// Unique identifier for the upload session
    #[sea_orm(unique)]
    pub upload_id: Uuid,

    /// Original filename
    pub filename: String,

    /// MIME type if detected
    pub mime_type: Option<String>,

    /// Total file size in bytes
    pub total_size: i64,

    /// Total number of chunks
    pub total_chunks: i32,

    /// Number of chunks uploaded so far
    #[sea_orm(default_value = 0)]
    pub uploaded_chunks: i32,

    /// Size of each chunk in bytes
    pub chunk_size: i32,

    /// Expiration time for the final file (optional)
    pub expires_in_hours: Option<i32>,

    /// Maximum number of downloads allowed (optional)
    pub max_downloads: Option<i32>,

    /// When the upload session was created
    pub created_at: ChronoDateTimeUtc,

    /// When the upload session expires (1 hour by default)
    pub expires_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            upload_id: Set(Uuid::new_v4()),
            created_at: Set(chrono::Utc::now()),
            expires_at: Set(chrono::Utc::now() + chrono::Duration::hours(1)),
            ..ActiveModelTrait::default()
        }
    }
}

impl Model {
    /// Check if the upload session has expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }

    /// Check if all chunks have been uploaded
    pub fn is_complete(&self) -> bool {
        self.uploaded_chunks >= self.total_chunks
    }

    /// Get upload progress as a percentage (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.total_chunks == 0 {
            0.0
        } else {
            self.uploaded_chunks as f32 / self.total_chunks as f32
        }
    }

    /// Get the next expected chunk number
    pub fn next_chunk_number(&self) -> i32 {
        self.uploaded_chunks
    }
}
