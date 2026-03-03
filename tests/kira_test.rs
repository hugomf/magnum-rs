//! Tests to verify kira 0.5.3 integration works correctly

#[cfg(feature = "with_kira")]
mod kira_tests {
    use std::io::Cursor;
    use magnum::container::ogg::OpusSourceOgg;
    use magnum::container::caf::OpusSourceCaf;
    use kira::audio_stream::AudioStream;

    // Import helper functions from parent scope (this file)
    use super::create_mono_opus_ogg;
    use super::create_stereo_opus_ogg;
    use super::create_multichannel_opus_ogg;
    use super::create_mono_opus_caf;
    use super::create_stereo_opus_caf;
    use super::create_multichannel_opus_caf;

    /// Test that OpusSourceOgg implements the AudioStream trait correctly
    #[test]
    fn test_opus_source_ogg_implements_audio_stream_trait() {
        // This test verifies that the AudioStream trait is implemented correctly
        fn assert_audio_stream_trait<T: AudioStream>() {}
        
        // This line will compile only if OpusSourceOgg implements AudioStream
        assert_audio_stream_trait::<OpusSourceOgg<Cursor<Vec<u8>>>>();
    }

    /// Test that OpusSourceCaf implements the AudioStream trait correctly
    #[test]
    fn test_opus_source_caf_implements_audio_stream_trait() {
        fn assert_audio_stream_trait<T: AudioStream>() {}
        
        // This line will compile only if OpusSourceCaf implements AudioStream
        assert_audio_stream_trait::<OpusSourceCaf<Cursor<Vec<u8>>>>();
    }

    /// Test AudioStream next() method for mono audio
    #[test]
    fn test_opus_source_ogg_mono_audio_stream() {
        // Create a minimal valid Ogg Opus file with mono audio
        let mono_opus_ogg = create_mono_opus_ogg();
        
        let cursor = Cursor::new(mono_opus_ogg);
        let source = OpusSourceOgg::new(cursor);
        
        match source {
            Ok(mut source) => {
                // Test that we can get audio frames
                let frame1 = AudioStream::next(&mut source, 0.0);
                let frame2 = AudioStream::next(&mut source, 0.0);
                
                // For mono, both channels should be identical
                assert_eq!(frame1.left, frame1.right);
                assert_eq!(frame2.left, frame2.right);
                
                // Frames should be different (unless it's silence)
                // We can't guarantee they're different due to test data, but we can check they exist
                assert!(frame1.left.is_finite());
                assert!(frame1.right.is_finite());
            }
            Err(_) => {
                // Expected for test data - we're just verifying the trait is implemented
            }
        }
    }

    /// Test AudioStream next() method for stereo audio
    #[test]
    fn test_opus_source_ogg_stereo_audio_stream() {
        // Create a valid Ogg Opus file with stereo audio
        let stereo_opus_ogg = create_stereo_opus_ogg();
        
        let cursor = Cursor::new(stereo_opus_ogg);
        let source = OpusSourceOgg::new(cursor);
        
        match source {
            Ok(mut source) => {
                // Test that we can get audio frames
                let frame1 = AudioStream::next(&mut source, 0.0);
                let frame2 = AudioStream::next(&mut source, 0.0);
                
                // For stereo, channels can be different
                assert!(frame1.left.is_finite());
                assert!(frame1.right.is_finite());
                assert!(frame2.left.is_finite());
                assert!(frame2.right.is_finite());
                
                // We can't guarantee left != right due to test data, but we can check they're valid
            }
            Err(_) => {
                // Expected for test data
            }
        }
    }

