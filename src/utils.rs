use crate::error::{Error, Result};
use addr::parse_domain_name;
use std::path::Path;

// Determine the content type of a file based on its extension
pub fn determine_content_type(path: &Path) -> &str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("avi") => "video/x-msvideo",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    }
}

/// Validate an URL
pub fn is_valid_url(url: &str) -> Result<()> {
    // Try to parse the URL
    let parsed_url = match url::Url::parse(url) {
        Ok(url) => url,
        Err(_) => return Err(Error::InvalidRequest("Invalid URL: ".to_string() + url)),
    };

    // Check scheme - must be http or https
    match parsed_url.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(Error::InvalidRequest(
                "URL must use http or https scheme".to_string(),
            ))
        }
    }

    // Check if host exists
    let host = match parsed_url.host_str() {
        Some(host) => host,
        None => {
            return Err(Error::InvalidRequest(
                "URL must have a valid domain".to_string(),
            ))
        }
    };

    // Check if host is empty
    if host.is_empty() {
        return Err(Error::InvalidRequest(
            "URL must have a valid domain".to_string(),
        ));
    }

    if parse_domain_name(host).is_err() {
        return Err(Error::InvalidRequest(
            "URL must have a valid domain".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::utils::{determine_content_type, is_valid_url};
    use std::path::Path;

    #[test]
    fn test_determine_content_type_jpg() {
        let path = Path::new("image.jpg");
        assert_eq!(determine_content_type(path), "image/jpeg");
    }

    #[test]
    fn test_determine_content_type_jpeg() {
        let path = Path::new("photo.jpeg");
        assert_eq!(determine_content_type(path), "image/jpeg");
    }

    #[test]
    fn test_determine_content_type_png() {
        let path = Path::new("screenshot.png");
        assert_eq!(determine_content_type(path), "image/png");
    }

    #[test]
    fn test_determine_content_type_gif() {
        let path = Path::new("animation.gif");
        assert_eq!(determine_content_type(path), "image/gif");
    }

    #[test]
    fn test_determine_content_type_mp4() {
        let path = Path::new("video.mp4");
        assert_eq!(determine_content_type(path), "video/mp4");
    }

    #[test]
    fn test_determine_content_type_mov() {
        let path = Path::new("movie.mov");
        assert_eq!(determine_content_type(path), "video/quicktime");
    }

    #[test]
    fn test_determine_content_type_avi() {
        let path = Path::new("clip.avi");
        assert_eq!(determine_content_type(path), "video/x-msvideo");
    }

    #[test]
    fn test_determine_content_type_webm() {
        let path = Path::new("web_video.webm");
        assert_eq!(determine_content_type(path), "video/webm");
    }

    #[test]
    fn test_determine_content_type_uppercase_extension() {
        let path = Path::new("IMAGE.JPG");
        // The current implementation is case-sensitive, so uppercase should default to octet-stream
        assert_eq!(determine_content_type(path), "application/octet-stream");
    }

    #[test]
    fn test_determine_content_type_mixed_case() {
        let path = Path::new("photo.JpEg");
        // Case-sensitive implementation should default to octet-stream
        assert_eq!(determine_content_type(path), "application/octet-stream");
    }

    #[test]
    fn test_determine_content_type_no_extension() {
        let path = Path::new("filename_without_extension");
        assert_eq!(determine_content_type(path), "application/octet-stream");
    }

    #[test]
    fn test_determine_content_type_unknown_extension() {
        let path = Path::new("document.xyz");
        assert_eq!(determine_content_type(path), "application/octet-stream");
    }

    #[test]
    fn test_determine_content_type_empty_extension() {
        let path = Path::new("filename.");
        assert_eq!(determine_content_type(path), "application/octet-stream");
    }

    #[test]
    fn test_determine_content_type_multiple_dots() {
        let path = Path::new("archive.tar.gz");
        // Should use the last extension
        assert_eq!(determine_content_type(path), "application/octet-stream");

        let path = Path::new("backup.file.jpg");
        assert_eq!(determine_content_type(path), "image/jpeg");
    }

    #[test]
    fn test_determine_content_type_path_with_directory() {
        let path = Path::new("/home/user/photos/vacation.png");
        assert_eq!(determine_content_type(path), "image/png");
    }

    #[test]
    fn test_determine_content_type_relative_path() {
        let path = Path::new("./assets/video.mp4");
        assert_eq!(determine_content_type(path), "video/mp4");
    }

    #[test]
    fn test_determine_content_type_non_utf8_filename() {
        // Create a path that might have non-UTF8 characters in the extension
        // This tests the `.to_str()` fallback behavior
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Create invalid UTF-8 bytes
        let invalid_utf8 = &[0xFF, 0xFE];
        let os_str = OsStr::from_bytes(invalid_utf8);
        let mut path_buf = std::path::PathBuf::new();
        path_buf.push("test");
        path_buf.set_extension(os_str);

        assert_eq!(
            determine_content_type(&path_buf),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_valid_https_url() {
        assert!(is_valid_url("https://www.example.com").is_ok());
    }

    #[test]
    fn test_valid_http_url() {
        assert!(is_valid_url("http://www.example.com").is_ok());
    }

    #[test]
    fn test_valid_url_with_path() {
        assert!(is_valid_url("https://www.example.com/path/to/content").is_ok());
    }

    #[test]
    fn test_valid_url_with_query_params() {
        assert!(is_valid_url("https://www.example.com/video?id=123&t=456").is_ok());
    }

    #[test]
    fn test_valid_url_with_fragment() {
        assert!(is_valid_url("https://www.example.com/page#section").is_ok());
    }

    #[test]
    fn test_valid_subdomain_url() {
        assert!(is_valid_url("https://subdomain.example.com").is_ok());
    }

    #[test]
    fn test_invalid_scheme_ftp() {
        let result = is_valid_url("ftp://example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_scheme_file() {
        let result = is_valid_url("file:///path/to/file");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url_no_scheme() {
        let result = is_valid_url("www.example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url_malformed() {
        let result = is_valid_url("https://");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url_ip_address_v4() {
        let result = is_valid_url("https://192.168.1.1");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url_ip_address_v6() {
        let result = is_valid_url("https://[::1]");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_url() {
        let result = is_valid_url("");
        assert!(result.is_err());
    }

    #[test]
    fn test_social_media_urls() {
        // Test common social media URLs
        let social_media_urls = vec![
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "https://twitter.com/username/status/123456789",
            "https://www.instagram.com/p/ABC123/",
            "https://www.facebook.com/username/posts/123456789",
            "https://www.tiktok.com/@username/video/123456789",
            "https://www.linkedin.com/posts/activity-123456789",
        ];

        for url in social_media_urls {
            assert!(is_valid_url(url).is_ok(), "Failed to validate: {}", url);
        }
    }
}
