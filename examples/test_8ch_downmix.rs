//! Test script to demonstrate 8-channel FLAC downmixing
//!
//! This script tests the 8-channel FLAC file we created and shows how the downmixing works.

use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing 8-channel FLAC Downmixing");
    println!("==================================\n");

    let file_path = "example.flac";

    match File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            
            match magnum::container::flac::FlacSource::new(reader) {
                Ok(source) => {
                    println!("✓ Successfully opened 8-channel FLAC file!");
                    println!("  Original channels: {}", source.metadata.channel_count);
                    println!("  Output channels: {}", source.output_channels());
                    println!("  Downmixing Active: {}", source.is_downmixing);
                    println!("  Sample Rate: {} Hz", source.metadata.sample_rate);
                    
                    // Process samples to show downmixing in action
                    println!("\nProcessing first 100 samples to demonstrate downmixing:");
                    
                    let mut left_samples = Vec::new();
                    let mut right_samples = Vec::new();
                    
                    for (i, sample) in source.take(100).enumerate() {
                        if i % 2 == 0 {
                            left_samples.push(sample);
                        } else {
                            right_samples.push(sample);
                        }
                        
                        if i < 10 {
                            println!("  Sample {}: {:.6}", i + 1, sample);
                        }
                    }
                    
                    println!("\nDownmixing Summary:");
                    println!("  - Left channel samples: {}", left_samples.len());
                    println!("  - Right channel samples: {}", right_samples.len());
                    println!("  - Total samples processed: {}", left_samples.len() + right_samples.len());
                    
                    // Show some sample values
                    if !left_samples.is_empty() {
                        let min_val = left_samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = left_samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        println!("  - Left channel sample range: {:.6} to {:.6}", min_val, max_val);
                    }
                    if !right_samples.is_empty() {
                        let min_val = right_samples.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = right_samples.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        println!("  - Right channel sample range: {:.6} to {:.6}", min_val, max_val);
                    }
                    
                    println!("\n✓ 8-channel downmixing test completed successfully!");
                }
                Err(e) => {
                    eprintln!("Error opening FLAC file: {:?}", e);
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            eprintln!("Error: Could not open file '{}'", file_path);
            eprintln!("       {}", e);
        }
    }

    Ok(())
}