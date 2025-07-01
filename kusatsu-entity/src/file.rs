use sea_orm::entity::prelude::*;
use sea_orm::Set;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "files")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[serde(skip_deserializing)]
    pub id: i32,

    /// Unique identifier for the file (used in URLs)
    #[sea_orm(unique)]
    pub file_id: Uuid,

    /// Original file size in bytes
    pub original_size: i64,

    /// Encrypted file size in bytes
    pub encrypted_size: i64,

    /// MIME type if detected
    pub mime_type: Option<String>,

    /// Path to the encrypted file on disk
    pub file_path: String,

    /// Nonce used for file encryption
    pub nonce: Vec<u8>,

    /// Encrypted filename
    pub encrypted_filename: Vec<u8>,

    /// Nonce used for filename encryption
    pub filename_nonce: Vec<u8>,

    /// When the file was uploaded
    pub created_at: ChronoDateTimeUtc,

    /// When the file expires (optional)
    pub expires_at: Option<ChronoDateTimeUtc>,

    /// Download count
    #[sea_orm(default_value = 0)]
    pub download_count: i32,

    /// Maximum number of downloads allowed (optional)
    pub max_downloads: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            file_id: Set(Uuid::new_v4()),
            created_at: Set(chrono::Utc::now()),
            ..ActiveModelTrait::default()
        }
    }
}

impl Model {
    /// Check if the file has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    /// Check if the file has reached maximum downloads
    pub fn is_download_limit_reached(&self) -> bool {
        if let Some(max_downloads) = self.max_downloads {
            self.download_count >= max_downloads
        } else {
            false
        }
    }

    /// Check if the file is accessible (not expired and not over download limit)
    pub fn is_accessible(&self) -> bool {
        !self.is_expired() && !self.is_download_limit_reached()
    }
}
