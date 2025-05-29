use realitydefender::Config;
use realitydefender::Error;

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.api_key, "".to_string());
    assert_eq!(config.base_url, None);
    assert_eq!(config.timeout_seconds, None);
}

#[test]
fn test_config_with_custom_url() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some("https://custom-api.example.com".to_string()),
        timeout_seconds: None,
    };

    assert_eq!(config.api_key, "test_api_key");
    assert_eq!(
        config.base_url,
        Some("https://custom-api.example.com".to_string())
    );
    assert_eq!(config.timeout_seconds, None);
}

#[test]
fn test_config_with_timeout() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: None,
        timeout_seconds: Some(120),
    };

    assert_eq!(config.api_key, "test_api_key");
    assert_eq!(config.base_url, None);
    assert_eq!(config.timeout_seconds, Some(120));
}

#[test]
fn test_config_with_all_options() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some("https://custom-api.example.com".to_string()),
        timeout_seconds: Some(120),
    };

    assert_eq!(config.api_key, "test_api_key");
    assert_eq!(
        config.base_url,
        Some("https://custom-api.example.com".to_string())
    );
    assert_eq!(config.timeout_seconds, Some(120));
}

#[test]
fn test_validate_valid_config() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: None,
        timeout_seconds: None,
    };

    let result = config.validate();
    assert!(result.is_ok());
}

#[test]
fn test_validate_empty_api_key() {
    let config = Config::default(); // Empty API key
    let result = config.validate();
    assert!(result.is_err());

    match result {
        Err(Error::InvalidConfig(msg)) => {
            assert!(msg.contains("API key is required"));
        }
        _ => panic!("Expected InvalidConfig error"),
    }
}

#[test]
fn test_validate_empty_base_url() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some("".to_string()),
        timeout_seconds: None,
    };

    let result = config.validate();
    assert!(result.is_err());

    match result {
        Err(Error::InvalidConfig(msg)) => {
            assert!(msg.contains("Base URL cannot be empty"));
        }
        _ => panic!("Expected InvalidConfig error"),
    }
}

#[test]
fn test_get_base_url_default() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: None,
        timeout_seconds: None,
    };

    assert_eq!(config.get_base_url(), "https://api.prd.realitydefender.xyz");
}

#[test]
fn test_get_base_url_custom() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: Some("https://custom-api.example.com".to_string()),
        timeout_seconds: None,
    };

    assert_eq!(config.get_base_url(), "https://custom-api.example.com");
}

#[test]
fn test_get_timeout_seconds_default() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: None,
        timeout_seconds: None,
    };

    assert_eq!(config.get_timeout_seconds(), 30); // Default is 30 seconds
}

#[test]
fn test_get_timeout_seconds_custom() {
    let config = Config {
        api_key: "test_api_key".to_string(),
        base_url: None,
        timeout_seconds: Some(120),
    };

    assert_eq!(config.get_timeout_seconds(), 120);
}
