#[derive(Debug)]
pub struct FileTypeConfig {
    pub extensions: &'static [&'static str],
    pub size_limit: u64,
}

pub const SUPPORTED_FILE_TYPES: &[FileTypeConfig] = &[
    FileTypeConfig {
        extensions: &["mp4", "mov"],
        size_limit: 262144000, // 250 MB
    },
    FileTypeConfig {
        extensions: &["jpg", "png", "jpeg", "gif", "webp"],
        size_limit: 52428800, // 50 MB
    },
    FileTypeConfig {
        extensions: &["flac", "wav", "mp3", "m4a", "aac", "alac", "ogg"],
        size_limit: 20971520, // 20 MB
    },
    FileTypeConfig {
        extensions: &["txt"],
        size_limit: 5242880, // 5 MB
    },
];