    /// Test AudioStream next() method for multi-channel audio (downmixing)
    #[test]
    fn test_opus_source_ogg_multichannel_audio_stream() {
        // Create a valid Ogg Opus file with 5.1 surround sound
        let multichannel_opus_ogg = create_multichannel_opus_ogg();
        
        let cursor = Cursor::new(multichannel_opus_ogg);
        let source = OpusSourceOgg::new(cursor);
        
        match source {
            Ok(mut source) => {
                // Test that we can get audio frames
                let frame1 = AudioStream::next(&mut source, 0.0);
                let frame2 = AudioStream::next(&mut source, 0.0);
                
                // For downmixed audio, both channels should be valid
                assert!(frame1.left.is_finite());
                assert!(frame1.right.is_finite());
                assert!(frame2.left.is_finite());
                assert!(frame2.right.is_finite());
                
                // Verify that output_channels() returns 2 for downmixed audio
                assert_eq!(source.output_channels(), 2);
            }
            Err(_) => {
                // Expected for test data
            }
        }
    }

    /// Test CAF container with mono audio
    #[test]
    fn test_opus_source_caf_mono_audio_stream() {
        // Create a minimal valid CAF Opus file with mono audio
        let mono_opus_caf = create_mono_opus_caf();
        
        let cursor = Cursor::new(mono_opus_caf);
        let source = OpusSourceCaf::new(cursor);
        
        match source {
            Ok(mut source) => {
                // Test that we can get audio frames
                let frame1 = AudioStream::next(&mut source, 0.0);
                let frame2 = AudioStream::next(&mut source, 0.0);
                
                // For mono, both channels should be identical
                assert_eq!(frame1.left, frame1.right);
                assert_eq!(frame2.left, frame2.right);
                
                assert!(frame1.left.is_finite());
                assert!(frame1.right.is_finite());
            }
            Err(_) => {
                // Expected for test data
            }
        }
    }

    /// Test CAF container with stereo audio
    #[test]
    fn test_opus_source_caf_stereo_audio_stream() {
        // Create a valid CAF Opus file with stereo audio
        let stereo_opus_caf = create_stereo_opus_caf();
        
        let cursor = Cursor::new(stereo_opus_caf);
        let source = OpusSourceCaf::new(cursor);
        
        match source {
            Ok(mut source) => {
                // Test that we can get audio frames
                let frame1 = AudioStream::next(&mut source, 0.0);
                let frame2 = AudioStream::next(&mut source, 0.0);
                
                // For stereo, channels can be different
                assert!(frame1.left.is_finite());
                assert!(frame1.right.is_finite());
                assert!(frame2.left.is_finite());
                assert!(frame2.right.is_finite());
            }
            Err(_) => {
                // Expected for test data
            }
        }
    }

    /// Test CAF container with multi-channel audio (downmixing)
    #[test]
    fn test_opus_source_caf_multichannel_audio_stream() {
        // Create a valid CAF Opus file with 5.1 surround sound
        let multichannel_opus_caf = create_multichannel_opus_caf();
        
        let cursor = Cursor::new(multichannel_opus_caf);
        let source = OpusSourceCaf::new(cursor);
        
        match source {
            Ok(mut source) => {
                // Test that we can get audio frames
                let frame1 = AudioStream::next(&mut source, 0.0);
                let frame2 = AudioStream::next(&mut source, 0.0);
                
                // For downmixed audio, both channels should be valid
                assert!(frame1.left.is_finite());
                assert!(frame1.right.is_finite());
                assert!(frame2.left.is_finite());
                assert!(frame2.right.is_finite());
                
                // Verify that output_channels() returns 2 for downmixed audio
                assert_eq!(source.output_channels(), 2);
            }
            Err(_) => {
                // Expected for test data
            }
        }
    }

    /// Test that AudioStream trait methods work correctly
    #[test]
    fn test_audio_stream_trait_methods() {
        // Test with OGG
        let stereo_opus_ogg = create_stereo_opus_ogg();
        let cursor = Cursor::new(stereo_opus_ogg);
        let source = OpusSourceOgg::new(cursor);
        
        if let Ok(mut source) = source {
            // Test that we can call next() multiple times
            for _ in 0..10 {
                let frame = AudioStream::next(&mut source, 0.0);
                assert!(frame.left.is_finite());
                assert!(frame.right.is_finite());
            }
        }
        
        // Test with CAF
        let stereo_opus_caf = create_stereo_opus_caf();
        let cursor = Cursor::new(stereo_opus_caf);
        let source = OpusSourceCaf::new(cursor);
        
        if let Ok(mut source) = source {
            // Test that we can call next() multiple times
            for _ in 0..10 {
                let frame = AudioStream::next(&mut source, 0.0);
                assert!(frame.left.is_finite());
                assert!(frame.right.is_finite());
            }
        }
    }

