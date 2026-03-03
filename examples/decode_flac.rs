//! Basic example showing how to decode a FLAC file.
//!
//! Run with: cargo run --example decode_flac --features "with_flac with_rodio"
//!
//! This example demonstrates:
//! - Opening a FLAC file
//! - Iterating over decoded audio samples
//! - Basic metadata inspection

use std::fs::File;
use std::io::BufReader;

fn main() {
    println!("Magnum FLAC Decoder - FLAC Example");
    println!("===================================\n");

    // Note: You'll need a real FLAC file to test this
    // You can create a FLAC file using: ffmpeg -i input.wav -c:a flac output.flac
    let file_path = "example.flac";

    match File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            
            match magnum::container::flac::FlacSource::new(reader) {
                Ok(source) => {
                    println!("Successfully opened FLAC file!");
                    println!("  Channels: {}", source.metadata.channel_count);
                    println!("  Sample Rate: {} Hz", source.metadata.sample_rate);
                    println!("  Output Channels: {}", source.output_channels());
                    println!("  Downmixing Active: {}", source.is_downmixing);
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
                    eprintln!("Error opening FLAC file: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: Could not open file '{}'", file_path);
            eprintln!("       {}", e);
            eprintln!("\nTo test this example, place a FLAC file named 'example.flac' in the current directory.");
            eprintln!("You can convert audio files using: ffmpeg -i input.wav -c:a flac example.flac");
            eprintln!("Or create a test tone with: ffmpeg -f lavfi -i \"sine=frequency=440:duration=5\" -c:a flac example.flac");
        }
    }
}