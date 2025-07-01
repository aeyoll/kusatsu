use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UploadSessions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UploadSessions::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UploadSessions::UploadId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(UploadSessions::Filename).string().not_null())
                    .col(ColumnDef::new(UploadSessions::MimeType).string())
                    .col(
                        ColumnDef::new(UploadSessions::TotalSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadSessions::TotalChunks)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UploadSessions::UploadedChunks)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(UploadSessions::ChunkSize)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UploadSessions::ExpiresInHours).integer())
                    .col(ColumnDef::new(UploadSessions::MaxDownloads).integer())
                    .col(
                        ColumnDef::new(UploadSessions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(UploadSessions::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on upload_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-upload_sessions-upload_id")
                    .table(UploadSessions::Table)
                    .col(UploadSessions::UploadId)
                    .to_owned(),
            )
            .await?;

        // Create index on expires_at for cleanup queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-upload_sessions-expires_at")
                    .table(UploadSessions::Table)
                    .col(UploadSessions::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UploadSessions::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UploadSessions {
    Table,
    Id,
    UploadId,
    Filename,
    MimeType,
    TotalSize,
    TotalChunks,
    UploadedChunks,
    ChunkSize,
    ExpiresInHours,
    MaxDownloads,
    CreatedAt,
    ExpiresAt,
}
