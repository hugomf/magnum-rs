//! FLAC Audio Processing Demo
//!
//! This example demonstrates how to use magnum's FLAC support to:
//! - Create a FLAC source from a file
//! - Access audio metadata
//! - Process audio samples
//! - Handle multi-channel downmixing
//! - Integrate with audio engines

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::time::Duration;

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

    // In a real application, you would open an actual FLAC file:
    // let file = File::open("path/to/audio.flac")?;
    // let reader = BufReader::new(file);
    // let mut flac_source = FlacSource::new(reader)?;

    // For this demo, we'll use mock data to show the API structure
    let mock_flac_data = create_mock_flac_data();
    let cursor = Cursor::new(mock_flac_data);
    let mut flac_source = FlacSource::new(cursor)?;

    println!("✓ FLAC source created successfully");
    println!("  Sample rate: {} Hz", flac_source.metadata.sample_rate);
    println!("  Channel count: {}", flac_source.metadata.channel_count);
    println!("  Output channels: {}", flac_source.output_channels());
    println!("  Downmixing active: {}", flac_source.is_downmixing);

    // Process first few samples
    println!("  First 10 samples:");
    for (i, sample) in flac_source.by_ref().take(10).enumerate() {
        println!("    Sample {}: {:.6}", i + 1, sample);
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

    // Simulate a 5.1 surround sound FLAC file
    let mock_51_data = create_mock_multichannel_data(6); // 6 channels = 5.1
    let cursor = Cursor::new(mock_51_data);
    let mut flac_source = FlacSource::new(cursor)?;

    println!("✓ 5.1 surround sound FLAC source created");
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

    let mock_data = create_mock_flac_data();
    let cursor = Cursor::new(mock_data);

    // Create FLAC source
    let flac_source = FlacSource::new(cursor)?;

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

    let mock_data = create_mock_flac_data();
    let cursor = Cursor::new(mock_data);
    let mut flac_source = FlacSource::new(cursor)?;

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

// Helper functions to create mock data for demonstration

fn create_mock_flac_data() -> Vec<u8> {
    // Create mock FLAC data (not actual FLAC, just for API demonstration)
    vec![
        0x66, 0x4C, 0x61, 0x43, // FLAC signature
        0x00, 0x00, 0x00, 0x00, // Mock header data
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ]
}

fn create_mock_multichannel_data(channels: u8) -> Vec<u8> {
    // Create mock multi-channel audio data
    let mut data = vec![
        0x66, 0x4C, 0x61, 0x43, // FLAC signature
        0x00, 0x00, 0x00, 0x00, // Mock header with channel info
    ];
    
    // Add channel count to mock data
    data.push(channels);
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_data_creation() {
        let data = create_mock_flac_data();
        assert!(data.len() > 0);
        assert_eq!(data[0], 0x66); // 'f'
        assert_eq!(data[1], 0x4C); // 'L'
        assert_eq!(data[2], 0x61); // 'a'
        assert_eq!(data[3], 0x43); // 'C'
    }

    #[test]
    fn test_multichannel_data_creation() {
        let data = create_mock_multichannel_data(6);
        assert!(data.len() > 0);
        assert_eq!(data[8], 6); // Channel count
    }
}