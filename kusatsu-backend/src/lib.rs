use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use sea_orm::DatabaseConnection;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

// Re-export shared types from kusatsu-types
pub use kusatsu_types::*;

pub mod chunk_storage;
pub mod config;
pub mod database;
pub mod error;
pub mod handlers;
pub mod storage;

use chunk_storage::ChunkStorage;
use config::Config;
use database::setup_database;
use error::{AppError, Result};
use storage::FileStorage;

// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Config,
    pub storage: FileStorage,
    pub chunk_storage: ChunkStorage,
}

// All API types are now defined in kusatsu-types and re-exported above

pub async fn run_server() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env()?;

    // Setup database
    let db = setup_database(&config.database_url).await?;

    // Setup file storage
    let storage = FileStorage::new(&config.storage_dir);
    storage.init().await?;

    // Setup chunk storage
    let chunk_storage = ChunkStorage::new(&config.storage_dir);
    chunk_storage.init().await?;

    // Extract config values before moving state
    let server_address = config.server_address.clone();
    let storage_dir = config.storage_dir.clone();

    // Create application state
    let state = AppState {
        db,
        config,
        storage,
        chunk_storage,
    };

    // Build the application router
    let app = create_app(state);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(&server_address)
        .await
        .map_err(|e| {
            AppError::ServerError(format!("Failed to bind to {}: {}", server_address, e))
        })?;

    tracing::info!("🚀 Kusatsu backend server starting on {}", server_address);
    tracing::info!("📁 File storage directory: {}", storage_dir);

    // Start the server
    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::ServerError(format!("Server error: {}", e)))?;

    Ok(())
}

fn create_app(state: AppState) -> Router {
    Router::new()
        // File operations (legacy single upload)
        .route("/api/upload", post(handlers::upload_file))
        .route(
            "/api/files/:file_id/form",
            post(handlers::download_file_form),
        )
        .route("/api/files/:file_id/info", post(handlers::get_file_info))
        // Chunked upload operations
        .route(
            "/api/upload/start",
            post(handlers::start_chunked_upload).layer(DefaultBodyLimit::max(1024 * 1024)),
        ) // 1MB for JSON requests
        .route(
            "/api/upload/chunk/:upload_id/:chunk_number",
            post(handlers::upload_chunk).layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        ) // 20MB for chunk uploads
        .route(
            "/api/upload/complete",
            post(handlers::complete_chunked_upload),
        )
        .route(
            "/api/upload/status/:upload_id",
            get(handlers::get_upload_status),
        )
        // Cleanup operations
        .route(
            "/api/admin/cleanup/files",
            post(handlers::cleanup_expired_files),
        )
        .route(
            "/api/admin/cleanup/upload-sessions",
            post(handlers::cleanup_expired_upload_sessions),
        )
        // Health check
        .route("/health", get(handlers::health_check))
        // Static file serving for frontend
        .nest_service("/", ServeDir::new("static"))
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        )
        .with_state(state)
}
