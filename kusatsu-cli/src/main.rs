use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::multipart;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs as async_fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

// Import shared types
use kusatsu_types::*;

// Constants for chunked uploads
const MAX_SINGLE_UPLOAD_SIZE: usize = 5 * 1024 * 1024; // 5MB
const CHUNK_SIZE: usize = 5 * 1024 * 1024; // 5MB chunks

#[derive(Parser)]
#[command(name = "kusatsu")]
#[command(about = "A secure file sharing CLI with client-side encryption")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Backend server URL
    #[arg(long, default_value = "http://localhost:3000")]
    server: String,

    /// Timeout for requests in seconds
    #[arg(long, default_value_t = 30)]
    timeout: u64,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload a file
    Upload {
        /// File to upload
        file: PathBuf,

        /// Hours until file expires (optional)
        #[arg(long)]
        expires_in_hours: Option<i32>,

        /// Maximum number of downloads (optional)
        #[arg(long)]
        max_downloads: Option<i32>,

        /// Output format (json or url)
        #[arg(long, default_value = "url")]
        output: OutputFormat,
    },
}

#[derive(Clone, Debug)]
enum OutputFormat {
    Json,
    Url,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "url" => Ok(OutputFormat::Url),
            _ => Err(format!("Invalid output format: {}", s)),
        }
    }
}

#[derive(Clone)]
struct UploadConfig {
    expires_in_hours: Option<i32>,
    max_downloads: Option<i32>,
    output_format: OutputFormat,
}

// All API types are now defined in kusatsu-types and imported above

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(cli.timeout))
        .build()
        .context("Failed to create HTTP client")?;

    match cli.command {
        Commands::Upload {
            file,
            expires_in_hours,
            max_downloads,
            output,
        } => {
            upload_file(
                &client,
                &cli.server,
                &file,
                expires_in_hours,
                max_downloads,
                output,
            )
            .await?;
        }
    }

    Ok(())
}

async fn upload_file(
    client: &reqwest::Client,
    server: &str,
    file_path: &Path,
    expires_in_hours: Option<i32>,
    max_downloads: Option<i32>,
    output_format: OutputFormat,
) -> Result<()> {
    // Get file metadata
    let metadata = async_fs::metadata(file_path)
        .await
        .with_context(|| format!("Failed to read file metadata: {}", file_path.display()))?;

    let file_size = metadata.len() as usize;
    let filename = file_path
        .file_name()
        .context("Invalid filename")?
        .to_string_lossy()
        .to_string();

    println!("📁 Uploading file: {} ({} bytes)", filename, file_size);

    // Detect MIME type
    let mime_type = mime_guess::from_path(file_path)
        .first()
        .map(|mime| mime.to_string());

    let config = UploadConfig {
        expires_in_hours,
        max_downloads,
        output_format,
    };

    // Decide between single and chunked upload
    if file_size <= MAX_SINGLE_UPLOAD_SIZE {
        println!("📦 Using single upload (file size: {} bytes)", file_size);
        perform_single_upload(client, server, file_path, &filename, mime_type, &config).await
    } else {
        println!("🧩 Using chunked upload (file size: {} bytes)", file_size);
        perform_chunked_upload(
            client, server, file_path, &filename, file_size, mime_type, &config,
        )
        .await
    }
}

async fn perform_single_upload(
    client: &reqwest::Client,
    server: &str,
    file_path: &Path,
    filename: &str,
    mime_type: Option<String>,
    config: &UploadConfig,
) -> Result<()> {
    // Read the entire file
    let file_data = async_fs::read(file_path)
        .await
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Create multipart form
    let mut form = multipart::Form::new()
        .part(
            "file_data",
            multipart::Part::bytes(file_data).file_name(filename.to_string()),
        )
        .part("filename", multipart::Part::text(filename.to_string()));

    if let Some(mime) = mime_type {
        form = form.part("mime_type", multipart::Part::text(mime));
    }

    // Build URL with query parameters
    let mut url = format!("{}/api/upload", server);
    let mut params = Vec::new();

    if let Some(hours) = config.expires_in_hours {
        params.push(format!("expires_in_hours={}", hours));
    }

    if let Some(max_dl) = config.max_downloads {
        params.push(format!("max_downloads={}", max_dl));
    }

    if !params.is_empty() {
        url.push('?');
        url.push_str(&params.join("&"));
    }

    // Send the request
    let response = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .context("Failed to send upload request")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(
            "Upload failed with status {}: {}",
            status,
            error_text
        ));
    }

    let upload_response: UploadResponse = response
        .json()
        .await
        .context("Failed to parse upload response")?;

    print_upload_result(upload_response, &config.output_format)?;
    Ok(())
}

