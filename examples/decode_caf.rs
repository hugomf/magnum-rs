//! Basic example showing how to decode a CAF Opus file.
//!
//! Run with: cargo run --example decode_caf --features "with_caf with_rodio"
//!
//! This example demonstrates:
//! - Opening a CAF Opus file
//! - Iterating over decoded audio samples
//! - Basic metadata inspection

use std::fs::File;
use std::io::BufReader;

fn main() {
    println!("Magnum Opus Decoder - CAF Example");
    println!("==================================\n");

    // Note: You'll need a real CAF Opus file to test this
    // CAF is Apple's Core Audio Format - you can create Opus-in-CAF using:
    // ffmpeg -i input.wav -c:a libopus -f caf output.caf
    let file_path = "example.caf";

    match File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            
            match magnum::container::caf::OpusSourceCaf::new(reader) {
                Ok(source) => {
                    println!("Successfully opened CAF Opus file!");
                    println!("  Channels: {}", source.metadata.channel_count);
                    println!("  Sample Rate: {} Hz", source.metadata.sample_rate);
                    println!("  Output Channels: {}", source.output_channels());
                    println!("\nDecoding first 1000 samples...\n");

                    // Iterate over samples (this is an Iterator<Item=f32>)
                    for (i, sample) in source.take(1000).enumerate() {
                        if i % 48000 == 0 {
                            println!("  Sample {}: {:.4}", i, sample);
                        }
                    }
                    println!("\nDone!");
                }
                Err(e) => {
                    eprintln!("Error opening CAF Opus file: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: Could not open file '{}'", file_path);
            eprintln!("       {}", e);
            eprintln!("\nTo test this example, place a CAF Opus file named 'example.caf' in the current directory.");
            eprintln!("You can convert audio files using: ffmpeg -i input.wav -c:a libopus -f caf example.caf");
        }
    }
}
