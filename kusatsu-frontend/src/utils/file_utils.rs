pub fn format_file_size(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

pub fn get_file_icon(filename: &str, mime_type: Option<&str>) -> &'static str {
    let name = filename.to_lowercase();
    let mime = mime_type.unwrap_or("").to_lowercase();

    // Check MIME type first if available
    if !mime.is_empty() {
        if mime.starts_with("image/") {
            return "🖼️";
        } else if mime.starts_with("video/") {
            return "🎥";
        } else if mime.starts_with("audio/") {
            return "🎵";
        } else if mime.contains("pdf") {
            return "📄";
        } else if mime.contains("text/") {
            return "📝";
        }
    }

    // Fall back to filename extension
    if name.ends_with(".pdf") {
        "📄"
    } else if name.ends_with(".txt") || name.ends_with(".md") {
        "📝"
    } else if name.ends_with(".zip") || name.ends_with(".rar") || name.ends_with(".7z") {
        "📦"
    } else if name.ends_with(".jpg") || name.ends_with(".jpeg") || name.ends_with(".png") || name.ends_with(".gif") || name.ends_with(".webp") || name.ends_with(".svg") {
        "🖼️"
    } else if name.ends_with(".mp4") || name.ends_with(".avi") || name.ends_with(".mov") || name.ends_with(".mkv") || name.ends_with(".webm") {
        "🎥"
    } else if name.ends_with(".mp3") || name.ends_with(".wav") || name.ends_with(".flac") || name.ends_with(".ogg") {
        "🎵"
    } else {
        "📁"
    }
}