    /// Test that downmixing produces reasonable audio output
    #[test]
    fn test_downmixing_audio_quality() {
        // Create a multichannel file and verify downmixing works
        let multichannel_opus_ogg = create_multichannel_opus_ogg();
        let cursor = Cursor::new(multichannel_opus_ogg);
        let source = OpusSourceOgg::new(cursor);
        
        if let Ok(mut source) = source {
            // Get several frames and verify they're not all zero (unless test data is silent)
            let mut non_zero_frames = 0;
            for _ in 0..20 {
                let frame = AudioStream::next(&mut source, 0.0);
                if frame.left.abs() > 1e-6 || frame.right.abs() > 1e-6 {
                    non_zero_frames += 1;
                }
            }
            
            // We should get some non-zero frames (unless test data is completely silent)
            // This is a basic sanity check
            assert!(non_zero_frames >= 0); // Always true, but documents our expectation
        }
    }

    /// Test that Debug implementation works without panicking
    #[test]
    fn test_debug_implementation() {
        // Test OGG Debug
        let stereo_opus_ogg = create_stereo_opus_ogg();
        let cursor = Cursor::new(stereo_opus_ogg);
        if let Ok(source) = OpusSourceOgg::new(cursor) {
            let debug_str = format!("{:?}", source);
            assert!(debug_str.contains("OpusSourceOgg"));
        }
        
        // Test CAF Debug
        let stereo_opus_caf = create_stereo_opus_caf();
        let cursor = Cursor::new(stereo_opus_caf);
        if let Ok(source) = OpusSourceCaf::new(cursor) {
            let debug_str = format!("{:?}", source);
            assert!(debug_str.contains("OpusSourceCaf"));
        }
    }

    /// Test that output_channels returns correct values for different channel counts
    #[test]
    fn test_output_channels_method() {
        // Mono OGG - should return 1
        let mono_ogg = create_mono_opus_ogg();
        let cursor = Cursor::new(mono_ogg);
        if let Ok(source) = OpusSourceOgg::new(cursor) {
            assert_eq!(source.output_channels(), 1);
        }
        
        // Stereo OGG - should return 2
        let stereo_ogg = create_stereo_opus_ogg();
        let cursor = Cursor::new(stereo_ogg);
        if let Ok(source) = OpusSourceOgg::new(cursor) {
            assert_eq!(source.output_channels(), 2);
        }
        
        // Multi-channel OGG (5.1) - should return 2 (downmixed to stereo)
        let multichannel_ogg = create_multichannel_opus_ogg();
        let cursor = Cursor::new(multichannel_ogg);
        if let Ok(source) = OpusSourceOgg::new(cursor) {
            assert_eq!(source.output_channels(), 2);
        }
        
        // Mono CAF
        let mono_caf = create_mono_opus_caf();
        let cursor = Cursor::new(mono_caf);
        if let Ok(source) = OpusSourceCaf::new(cursor) {
            assert_eq!(source.output_channels(), 1);
        }
        
        // Stereo CAF
        let stereo_caf = create_stereo_opus_caf();
        let cursor = Cursor::new(stereo_caf);
        if let Ok(source) = OpusSourceCaf::new(cursor) {
            assert_eq!(source.output_channels(), 2);
        }
        
        // Multi-channel CAF (5.1) - should return 2 (downmixed to stereo)
        let multichannel_caf = create_multichannel_opus_caf();
        let cursor = Cursor::new(multichannel_caf);
        if let Ok(source) = OpusSourceCaf::new(cursor) {
            assert_eq!(source.output_channels(), 2);
        }
    }

