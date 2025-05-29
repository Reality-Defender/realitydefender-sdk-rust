use std::path::Path;

/// Check if a file exists and is accessible
pub fn file_exists(path: &str) -> bool {
    Path::new(path).exists()
}
