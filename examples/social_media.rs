use realitydefender::{Client, Config, GetResultOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = std::env::var("REALITY_DEFENDER_API_KEY")
        .expect("Please set the REALITY_DEFENDER_API_KEY environment variable");

    // Create a client with the API key
    let client = Client::new(Config {
        api_key,
        base_url: None,        // Uses default production URL
        timeout_seconds: None, // Uses default timeout
    })?;

    // Example social media URLs to analyze
    let social_media_urls = vec![
        "https://www.youtube.com/watch?v=6O0fySNw-Lw",
        "https://youtube.com/watch?v=ABC123",
    ];

    println!("🔗 Uploading social media links for analysis...\n");

    for (i, url) in social_media_urls.iter().enumerate() {
        println!("📤 Uploading link {}: {}", i + 1, url);

        // Upload the social media link for analysis
        match client.upload_social_media(url).await {
            Ok(upload_result) => {
                println!("✅ Upload successful!");
                println!("   Request ID: {}", upload_result.request_id);

                // Wait for analysis to complete and get results
                println!("⏳ Waiting for analysis to complete...");

                match client
                    .get_result(
                        &upload_result.request_id,
                        Some(GetResultOptions {
                            max_attempts: Some(30),       // Try up to 30 times
                            polling_interval: Some(2000), // Wait 2 seconds between attempts
                        }),
                    )
                    .await
                {
                    Ok(detection_result) => {
                        println!("🎯 Analysis completed!");
                        println!("   Status: {}", detection_result.status);

                        if let Some(score) = detection_result.score {
                            println!("   Overall Score: {:.2}% ({:.3})", score * 100.0, score);

                            // Interpret the score
                            let interpretation = match score {
                                s if s < 0.3 => "Likely authentic",
                                s if s < 0.7 => "Uncertain - requires human review",
                                _ => "Likely manipulated",
                            };
                            println!("   Interpretation: {}", interpretation);
                        } else {
                            println!("   Score: Not available");
                        }

                        // Show model-specific results
                        if !detection_result.models.is_empty() {
                            println!("   Model Results:");
                            for model in &detection_result.models {
                                let model_score = model
                                    .score
                                    .map(|s| format!("{:.2}% ({:.3})", s * 100.0, s))
                                    .unwrap_or_else(|| "N/A".to_string());
                                println!(
                                    "     - {}: {} (Score: {})",
                                    model.name, model.status, model_score
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to get analysis result: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Upload failed: {}", e);

                // Show helpful error messages for common issues
                let error_msg = format!("{}", e);
                if error_msg.contains("Invalid URL format") {
                    eprintln!("   💡 Tip: Make sure the URL starts with http:// or https://");
                } else if error_msg.contains("http or https scheme") {
                    eprintln!("   💡 Tip: Only HTTP and HTTPS URLs are supported");
                } else if error_msg.contains("domain, not an IP address") {
                    eprintln!(
                        "   💡 Tip: Social media links must use domain names, not IP addresses"
                    );
                } else if error_msg.contains("proper TLD") {
                    eprintln!(
                        "   💡 Tip: URL must have a valid top-level domain (e.g., .com, .org)"
                    );
                }
            }
        }

        println!(); // Add spacing between uploads
    }

    println!("🏁 Social media analysis complete!");

    Ok(())
}
