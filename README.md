# Kusatsu - Secure File Sharing

A secure file sharing application built with Rust.

## Key Features

- **Multiple interfaces**: Web frontend and CLI for different use cases
- **Dark mode**: Dark mode for the web frontend
- **Expiration and limits**: Files can have expiration dates and download limits

## Project Structure

This workspace contains multiple crates:

```
kusatsu/
├── kusatsu-backend/     # Axum-based REST API server
├── kusatsu-cli/         # Command-line interface
├── kusatsu-encrypt/     # Encryption library
├── kusatsu-entity/      # Database models (SeaORM)
├── kusatsu-frontend/    # Web interface (Yew/WASM)
└── kusatsu-migration/   # Database migrations
```

### Core Components

- **`kusatsu-encrypt`**: Handles AES-256-GCM encryption/decryption, key generation, and secure key handling
- **`kusatsu-entity`**: Database models for file metadata
- **`kusatsu-migration`**: Database schema and migrations
- **`kusatsu-backend`**: REST API for file upload/download
- **`kusatsu-cli`**: Command-line tool for uploading files
- **`kusatsu-frontend`**: Web-based drag-and-drop interface

## Quick Start

### Prerequisites
- Rust 1.70+
- PostgreSQL (for production) or SQLite (for development)

### Building the Project

```bash
# Check that everything compiles
cargo check

# Run tests
cargo test

# Test encryption functionality specifically
cargo test -p kusatsu-encrypt
```

### Running the Backend Server

```bash
# Set environment variables (optional - defaults provided)
export KUSATSU_DATABASE_URL="sqlite://kusatsu.db"
export KUSATSU_SERVER_ADDRESS="127.0.0.1:3000"
export KUSATSU_BASE_URL="http://localhost:3000"
export KUSATSU_MAX_FILE_SIZE="100"  # MB
export KUSATSU_STORAGE_DIR="./storage"
export KUSATSU_API_URL="http://localhost:3000"
export KUSATSU_CLEANUP_INTERVAL_HOURS="24"

# Run the backend server
cargo run -p kusatsu-backend
```

The server will start on `http://localhost:3000` with the following endpoints:

- `GET /health` - Health check endpoint
- `POST /api/upload` - Upload encrypted files (multipart form)
- `GET /api/files/{file_id}` - Download encrypted file data
- `GET /api/files/{file_id}/info` - Get file metadata
- `GET /api/admin/cleanup/files` - Cleanup expired files (setup cron job to run every day)
- `GET /api/admin/cleanup/upload-sessions` - Cleanup expired upload sessions (setup cron job to run every day)

### Using the CLI Application

The CLI provides a user-friendly interface for file operations:

```bash
# Upload a file
cargo run -p kusatsu-cli -- upload document.pdf

# Upload with expiration and download limits
cargo run -p kusatsu-cli -- upload secret.txt --expires-in-hours 24 --max-downloads 5

# See all options
cargo run -p kusatsu-cli -- --help
```

See [`kusatsu-cli/README.md`](kusatsu-cli/README.md) for detailed CLI documentation.

### Testing Encryption

The encryption crate includes comprehensive tests:

```bash
cargo test -p kusatsu-encrypt -- --nocapture
```

Run the encryption example:

```bash
cargo run --example basic_usage -p kusatsu-encrypt
```

## Backend Architecture

### Configuration
The backend uses environment variables for configuration:

| Variable | Default | Description |
|----------|---------|-------------|
| `KUSATSU_DATABASE_URL` | `sqlite://kusatsu.db` | Database connection string |
| `KUSATSU_SERVER_ADDRESS` | `127.0.0.1:3000` | Server bind address |
| `KUSATSU_BASE_URL` | `http://localhost:8080` | Public base URL for browser access |
| `KUSATSU_API_URL` | `http://localhost:3000` | API base URL for direct downloads |
| `KUSATSU_MAX_FILE_SIZE` | `100` | Maximum file size in MB |
| `KUSATSU_STORAGE_DIR` | `./storage` | File storage directory |
| `KUSATSU_CLEANUP_INTERVAL_HOURS` | `24` | Expired file cleanup interval |

## License

MIT License - see LICENSE file for details.