use realitydefender::{BatchOptions, Client, Config};
use std::{env, path::Path};

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

    // List of files to analyze - replace with paths to actual files on your system
    let files = vec![
        "images/image1.jpg",
        "images/image2.jpg",
        "images/video1.mp4",
    ];

    // Check if files exist
    let mut valid_files = Vec::new();
    for file in &files {
        if Path::new(file).exists() {
            valid_files.push(*file);
        } else {
            println!("Warning: File does not exist at path: {}", file);
        }
    }

    if valid_files.is_empty() {
        println!("No valid files found. Please provide paths to existing files.");
        return Ok(());
    }

    println!("Processing {} valid files in batch...", valid_files.len());

    // Process multiple files concurrently with batch processing
    let results = client
        .process_batch(
            valid_files.clone(),
            BatchOptions {
                max_concurrency: Some(2), // Process 2 files at a time
                wait: Some(true),         // Wait for results
                timeout_seconds: Some(120),
            },
        )
        .await?;

    // Print results
    println!("\nBatch processing complete!");
    for (i, result) in results.iter().enumerate() {
        println!("\nFile: {}", valid_files[i]);
        println!("Status: {}", result.status);

        // Display normalized score (normalized by SDK)
        if let Some(score) = result.score {
            println!("Score: {:.4} ({:.1}%)", score, score * 100.0);
        } else {
            println!("No overall score available");
        }

        // Print model-specific results
        if !result.models.is_empty() {
            println!("Model-specific results:");
            for model in &result.models {
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
    }

    Ok(())
}
