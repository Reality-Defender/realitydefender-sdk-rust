use realitydefender::utils::file_exists;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_file_exists_with_existing_file() {
    // Create a temporary directory that gets cleaned up automatically
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_file.txt");

    // Create a file in the temporary directory
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "Test content").unwrap();

    // Test the file_exists function
    assert!(file_exists(file_path.to_str().unwrap()));
}

#[test]
fn test_file_exists_with_nonexistent_file() {
    // Path to a file that definitely doesn't exist
    let file_path = "/path/to/nonexistent/file_12345.txt";

    // Test the file_exists function
    assert!(!file_exists(file_path));
}

#[test]
fn test_file_exists_with_directory() {
    // Create a temporary directory that gets cleaned up automatically
    let dir = tempdir().unwrap();

    // Test the file_exists function with a directory path
    assert!(file_exists(dir.path().to_str().unwrap()));
}

#[test]
fn test_file_exists_with_empty_path() {
    // Test with an empty path
    assert!(!file_exists(""));
}