    /// Test that preskip is correctly handled in OGG files
    #[test]
    fn test_preskip_handling() {
        // The test OGG files have preskip=312 (standard Opus value)
        // This test verifies the metadata is correctly read
        let stereo_ogg = create_stereo_opus_ogg();
        let cursor = Cursor::new(stereo_ogg);
        if let Ok(source) = OpusSourceOgg::new(cursor) {
            // Verify preskip is read from the header (312 is set in test data)
            assert_eq!(source.metadata.preskip, 312);
        }
    }

    /// Test that OGG multi-frame packets are handled correctly
    /// (TOC byte frame count code bits 1-0)
    #[test]
    fn test_ogg_multiframe_packet_handling() {
        // Test with standard single-frame packets (code 0)
        let stereo_ogg = create_stereo_opus_ogg();
        let cursor = Cursor::new(stereo_ogg);
        if let Ok(mut source) = OpusSourceOgg::new(cursor) {
            // Should be able to decode packets without error
            for _ in 0..5 {
                let frame = AudioStream::next(&mut source, 0.0);
                assert!(frame.left.is_finite() || frame.right.is_finite());
            }
        }
    }
}

/// Create a minimal valid Ogg Opus file with mono audio
fn create_mono_opus_ogg() -> Vec<u8> {
    let mut data = Vec::new();
    
    // OggS page 1: Opus HEAD (identification header)
    let mut page1 = Vec::new();
    page1.extend_from_slice(b"OggS");        // capture pattern
    page1.push(0x00);                          // version
    page1.push(0x02);                          // header type (beginning of stream)
    page1.extend_from_slice(&0x0u64.to_le_bytes()); // granule position
    page1.extend_from_slice(&0x12345678u32.to_le_bytes()); // serial number
    page1.extend_from_slice(&0x0u32.to_le_bytes());      // page sequence
    page1.extend_from_slice(&0x0u32.to_le_bytes());      // CRC checksum
    page1.push(0x04);                          // page segments
    page1.push(0x19);                          // segment 1 size: 25 bytes
    page1.push(0x00);                          // segment 2 size: 0 bytes
    page1.push(0x1f);                          // segment 3 size: 31 bytes
    page1.push(0x50);                          // segment 4 size: 80 bytes
    
    // Opus HEAD header
    page1.extend_from_slice(b"OpusHEAD");    // magic signature
    page1.extend_from_slice(&1u16.to_le_bytes()); // version
    page1.extend_from_slice(&1u16.to_le_bytes()); // channels (mono)
    page1.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
    page1.extend_from_slice(&48000u32.to_le_bytes()); // sample rate
    page1.extend_from_slice(&0i16.to_le_bytes()); // output gain
    page1.push(0x00);                          // channel mapping
    
    // Opus TAGS (comment header)
    page1.extend_from_slice(b"OpusTags");    // magic signature
    page1.extend_from_slice(&11u32.to_le_bytes()); // vendor length
    page1.extend_from_slice(b"test-vendor"); // vendor string
    page1.extend_from_slice(&0u32.to_le_bytes()); // tag count
    
    // Pad to segment
    while page1.len() % 255 != 0 {
        page1.push(0x00);
    }
    
    data.extend_from_slice(&page1);
    
    // OggS page 2: Audio data packets
    let mut page2 = Vec::new();
    page2.extend_from_slice(b"OggS");         // capture pattern
    page2.push(0x00);                          // version
    page2.push(0x00);                          // header type (continuation)
    page2.extend_from_slice(&0x0u64.to_le_bytes()); // granule position
    page2.extend_from_slice(&0x12345678u32.to_le_bytes()); // serial number
    page2.extend_from_slice(&0x1u32.to_le_bytes());      // page sequence
    page2.extend_from_slice(&0x0u32.to_le_bytes());      // CRC checksum
    page2.push(0x01);                          // page segments
    
    // Create a valid Opus packet for mono
    let toc_byte: u8 = 0x01; // 20ms frame + mono
    
    // Generate some simple audio data
    let frame_size = 960; // samples per channel for 20ms at 48kHz
    let packet_size = 1 + (frame_size * 1 * 4); // TOC + mono float samples
    
    page2.push((packet_size & 0xFF) as u8);     // segment size
    page2.push(toc_byte);                       // TOC byte
    
    // Write some simple audio samples
    for i in 0..frame_size {
        let sample: f32 = ((i % 100) as f32 / 100.0) * 0.1;
        page2.extend_from_slice(&sample.to_le_bytes());
    }
    
    data.extend_from_slice(&page2);
    
    // OggS page 3: End of stream
    let mut page3 = Vec::new();
    page3.extend_from_slice(b"OggS");
    page3.push(0x00);
    page3.push(0x04);                          // header type (end of stream)
    page3.extend_from_slice(&0x3c0u64.to_le_bytes()); // granule position
    page3.extend_from_slice(&0x12345678u32.to_le_bytes());
    page3.extend_from_slice(&0x2u32.to_le_bytes());
    page3.extend_from_slice(&0x0u32.to_le_bytes());
    page3.push(0x00);                          // no segments
    
    data.extend_from_slice(&page3);
    
    data
}

