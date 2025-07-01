# Kusatsu CLI - Secure File Sharing

A command-line interface for the Kusatsu secure file sharing system

## Features

- **🔗 Shareable URLs**: Generate secure URLs with encryption keys embedded in anchors
- **⏰ Expiration control**: Set automatic file expiration dates
- **📊 Download limits**: Limit the number of downloads per file

## Installation

```bash
# Build the CLI
cargo build --release -p kusatsu-cli

# Or run directly with cargo
cargo run -p kusatsu-cli -- <command>
```

## Quick Start

### Upload a File

```bash
# Upload a file (basic)
kusatsu-cli upload document.pdf

# Upload with expiration and download limit
kusatsu-cli upload secret.txt --expires-in-hours 24 --max-downloads 5

# Upload with JSON output
kusatsu-cli upload data.json --output json
```

## Commands

### `upload`

Upload and encrypt a file to the server.

```bash
kusatsu-cli upload <FILE> [OPTIONS]

Options:
  --expires-in-hours <HOURS>    File expiration time
  --max-downloads <COUNT>       Maximum download limit
  --output <FORMAT>             Output format: url (default) or json
```

**Example:**
```bash
kusatsu-cli upload presentation.pptx --expires-in-hours 48 --max-downloads 10
```
