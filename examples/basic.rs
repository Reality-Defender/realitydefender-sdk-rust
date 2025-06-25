use realitydefender::{Client, Config, UploadOptions};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = env::var("REALITY_DEFENDER_API_KEY")
        .expect("REALITY_DEFENDER_API_KEY environment variable must be set");

    // Initialize the client
    let client = Client::new(Config {
        api_key,
        ..Default::default()
    })?;

    // Path to the file to analyze - make sure this file exists
    let file_path = "files/test_image.jpg";

    println!("Checking if file exists at path: {}", file_path);
    if !std::path::Path::new(file_path).exists() {
        println!("Warning: File does not exist at path: {}", file_path);
        println!("Please provide a valid file path to an existing image or video file.");
        return Ok(());
    }

    println!("Uploading file: {}", file_path);

    // Upload the file
    let upload_result = client
        .upload(UploadOptions {
            file_path: file_path.to_string(),
        })
        .await?;

    println!(
        "Upload successful! Request ID: {}",
        upload_result.request_id
    );

    // Get the analysis result with waiting
    println!("Waiting for analysis result...");
    let result = client
        .get_result(
            &upload_result.request_id,
            Some(realitydefender::GetResultOptions {
                max_attempts: Some(30),
                polling_interval: Some(2000),
            }),
        )
        .await?;

    // Print the result
    println!("Analysis complete!");
    println!("Status: {}", result.status);

    if let Some(score) = result.score {
        println!("Score: {:.4} ({:.1}%)", score, score * 100.0);
    } else {
        println!("No overall score available");
    }

    // Print model-specific results
    if !result.models.is_empty() {
        println!("\nModel-specific results:");
        for model in result.models {
            if model.status != "NOT_APPLICABLE" {
                println!(
                    "- {}: Status: {}, Score: {}",
                    model.name,
                    model.status,
                    model
                        .score
                        .map_or("N/A".to_string(), |s| format!("{:.4}", s))
                );
            }
        }
    }

    Ok(())
}