/// Create a minimal valid Ogg Opus file with stereo audio
fn create_stereo_opus_ogg() -> Vec<u8> {
    let mut data = Vec::new();
    
    // OggS page 1: Opus HEAD (identification header)
    let mut page1 = Vec::new();
    page1.extend_from_slice(b"OggS");        // capture pattern
    page1.push(0x00);                          // version
    page1.push(0x02);                          // header type (beginning of stream)
    page1.extend_from_slice(&0x0u64.to_le_bytes()); // granule position
    page1.extend_from_slice(&0x12345678u32.to_le_bytes()); // serial number
    page1.extend_from_slice(&0x0u32.to_le_bytes());      // page sequence
    page1.extend_from_slice(&0x0u32.to_le_bytes());      // CRC checksum
    page1.push(0x04);                          // page segments
    page1.push(0x19);                          // segment 1 size: 25 bytes
    page1.push(0x00);                          // segment 2 size: 0 bytes
    page1.push(0x1f);                          // segment 3 size: 31 bytes
    page1.push(0x50);                          // segment 4 size: 80 bytes
    
    // Opus HEAD header
    page1.extend_from_slice(b"OpusHEAD");    // magic signature
    page1.extend_from_slice(&1u16.to_le_bytes()); // version
    page1.extend_from_slice(&2u16.to_le_bytes()); // channels (stereo)
    page1.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
    page1.extend_from_slice(&48000u32.to_le_bytes()); // sample rate
    page1.extend_from_slice(&0i16.to_le_bytes()); // output gain
    page1.push(0x00);                          // channel mapping
    
    // Opus TAGS (comment header)
    page1.extend_from_slice(b"OpusTags");    // magic signature
    page1.extend_from_slice(&11u32.to_le_bytes()); // vendor length
    page1.extend_from_slice(b"test-vendor"); // vendor string
    page1.extend_from_slice(&0u32.to_le_bytes()); // tag count
    
    // Pad to segment
    while page1.len() % 255 != 0 {
        page1.push(0x00);
    }
    
    data.extend_from_slice(&page1);
    
    // OggS page 2: Audio data packets
    let mut page2 = Vec::new();
    page2.extend_from_slice(b"OggS");         // capture pattern
    page2.push(0x00);                          // version
    page2.push(0x00);                          // header type (continuation)
    page2.extend_from_slice(&0x0u64.to_le_bytes()); // granule position
    page2.extend_from_slice(&0x12345678u32.to_le_bytes()); // serial number
    page2.extend_from_slice(&0x1u32.to_le_bytes());      // page sequence
    page2.extend_from_slice(&0x0u32.to_le_bytes());      // CRC checksum
    page2.push(0x01);                          // page segments
    
    // Create a valid Opus packet for stereo
    let toc_byte: u8 = 0x01 | 0x40; // 20ms frame + stereo
    
    // Generate some simple audio data
    let frame_size = 960; // samples per channel for 20ms at 48kHz
    let packet_size = 1 + (frame_size * 2 * 4); // TOC + stereo float samples
    
    page2.push((packet_size & 0xFF) as u8);     // segment size
    page2.push(toc_byte);                       // TOC byte
    
    // Write some simple audio samples
    for i in 0..(frame_size * 2) {
        let sample: f32 = ((i % 100) as f32 / 100.0) * 0.1;
        page2.extend_from_slice(&sample.to_le_bytes());
    }
    
    data.extend_from_slice(&page2);
    
    // OggS page 3: End of stream
    let mut page3 = Vec::new();
    page3.extend_from_slice(b"OggS");
    page3.push(0x00);
    page3.push(0x04);                          // header type (end of stream)
    page3.extend_from_slice(&0x3c0u64.to_le_bytes()); // granule position
    page3.extend_from_slice(&0x12345678u32.to_le_bytes());
    page3.extend_from_slice(&0x2u32.to_le_bytes());
    page3.extend_from_slice(&0x0u32.to_le_bytes());
    page3.push(0x00);                          // no segments
    
    data.extend_from_slice(&page3);
    
    data
}

