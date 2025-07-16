use realitydefender::{Client, Config, GetResultsOptions};
use std::env;

/// Example demonstrating how to retrieve paginated detection results
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = env::var("REALITY_DEFENDER_API_KEY")
        .expect("Please set REALITY_DEFENDER_API_KEY environment variable");

    // Initialize the client
    let client = Client::new(Config {
        api_key,
        ..Default::default()
    })?;

    println!("Reality Defender SDK - Get Results Example");
    println!("=========================================\n");

    // Example 1: Get first page of results with default settings
    println!("1. Fetching first page of results (default settings):");
    match client.get_results(None).await {
        Ok(results) => {
            print_results(&results);
        }
        Err(e) => {
            eprintln!("Error fetching results: {:?}", e);
        }
    }

    // Example 2: Get results with pagination
    println!("\n2. Fetching results with pagination (page 0, size 5):");
    let options = GetResultsOptions {
        page_number: Some(0),
        size: Some(5),
        ..Default::default()
    };

    match client.get_results(Some(options)).await {
        Ok(results) => {
            print_results(&results);
        }
        Err(e) => {
            eprintln!("Error fetching paginated results: {:?}", e);
        }
    }

    // Example 3: Get results with date filter (broad range to catch existing results)
    println!("\n3. Fetching results with date filter (2024-2025):");
    let date_options = GetResultsOptions {
        page_number: Some(0),
        size: Some(10),
        start_date: Some("2024-01-01".to_string()),
        end_date: Some("2025-12-31".to_string()),
        ..Default::default()
    };

    match client.get_results(Some(date_options)).await {
        Ok(results) => {
            print_results(&results);
        }
        Err(e) => {
            eprintln!("Error fetching date-filtered results: {:?}", e);
        }
    }

    // Example 4: Get results with name filter
    println!("\n4. Fetching results with name filter:");
    let name_options = GetResultsOptions {
        page_number: Some(0),
        size: Some(10),
        name: Some("test".to_string()),
        ..Default::default()
    };

    match client.get_results(Some(name_options)).await {
        Ok(results) => {
            print_results(&results);
        }
        Err(e) => {
            eprintln!("Error fetching name-filtered results: {:?}", e);
        }
    }

    // Example 5: Get results with waiting/polling for completion
    println!("\n5. Fetching results with polling for completion:");
    let polling_options = GetResultsOptions {
        page_number: Some(0),
        size: Some(5),
        max_attempts: Some(30),
        polling_interval: Some(2000), // 2 seconds
        ..Default::default()
    };

    match client.get_results(Some(polling_options)).await {
        Ok(results) => {
            print_results(&results);
        }
        Err(e) => {
            eprintln!("Error fetching results with polling: {:?}", e);
        }
    }

    Ok(())
}

fn print_results(results: &realitydefender::FormattedDetectionResultList) {
    println!("Total Results: {}", results.total_items);
    println!(
        "Current Page: {} of {}",
        results.current_page + 1,
        results.total_pages
    );
    println!("Results on this page: {}", results.current_page_items_count);

    if !results.items.is_empty() {
        println!("\nDetection Results:");
        for (i, result) in results.items.iter().enumerate() {
            println!("\n{}. Request ID: {}", i + 1, result.request_id);
            println!("   Status: {}", result.status);
            if let Some(score) = result.score {
                println!("   Score: {:.4} ({:.1}%)", score, score * 100.0);
            } else {
                println!("   Score: None");
            }

            if !result.models.is_empty() {
                println!("   Models:");
                for model in &result.models {
                    print!("     - {}: {}", model.name, model.status);
                    if let Some(model_score) = model.score {
                        println!(" (Score: {:.4})", model_score);
                    } else {
                        println!(" (Score: None)");
                    }
                }
            }
        }
    } else {
        println!("\nNo results found.");
    }
}
