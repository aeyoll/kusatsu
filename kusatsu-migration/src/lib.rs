pub use sea_orm_migration::prelude::*;

mod m20231101_000001_create_files_table;
mod m20231102_000001_create_upload_sessions_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231101_000001_create_files_table::Migration),
            Box::new(m20231102_000001_create_upload_sessions_table::Migration),
        ]
    }
}
