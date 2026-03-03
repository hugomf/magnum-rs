//! FLAC Audio Processing Demo
//!
//! This example demonstrates how to use magnum's FLAC support to:
//! - Create a FLAC source from a file
//! - Access audio metadata
//! - Process audio samples
//! - Handle multi-channel downmixing
//! - Integrate with audio engines

use std::fs::File;
use std::io::BufReader;

// Import the FLAC source directly from the container module (only available with with_flac feature)
#[cfg(feature = "with_flac")]
use magnum::container::flac::FlacSource;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== FLAC Audio Processing Demo ===\n");

    // Example 1: Basic FLAC File Processing
    demo_basic_flac_processing()?;

    // Example 2: Multi-channel Audio Downmixing
    demo_downmixing()?;

    // Example 3: Audio Engine Integration
    demo_audio_engine_integration()?;

    // Example 4: Custom Audio Processing
    demo_custom_processing()?;

    Ok(())
}

#[cfg(feature = "with_flac")]
fn demo_basic_flac_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Basic FLAC File Processing");
    println!("--------------------------------");

    // Try to use a real test FLAC file first
    let test_flac_path = "/Users/hugo/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flac-0.5.0/tests/assets/input-SCPAP.flac";
    
    match File::open(test_flac_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut flac_source = FlacSource::new(reader)?;
            
            println!("✓ FLAC source created successfully from test file");
            println!("  Sample rate: {} Hz", flac_source.metadata.sample_rate);
            println!("  Channel count: {}", flac_source.metadata.channel_count);
            println!("  Output channels: {}", flac_source.output_channels());
            println!("  Downmixing active: {}", flac_source.is_downmixing);

            // Process first few samples
            println!("  First 10 samples:");
            for (i, sample) in flac_source.by_ref().take(10).enumerate() {
                println!("    Sample {}: {:.6}", i + 1, sample);
            }
        }
        Err(_) => {
            // Fallback to a simple message if no test file available
            println!("  Note: No test FLAC file available for this demo.");
            println!("  To test with real FLAC files, use the decode_flac example:");
            println!("  cargo run --example decode_flac --features with_flac");
            println!("  And create a FLAC file with: ffmpeg -f lavfi -i \"sine=frequency=440:duration=5\" -c:a flac example.flac");
        }
    }

    println!();
    Ok(())
}

#[cfg(not(feature = "with_flac"))]
fn demo_basic_flac_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Basic FLAC File Processing");
    println!("--------------------------------");
    println!("  Note: FLAC support not enabled. Enable with_flac feature to run this demo.");
    println!();
    Ok(())
}

#[cfg(feature = "with_flac")]
fn demo_downmixing() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Multi-channel Audio Downmixing");
    println!("----------------------------------");

    // Try to use a real test FLAC file that might be multi-channel
    let test_flac_path = "example.flac";
    
    match File::open(test_flac_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut flac_source = FlacSource::new(reader)?;
            
            println!("✓ FLAC source created successfully");
            println!("  Original channels: {}", flac_source.metadata.channel_count);
            println!("  Output channels: {}", flac_source.output_channels());
            println!("  Downmixing active: {}", flac_source.is_downmixing);

            // Show how downmixing works
            println!("  Downmixing algorithm:");
            println!("    - Odd channels (1,3,5) -> Left channel");
            println!("    - Even channels (2,4,6) -> Right channel");
            println!("    - Volume normalized by channel count");

            // Process some samples to demonstrate downmixing
            let samples: Vec<f32> = flac_source.by_ref().take(8).collect();
            println!("  Processed {} samples (downmixed to stereo)", samples.len());
        }
        Err(_) => {
            println!("  Note: No test FLAC file available for downmixing demo.");
            println!("  To test downmixing with real multi-channel FLAC files:");
            println!("  1. Create a 6-channel FLAC file: ffmpeg -f lavfi -i \"sine=frequency=440:duration=5\" -ac 6 -c:a flac example_6ch.flac");
            println!("  2. Run: cargo run --example decode_flac --features with_flac");
        }
    }

    println!();
    Ok(())
}

