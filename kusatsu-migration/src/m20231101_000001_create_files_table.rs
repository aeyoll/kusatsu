use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Files::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Files::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Files::FileId)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Files::OriginalSize).big_integer().not_null())
                    .col(
                        ColumnDef::new(Files::EncryptedSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Files::MimeType).string())
                    .col(ColumnDef::new(Files::FilePath).string().not_null())
                    .col(ColumnDef::new(Files::Nonce).binary().not_null())
                    .col(ColumnDef::new(Files::EncryptedFilename).binary().not_null())
                    .col(ColumnDef::new(Files::FilenameNonce).binary().not_null())
                    .col(
                        ColumnDef::new(Files::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Files::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(Files::DownloadCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Files::MaxDownloads).integer())
                    .to_owned(),
            )
            .await?;

        // Create index on file_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-files-file_id")
                    .table(Files::Table)
                    .col(Files::FileId)
                    .to_owned(),
            )
            .await?;

        // Create index on created_at for cleanup queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-files-created_at")
                    .table(Files::Table)
                    .col(Files::CreatedAt)
                    .to_owned(),
            )
            .await?;

        // Create index on expires_at for cleanup queries
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-files-expires_at")
                    .table(Files::Table)
                    .col(Files::ExpiresAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Files::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Files {
    Table,
    Id,
    FileId,
    OriginalSize,
    EncryptedSize,
    MimeType,
    FilePath,
    Nonce,
    EncryptedFilename,
    FilenameNonce,
    CreatedAt,
    ExpiresAt,
    DownloadCount,
    MaxDownloads,
}