async fn perform_chunked_upload(
    client: &reqwest::Client,
    server: &str,
    file_path: &Path,
    filename: &str,
    file_size: usize,
    mime_type: Option<String>,
    config: &UploadConfig,
) -> Result<()> {
    // Step 1: Start upload session
    println!("🚀 Starting chunked upload session...");

    let start_request = StartUploadRequest {
        filename: filename.to_string(),
        file_size: file_size as i64,
        mime_type,
        chunk_size: Some(CHUNK_SIZE as i32),
        expires_in_hours: config.expires_in_hours,
        max_downloads: config.max_downloads,
    };

    let start_url = format!("{}/api/upload/start", server);
    let start_response = client
        .post(&start_url)
        .json(&start_request)
        .send()
        .await
        .context("Failed to start upload session")?;

    if !start_response.status().is_success() {
        let status = start_response.status();
        let error_text = start_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(
            "Failed to start upload session with status {}: {}",
            status,
            error_text
        ));
    }

    let start_upload_response: StartUploadResponse = start_response
        .json()
        .await
        .context("Failed to parse start upload response")?;

    let upload_id = start_upload_response.upload_id;
    let total_chunks = start_upload_response.total_chunks;
    let chunk_size = start_upload_response.chunk_size as usize;

    println!(
        "📊 Upload session started: {} chunks of {} bytes each",
        total_chunks, chunk_size
    );

    // Create progress bar
    let pb = ProgressBar::new(total_chunks as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} chunks ({percent}%) {msg}")
            .expect("Failed to set progress bar template")
            .progress_chars("#>-")
    );
    pb.set_message("Uploading chunks...");

    // Step 2: Upload chunks
    let mut file = async_fs::File::open(file_path)
        .await
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;

    for chunk_number in 0..total_chunks {
        let start_offset = chunk_number as usize * chunk_size;
        let remaining_size = file_size - start_offset;
        let current_chunk_size = std::cmp::min(chunk_size, remaining_size);

        // Read chunk from file
        let mut chunk_data = vec![0u8; current_chunk_size];
        file.seek(std::io::SeekFrom::Start(start_offset as u64))
            .await
            .with_context(|| format!("Failed to seek to offset {}", start_offset))?;

        file.read_exact(&mut chunk_data)
            .await
            .with_context(|| format!("Failed to read chunk {}", chunk_number))?;

        // Upload chunk
        let chunk_url = format!("{}/api/upload/chunk/{}/{}", server, upload_id, chunk_number);

        let chunk_form = multipart::Form::new().part(
            "chunk",
            multipart::Part::bytes(chunk_data).file_name(format!("chunk_{}", chunk_number)),
        );

        let chunk_response = client
            .post(&chunk_url)
            .multipart(chunk_form)
            .send()
            .await
            .with_context(|| format!("Failed to upload chunk {}", chunk_number))?;

        if !chunk_response.status().is_success() {
            let status = chunk_response.status();
            let error_text = chunk_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Failed to upload chunk {} with status {}: {}",
                chunk_number,
                status,
                error_text
            ));
        }

        let chunk_upload_response: ChunkUploadResponse = chunk_response
            .json()
            .await
            .with_context(|| format!("Failed to parse chunk {} upload response", chunk_number))?;

        // Update progress bar
        pb.set_position(chunk_upload_response.uploaded_chunks as u64);
        pb.set_message(format!(
            "Uploaded {} bytes",
            start_offset + current_chunk_size
        ));
    }

    // Finish progress bar
    pb.finish_with_message("All chunks uploaded successfully!");

    // Step 3: Complete upload
    println!("🏁 Completing upload...");

    let complete_request = CompleteUploadRequest { upload_id };

    let complete_url = format!("{}/api/upload/complete", server);
    let complete_response = client
        .post(&complete_url)
        .json(&complete_request)
        .send()
        .await
        .context("Failed to complete upload")?;

    if !complete_response.status().is_success() {
        let status = complete_response.status();
        let error_text = complete_response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(anyhow::anyhow!(
            "Failed to complete upload with status {}: {}",
            status,
            error_text
        ));
    }

    let complete_upload_response: UploadResponse = complete_response
        .json()
        .await
        .context("Failed to parse complete upload response")?;

    // Convert to standard UploadResponse format for consistent output
    let upload_response = UploadResponse {
        file_id: complete_upload_response.file_id,
        download_url: complete_upload_response.download_url,
        encryption_key: complete_upload_response.encryption_key,
        curl_command: complete_upload_response.curl_command,
    };

    print_upload_result(upload_response, &config.output_format)?;
    Ok(())
}

fn print_upload_result(
    upload_response: UploadResponse,
    output_format: &OutputFormat,
) -> Result<()> {
    // Create the complete shareable URL with encryption key (if available)
    let shareable_url = if let Some(ref encryption_key) = upload_response.encryption_key {
        format!("{}#{}", upload_response.download_url, encryption_key)
    } else {
        upload_response.download_url.to_string()
    };

    match output_format {
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "file_id": upload_response.file_id,
                "download_url": upload_response.download_url,
                "encryption_key": upload_response.encryption_key,
                "shareable_url": shareable_url,
                "curl_command": upload_response.curl_command
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Url => {
            println!("✅ File uploaded successfully!");
            if upload_response.encryption_key.is_some() {
                println!("📎 Shareable URL: {}", shareable_url);
            } else {
                println!("📎 Download URL: {}", shareable_url);
                println!("ℹ️  Note: This file was uploaded without encryption");
            }
            println!("💻 Download with curl: {}", upload_response.curl_command);
        }
    }

    Ok(())
}