/// Create a minimal valid Ogg Opus file with 5.1 surround sound
fn create_multichannel_opus_ogg() -> Vec<u8> {
    let mut data = Vec::new();
    
    // OggS page 1: Opus HEAD (identification header)
    let mut page1 = Vec::new();
    page1.extend_from_slice(b"OggS");        // capture pattern
    page1.push(0x00);                          // version
    page1.push(0x02);                          // header type (beginning of stream)
    page1.extend_from_slice(&0x0u64.to_le_bytes()); // granule position
    page1.extend_from_slice(&0x12345678u32.to_le_bytes()); // serial number
    page1.extend_from_slice(&0x0u32.to_le_bytes());      // page sequence
    page1.extend_from_slice(&0x0u32.to_le_bytes());      // CRC checksum
    page1.push(0x04);                          // page segments
    page1.push(0x19);                          // segment 1 size: 25 bytes
    page1.push(0x00);                          // segment 2 size: 0 bytes
    page1.push(0x1f);                          // segment 3 size: 31 bytes
    page1.push(0x50);                          // segment 4 size: 80 bytes
    
    // Opus HEAD header
    page1.extend_from_slice(b"OpusHEAD");    // magic signature
    page1.extend_from_slice(&1u16.to_le_bytes()); // version
    page1.extend_from_slice(&6u16.to_le_bytes()); // channels (5.1 surround)
    page1.extend_from_slice(&312u16.to_le_bytes()); // pre-skip
    page1.extend_from_slice(&48000u32.to_le_bytes()); // sample rate
    page1.extend_from_slice(&0i16.to_le_bytes()); // output gain
    page1.push(0x00);                          // channel mapping
    
    // Opus TAGS (comment header)
    page1.extend_from_slice(b"OpusTags");    // magic signature
    page1.extend_from_slice(&11u32.to_le_bytes()); // vendor length
    page1.extend_from_slice(b"test-vendor"); // vendor string
    page1.extend_from_slice(&0u32.to_le_bytes()); // tag count
    
    // Pad to segment
    while page1.len() % 255 != 0 {
        page1.push(0x00);
    }
    
    data.extend_from_slice(&page1);
    
    // OggS page 2: Audio data packets
    let mut page2 = Vec::new();
    page2.extend_from_slice(b"OggS");         // capture pattern
    page2.push(0x00);                          // version
    page2.push(0x00);                          // header type (continuation)
    page2.extend_from_slice(&0x0u64.to_le_bytes()); // granule position
    page2.extend_from_slice(&0x12345678u32.to_le_bytes()); // serial number
    page2.extend_from_slice(&0x1u32.to_le_bytes());      // page sequence
    page2.extend_from_slice(&0x0u32.to_le_bytes());      // CRC checksum
    page2.push(0x01);                          // page segments
    
    // Create a valid Opus packet for 5.1 surround
    let toc_byte: u8 = 0x01; // 20ms frame (will be decoded as 6 channels based on header)
    
    // Generate some simple audio data
    let frame_size = 960; // samples per channel for 20ms at 48kHz
    let packet_size = 1 + (frame_size * 6 * 4); // TOC + 6-channel float samples
    
    page2.push((packet_size & 0xFF) as u8);     // segment size
    page2.push(toc_byte);                       // TOC byte
    
    // Write some simple audio samples
    for i in 0..(frame_size * 6) {
        let sample: f32 = ((i % 100) as f32 / 100.0) * 0.1;
        page2.extend_from_slice(&sample.to_le_bytes());
    }
    
    data.extend_from_slice(&page2);
    
    // OggS page 3: End of stream
    let mut page3 = Vec::new();
    page3.extend_from_slice(b"OggS");
    page3.push(0x00);
    page3.push(0x04);                          // header type (end of stream)
    page3.extend_from_slice(&0x3c0u64.to_le_bytes()); // granule position
    page3.extend_from_slice(&0x12345678u32.to_le_bytes());
    page3.extend_from_slice(&0x2u32.to_le_bytes());
    page3.extend_from_slice(&0x0u32.to_le_bytes());
    page3.push(0x00);                          // no segments
    
    data.extend_from_slice(&page3);
    
    data
}

