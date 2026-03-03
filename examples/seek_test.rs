//! Test seek functionality for OGG Opus files
//! 
//! Run with: cargo run --example seek_test --features "with_ogg"

use std::fs::File;
use std::io::BufReader;
use magnum::container::ogg::OpusSourceOgg;

fn main() {
    // Open the example opus file
    let file = match File::open("example.opus") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error opening example.opus: {}. Make sure you have an opus file.", e);
            return;
        }
    };
    
    let reader = BufReader::new(file);
    
    // Create the Opus source
    let mut source = match OpusSourceOgg::new(reader) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error creating OpusSourceOgg: {:?}", e);
            return;
        }
    };
    
    println!("Opened opus file:");
    println!("  Sample rate: {} Hz", source.metadata.sample_rate);
    println!("  Channels: {}", source.metadata.channel_count);
    println!("  Pre-skip: {}", source.metadata.preskip);
    println!("  Duration: ~4.2 seconds");
    println!();
    
    // Test seeking to different positions - DON'T call seek(0) first
    // Just read from the start without seeking
    print!("Read from start (no seek): ");
    let samples: Vec<f32> = (0..5).filter_map(|_| source.next()).collect();
    for s in &samples {
        print!("{:.4} ", s);
    }
    println!();
    
    // Now test seeking to 1 second
    println!("\n--- Seeking to 1 second (48000 samples) ---");
    match source.seek(48000) {
        Ok(pos) => {
            print!("First 5 samples after seek: ");
            let samples: Vec<f32> = (0..5).filter_map(|_| source.next()).collect();
            for s in &samples {
                print!("{:.4} ", s);
            }
            println!("\nSeek returned position: {}", pos);
        }
        Err(e) => eprintln!("Seek failed: {:?}", e),
    }
    
    // Now test seeking to 2 seconds
    println!("\n--- Seeking to 2 seconds (96000 samples) ---");
    match source.seek(96000) {
        Ok(pos) => {
            print!("First 5 samples after seek: ");
            let samples: Vec<f32> = (0..5).filter_map(|_| source.next()).collect();
            for s in &samples {
                print!("{:.4} ", s);
            }
            println!("\nSeek returned position: {}", pos);
        }
        Err(e) => eprintln!("Seek failed: {:?}", e),
    }
    
    // Now test seeking to 3 seconds
    println!("\n--- Seeking to 3 seconds (144000 samples) ---");
    match source.seek(144000) {
        Ok(pos) => {
            print!("First 5 samples after seek: ");
            let samples: Vec<f32> = (0..5).filter_map(|_| source.next()).collect();
            for s in &samples {
                print!("{:.4} ", s);
            }
            println!("\nSeek returned position: {}", pos);
        }
        Err(e) => eprintln!("Seek failed: {:?}", e),
    }
    
    println!("\nSeek test completed!");
}
