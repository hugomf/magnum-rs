//! Basic example showing how to decode an OGG Opus file.
//!
//! Run with: cargo run --example decode_ogg --features "with_ogg with_rodio"
//!
//! This example demonstrates:
//! - Opening an OGG Opus file
//! - Iterating over decoded audio samples
//! - Basic metadata inspection

use std::fs::File;
use std::io::BufReader;

fn main() {
    println!("Magnum Opus Decoder - OGG Example");
    println!("==================================\n");

    // Note: You'll need a real Opus file to test this
    // You can convert any audio file using: ffmpeg -i input.wav -c:a opus output.opus
    let file_path = "example.opus";

    match File::open(file_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            
            match magnum::container::ogg::OpusSourceOgg::new(reader) {
                Ok(source) => {
                    println!("Successfully opened Opus file!");
                    println!("  Channels: {}", source.metadata.channel_count);
                    println!("  Sample Rate: {} Hz", source.metadata.sample_rate);
                    println!("  Pre-skip: {} samples", source.metadata.preskip);
                    println!("  Output Gain: {}", source.metadata.output_gain);
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
                    eprintln!("Error opening Opus file: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error: Could not open file '{}'", file_path);
            eprintln!("       {}", e);
            eprintln!("\nTo test this example, place an Opus file named 'example.opus' in the current directory.");
            eprintln!("You can convert audio files using: ffmpeg -i input.wav -c:a opus example.opus");
        }
    }
}
