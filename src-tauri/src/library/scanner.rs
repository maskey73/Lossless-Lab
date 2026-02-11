use std::path::Path;
use walkdir::WalkDir;

const AUDIO_EXTENSIONS: &[&str] = &[
    "flac", "mp3", "wav", "ogg", "m4a", "aac", "wma", "alac", "ape", "opus",
];

/// Scan a directory recursively for audio files.
pub fn scan_directory(path: &str) -> Vec<String> {
    let mut files = Vec::new();

    // Use simple recursive directory walk
    scan_dir_recursive(Path::new(path), &mut files);

    files.sort();
    files
}

fn scan_dir_recursive(dir: &Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir_recursive(&path, files);
            } else if is_audio_file(&path) {
                if let Some(path_str) = path.to_str() {
                    files.push(path_str.to_string());
                }
            }
        }
    }
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
