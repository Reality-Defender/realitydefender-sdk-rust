# Reality Defender Rust SDK

[![codecov](https://codecov.io/gh/Reality-Defender/realitydefender-sdk-rust/graph/badge.svg?token=QSZA0QTEQ5)](https://codecov.io/gh/Reality-Defender/realitydefender-sdk-rust)

The Reality Defender Rust SDK provides a simple and efficient way to integrate deepfake detection capabilities into your Rust applications.

## Features

- Asynchronous API built on Tokio
- Type-safe interfaces with Serde for serialization
- Secure file uploads using presigned URLs
- Comprehensive error handling
- High test coverage

## Installation

Add the SDK to your Cargo.toml:

```toml
[dependencies]
realitydefender = "0.1.2"
```

## Usage

### Basic Example

```rust
use realitydefender::{Client, Config, UploadOptions};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the client with your API key
    let client = Client::new(Config {
        api_key: env::var("REALITY_DEFENDER_API_KEY")?,
        ..Default::default()
    })?;

    // Upload a file for analysis
    let upload_result = client.upload(UploadOptions {
        file_path: "./image.jpg".to_string(),
        metadata: None,
    }).await?;

    println!("Request ID: {}", upload_result.request_id);

    // Get the analysis result with waiting for completion
    let result = client.get_result(
        &upload_result.request_id,
        Some(realitydefender::GetResultOptions {
            wait: Some(true),
            timeout_seconds: Some(60),
        }),
    ).await?;
    
    println!("Status: {}", result.status);
    if let Some(score) = result.score {
        println!("Score: {:.4} ({:.1}%)", score, score * 100.0);
    }

    // Access model-specific results
    for model in result.models {
        if model.status != "NOT_APPLICABLE" {
            println!(
                "Model: {}, Status: {}, Score: {:.4}", 
                model.name, 
                model.status, 
                model.score.unwrap_or(0.0)
            );
        }
    }

    Ok(())
}
```

### Processing Multiple Files

```rust
use realitydefender::{Client, Config, BatchOptions};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the client
    let client = Client::new(Config {
        api_key: env::var("REALITY_DEFENDER_API_KEY")?,
        ..Default::default()
    })?;

    // Process multiple files concurrently
    let results = client.process_batch(
        vec!["./files/image1.jpg", "./files/image2.jpg", "./files/video.mp4"],
        BatchOptions {
            max_concurrency: Some(3),
            wait: Some(true),
            timeout_seconds: Some(120),
        }
    ).await?;

    // Print results
    for (idx, result) in results.iter().enumerate() {
        println!("File {}: Status: {}", idx + 1, result.status);
        if let Some(score) = result.score {
            println!("  Score: {:.4} ({:.1}%)", score, score * 100.0);
        }
    }

    Ok(())
}
```

### Simplified Detection

```rust
use realitydefender::{Client, Config};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the client
    let client = Client::new(Config {
        api_key: env::var("REALITY_DEFENDER_API_KEY")?,
        ..Default::default()
    })?;

    // Detect a file with a single call
    let result = client.detect_file("./files/image.jpg").await?;
    
    println!("Status: {}", result.status);
    if let Some(score) = result.score {
        println!("Score: {:.4} ({:.1}%)", score, score * 100.0);
    }

    Ok(())
}
```

## Running the Examples

The SDK comes with several examples that demonstrate how to use its features. To run these examples, you need to set your API key as an environment variable:

```bash
export REALITY_DEFENDER_API_KEY=your_api_key_here
```

Then, you can run the examples using Cargo:

```bash
# Run the basic example
cargo run --example basic

# Run the batch processing example
cargo run --example batch_processing
```

### Required Test Files

To run the examples successfully, you'll need to add your own image and video files to the `files` directory:

1. Create an `files` directory in the root of the project (if it doesn't already exist):
   ```bash
   mkdir -p files
   ```

2. Add the following files to this directory:
   - `image1.jpg` - Any sample image for testing image analysis
   - `image2.jpg` - Another sample image
   - `test_image.jpg` - A third test image
   - `video1.mp4` - A sample video file for testing video analysis

You can use any JPG files and MP4 videos for testing purposes. The examples are configured to use these specific filenames from the `files` directory:

```rust
// Using the sample files in your code
let result = client.detect_file("./files/image1.jpg").await?;

// For batch processing
let results = client.process_batch(
    vec!["./files/image1.jpg", "./files/image2.jpg", "./files/video1.mp4"],
    BatchOptions::default()
).await?;
```

> **Note:** If you prefer to use different filenames or paths, make sure to update the example code accordingly.

## How It Works

The SDK implements the following workflow:

1. **Authentication**: Uses your API key to authenticate all requests to the Reality Defender API.
2. **File Upload**:
   - Requests a presigned URL from the Reality Defender API
   - Uploads the file directly to the storage provider using the presigned URL
   - Returns a request ID for tracking the analysis
3. **Result Retrieval**:
   - Polls the API for results using the request ID
   - Optionally waits until the analysis is complete
   - Returns detailed analysis results including overall and model-specific scores

## API Reference

See the [documentation](https://docs.rs/realitydefender) for complete API details.

## Development

### Prerequisites

- Rust 1.56 or later
- Cargo

### Setup

1. Clone the repository
2. Install dependencies:

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running with Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Xml
```