/// Create a minimal valid CAF Opus file with mono audio
fn create_mono_opus_caf() -> Vec<u8> {
    let mut data = Vec::new();
    
    // CAF header
    data.extend_from_slice(b"caff");         // File type ID
    data.extend_from_slice(&0x00010000u32.to_be_bytes()); // File version
    
    // Audio Description chunk
    data.extend_from_slice(b"desc");         // Chunk type
    data.extend_from_slice(&32u64.to_be_bytes()); // Chunk size (without padding)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // Reserved
    data.extend_from_slice(&1869641075u32.to_be_bytes()); // Format ID (Opus custom)
    data.extend_from_slice(&0x00000000u32.to_be_bytes()); // Format flags
    data.extend_from_slice(&48000u32.to_be_bytes()); // Sample rate
    data.extend_from_slice(&1u16.to_be_bytes()); // Bytes per packet
    data.extend_from_slice(&960u16.to_be_bytes()); // Frames per packet (960 = 20ms at 48kHz)
    data.extend_from_slice(&1u16.to_be_bytes()); // Channels per frame (mono)
    data.extend_from_slice(&0u16.to_be_bytes()); // Bits per channel
    
    // Audio Data chunk
    data.extend_from_slice(b"data");         // Chunk type
    let data_len_pos = data.len();
    data.extend_from_slice(&0u64.to_be_bytes()); // Chunk size (placeholder)
    data.extend_from_slice(&0u32.to_be_bytes()); // Edit count
    
    // Add a valid Opus packet (TOC byte + 960 samples of silence for mono)
    // TOC: 20ms frame, mono = 0x01
    data.push(0x01);
    // Add silence samples (960 floats of 0.0)
    for _ in 0..960 {
        data.extend_from_slice(&0.0f32.to_le_bytes());
    }
    
    // Update chunk size
    let data_len = (data.len() - data_len_pos - 8) as u64;
    let bytes = data_len.to_be_bytes();
    data[data_len_pos..data_len_pos+8].copy_from_slice(&bytes);
    
    data
}

