use std::{fs, path::{Path, PathBuf}};

const EXTS: [&str; 7] = ["mp3", "flac", "wav", "ogg", "aac", "m4a", "mp4"];

pub fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Audio files directly in `dir`, sorted by name. Hidden files skipped.
pub fn audio_files(dir: &Path) -> Vec<PathBuf> {
    list(dir, |t| t.is_file(), is_audio)
}

/// Subfolders directly in `dir`, sorted by name. Symlinks are not followed.
pub fn subdirs(dir: &Path) -> Vec<PathBuf> {
    list(dir, |t| t.is_dir(), |_| true)
}

fn list(dir: &Path, kind: fn(fs::FileType) -> bool, keep: fn(&Path) -> bool) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter(|e| e.file_type().map(kind).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| keep(p))
        .collect();
    out.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    out
}