#[cfg(not(feature = "with_flac"))]
fn demo_downmixing() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Multi-channel Audio Downmixing");
    println!("----------------------------------");
    println!("  Note: FLAC support not enabled. Enable with_flac feature to run this demo.");
    println!();
    Ok(())
}

#[cfg(feature = "with_flac")]
fn demo_audio_engine_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Audio Engine Integration");
    println!("----------------------------");

    // Try to use a real test FLAC file
    let test_flac_path = "/Users/hugo/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flac-0.5.0/tests/assets/input-SCPAP.flac";
    
    match File::open(test_flac_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let _flac_source = FlacSource::new(reader)?;

            // Show integration capabilities
            println!("✓ FLAC source ready for audio engines");

            #[cfg(feature = "with_rodio")]
            {
                println!("  Rodio integration:");
                println!("    - Implements rodio::source::Source trait");
                println!("    - Can be used directly with rodio audio engine");
                println!("    - Supports streaming playback");
            }

            #[cfg(feature = "with_kira")]
            {
                println!("  Kira integration:");
                println!("    - Implements kira::audio_stream::AudioStream trait");
                println!("    - Can be used directly with kira audio engine");
                println!("    - Supports real-time audio processing");
            }

            #[cfg(not(any(feature = "with_rodio", feature = "with_kira")))]
            {
                println!("  Note: Enable with_rodio or with_kira features for audio engine integration");
            }
        }
        Err(_) => {
            println!("  Note: No test FLAC file available for integration demo.");
            println!("  To test with real FLAC files and audio engines:");
            println!("  1. Create a FLAC file: ffmpeg -f lavfi -i \"sine=frequency=440:duration=5\" -c:a flac example.flac");
            println!("  2. Enable audio engine features: cargo run --example decode_flac --features with_flac,with_rodio");
        }
    }

    println!();
    Ok(())
}

#[cfg(not(feature = "with_flac"))]
fn demo_audio_engine_integration() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Audio Engine Integration");
    println!("----------------------------");
    println!("  Note: FLAC support not enabled. Enable with_flac feature to run this demo.");
    println!();
    Ok(())
}

#[cfg(feature = "with_flac")]
fn demo_custom_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Custom Audio Processing");
    println!("---------------------------");

    // Try to use a real test FLAC file
    let test_flac_path = "/Users/hugo/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/flac-0.5.0/tests/assets/input-SCPAP.flac";
    
    match File::open(test_flac_path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut flac_source = FlacSource::new(reader)?;

            println!("✓ Custom audio processing demo");

            // Calculate audio statistics
            let mut sample_count = 0;
            let mut sum = 0.0f32;
            let mut max_abs = 0.0f32;

            // Process samples and calculate statistics
            for sample in flac_source.by_ref().take(1000) {
                sample_count += 1;
                sum += sample;
                max_abs = max_abs.max(sample.abs());
            }

            let average = if sample_count > 0 { sum / sample_count as f32 } else { 0.0 };

            println!("  Processed {} samples", sample_count);
            println!("  Average amplitude: {:.6}", average);
            println!("  Peak amplitude: {:.6}", max_abs);
            println!("  Sample rate: {} Hz", flac_source.metadata.sample_rate);

            // Calculate duration
            let duration_seconds = sample_count as f64 / flac_source.metadata.sample_rate as f64;
            println!("  Duration: {:.2} seconds", duration_seconds);
        }
        Err(_) => {
            println!("  Note: No test FLAC file available for custom processing demo.");
            println!("  To test custom processing with real FLAC files:");
            println!("  1. Create a FLAC file: ffmpeg -f lavfi -i \"sine=frequency=440:duration=5\" -c:a flac example.flac");
            println!("  2. Run: cargo run --example decode_flac --features with_flac");
        }
    }

    println!();
    Ok(())
}

#[cfg(not(feature = "with_flac"))]
fn demo_custom_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Custom Audio Processing");
    println!("---------------------------");
    println!("  Note: FLAC support not enabled. Enable with_flac feature to run this demo.");
    println!();
    Ok(())
}