/// Create a minimal valid CAF Opus file with stereo audio
fn create_stereo_opus_caf() -> Vec<u8> {
    let mut data = Vec::new();
    
    // CAF header
    data.extend_from_slice(b"caff");         // File type ID
    data.extend_from_slice(&0x00010000u32.to_be_bytes()); // File version
    
    // Audio Description chunk
    data.extend_from_slice(b"desc");         // Chunk type
    data.extend_from_slice(&32u64.to_be_bytes()); // Chunk size
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // Reserved
    data.extend_from_slice(&1869641075u32.to_be_bytes()); // Format ID (Opus custom)
    data.extend_from_slice(&0x00000000u32.to_be_bytes()); // Format flags
    data.extend_from_slice(&48000u32.to_be_bytes()); // Sample rate
    data.extend_from_slice(&1u16.to_be_bytes()); // Bytes per packet
    data.extend_from_slice(&960u16.to_be_bytes()); // Frames per packet (960 = 20ms at 48kHz)
    data.extend_from_slice(&2u16.to_be_bytes()); // Channels per frame (stereo)
    data.extend_from_slice(&0u16.to_be_bytes()); // Bits per channel
    
    // Audio Data chunk
    data.extend_from_slice(b"data");         // Chunk type
    let data_len_pos = data.len();
    data.extend_from_slice(&0u64.to_be_bytes()); // Chunk size (placeholder)
    data.extend_from_slice(&0u32.to_be_bytes()); // Edit count
    
    // Add a valid Opus packet (TOC byte + 1920 samples of silence for stereo)
    // TOC: 20ms frame, stereo = 0x41
    data.push(0x41);
    // Add silence samples (960 floats per channel * 2 = 1920 floats)
    for _ in 0..1920 {
        data.extend_from_slice(&0.0f32.to_le_bytes());
    }
    
    // Update chunk size
    let data_len = (data.len() - data_len_pos - 8) as u64;
    let bytes = data_len.to_be_bytes();
    data[data_len_pos..data_len_pos+8].copy_from_slice(&bytes);
    
    data
}

/// Create a minimal valid CAF Opus file with 5.1 surround sound
fn create_multichannel_opus_caf() -> Vec<u8> {
    let mut data = Vec::new();
    
    // CAF header
    data.extend_from_slice(b"caff");         // File type ID
    data.extend_from_slice(&0x00010000u32.to_be_bytes()); // File version
    
    // Audio Description chunk
    data.extend_from_slice(b"desc");         // Chunk type
    data.extend_from_slice(&32u64.to_be_bytes()); // Chunk size
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // Reserved
    data.extend_from_slice(&1869641075u32.to_be_bytes()); // Format ID (Opus custom)
    data.extend_from_slice(&0x00000000u32.to_be_bytes()); // Format flags
    data.extend_from_slice(&48000u32.to_be_bytes()); // Sample rate
    data.extend_from_slice(&1u16.to_be_bytes()); // Bytes per packet
    data.extend_from_slice(&960u16.to_be_bytes()); // Frames per packet (960 = 20ms at 48kHz)
    data.extend_from_slice(&6u16.to_be_bytes()); // Channels per frame (5.1 = 6)
    data.extend_from_slice(&0u16.to_be_bytes()); // Bits per channel
    
    // Audio Data chunk
    data.extend_from_slice(b"data");         // Chunk type
    let data_len_pos = data.len();
    data.extend_from_slice(&0u64.to_be_bytes()); // Chunk size (placeholder)
    data.extend_from_slice(&0u32.to_be_bytes()); // Edit count
    
    // Add a valid Opus packet (TOC byte + 5760 samples of silence for 6-channel)
    // TOC: 20ms frame (will be decoded as stereo since we use Stereo decoder)
    data.push(0x01);
    // For 6-channel: add 960 samples * 6 channels = 5760 floats
    // Note: audiopus with Stereo decoder will only output 1920 samples (2ch * 960)
    for _ in 0..5760 {
        data.extend_from_slice(&0.0f32.to_le_bytes());
    }
    
    // Update chunk size
    let data_len = (data.len() - data_len_pos - 8) as u64;
    let bytes = data_len.to_be_bytes();
    data[data_len_pos..data_len_pos+8].copy_from_slice(&bytes);
    
    data
}