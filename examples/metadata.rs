//! Example showing how to inspect Opus file metadata without fully decoding.
//!
//! Run with: cargo run --example metadata --features "with_ogg with_caf"
//!
//! This example demonstrates:
//! - Reading metadata from OGG and CAF containers
//! - Understanding channel configuration
//! - Checking if downmixing will be applied

use std::fs::File;
use std::io::BufReader;

fn main() {
    println!("Magnum Opus Metadata Inspector");
    println!("==============================\n");

    // Check both OGG and CAF files
    let files = vec![
        ("example.opus", "OGG"),
        ("example.caf", "CAF"),
    ];

    for (file_path, format) in files {
        println!("Checking {} file: {}", format, file_path);
        println!("-----------------------------------");

        match File::open(file_path) {
            Ok(file) => {
                let reader = BufReader::new(file);

                let metadata_result = match format {
                    "OGG" => magnum::container::ogg::OpusSourceOgg::new(reader)
                        .map(|s| s.metadata),
                    "CAF" => magnum::container::caf::OpusSourceCaf::new(reader)
                        .map(|s| s.metadata),
                    _ => continue,
                };

                match metadata_result {
                    Ok(meta) => {
                        println!("  ✓ Valid Opus file");
                        println!("  Sample Rate: {} Hz", meta.sample_rate);
                        println!("  Channels: {}", meta.channel_count);
                        println!("  Pre-skip: {} samples", meta.preskip);
                        println!("  Output Gain: {}", meta.output_gain);

                        // Show channel configuration
                        let channel_config = match meta.channel_count {
                            1 => "Mono",
                            2 => "Stereo",
                            c if c <= 8 => "Multi-channel",
                            _ => "Unknown",
                        };
                        println!("  Configuration: {}", channel_config);

                        // Note about downmixing
                        if meta.channel_count > 2 {
                            println!("  ⚠ Note: Multi-channel audio will be downmixed to stereo");
                        }
                    }
                    Err(e) => {
                        println!("  ✗ Error reading metadata: {:?}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ✗ File not found: {}", e);
            }
        }
        println!();
    }

    println!("Tips:");
    println!("  - Place 'example.opus' or 'example.caf' in the current directory");
    println!("  - OGG files: ffmpeg -i input.wav -c:a opus example.opus");
    println!("  - CAF files: ffmpeg -i input.wav -c:a libopus -f caf example.caf");
}
