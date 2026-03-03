//! MKV container tests

#[cfg(feature = "with_mkv")]
mod tests {
    use std::io::Cursor;
    use magnum::{is_mkv_stream, create_mkv_source};

    /// Test MKV format detection with proper EBML header
    #[test]
    fn test_mkv_format_detection() {
        // EBML header starts with 0x1A45DFA3
        let ebml_header: [u8; 4] = [0x1A, 0x45, 0xDF, 0xA3];
        
        // Create a minimal MKV-like stream
        let mut cursor = Cursor::new(ebml_header.to_vec());
        
        // Test the actual format detection function
        let result = is_mkv_stream(&mut cursor);
        assert!(result.is_ok(), "Format detection should not fail");
        assert_eq!(result.unwrap(), true, "Should detect valid MKV header");
    }

    /// Test that non-MKV streams are not detected as MKV
    #[test]
    fn test_non_mkv_format_detection() {
        // FLAC starts with "fLaC"
        let flac_magic: [u8; 4] = [b'f', b'L', b'a', b'C'];
        let mut cursor = Cursor::new(flac_magic.to_vec());
        
        let result = is_mkv_stream(&mut cursor);
        assert!(result.is_ok(), "Format detection should not fail");
        assert_eq!(result.unwrap(), false, "Should not detect FLAC as MKV");
        
        // OGG starts with "OggS"
        let ogg_magic: [u8; 4] = [b'O', b'g', b'g', b'S'];
        let mut cursor = Cursor::new(ogg_magic.to_vec());
        
        let result = is_mkv_stream(&mut cursor);
        assert!(result.is_ok(), "Format detection should not fail");
        assert_eq!(result.unwrap(), false, "Should not detect OGG as MKV");
    }

    /// Test MKV source creation with minimal valid data
    #[test]
    fn test_mkv_source_creation() {
        // Create a minimal MKV stream with EBML header
        let mut mkv_data = vec![0x1A, 0x45, 0xDF, 0xA3]; // EBML header
        
        // Add some minimal MKV structure data
        // This is a very basic MKV structure that should be parseable
        mkv_data.extend_from_slice(&[
            0x18, 0x53, 0x80, 0x67, // Segment ID
            0x80, 0x00, 0x00, 0x00, // Segment size (placeholder)
        ]);
        
        let cursor = Cursor::new(mkv_data);
        
        // Test source creation - this should not panic
        let result = create_mkv_source(cursor);
        
        // Note: This might fail due to incomplete MKV structure, but should not panic
        // The important thing is that our parsing logic doesn't crash
        match result {
            Ok(_) => {
                // If it succeeds, great! Our parser handled it
            }
            Err(_) => {
                // If it fails, that's expected with minimal data, but it shouldn't panic
                // This test mainly ensures our code doesn't crash on invalid data
            }
        }
    }

    /// Test that empty streams are handled gracefully
    #[test]
    fn test_empty_stream_handling() {
        let empty_data = vec![];
        let mut cursor = Cursor::new(empty_data);
        
        let result = is_mkv_stream(&mut cursor);
        assert!(result.is_err(), "Empty stream should result in error");
    }

    /// Test that very short streams are handled gracefully
    #[test]
    fn test_short_stream_handling() {
        let short_data = vec![0x1A, 0x45]; // Too short for EBML header
        let mut cursor = Cursor::new(short_data);
        
        let result = is_mkv_stream(&mut cursor);
        assert!(result.is_err(), "Short stream should result in error");
    }

    /// Test that streams with wrong EBML header are rejected
    #[test]
    fn test_wrong_ebml_header() {
        let wrong_header: [u8; 4] = [0x1B, 0x45, 0xDF, 0xA3]; // Wrong first byte
        let mut cursor = Cursor::new(wrong_header.to_vec());
        
        let result = is_mkv_stream(&mut cursor);
        assert!(result.is_ok(), "Format detection should not fail");
        assert_eq!(result.unwrap(), false, "Should not detect wrong header as MKV");
    }
}

#[cfg(not(feature = "with_mkv"))]
mod tests {
    #[test]
    fn test_mkv_feature_disabled() {
        // When MKV feature is disabled, these tests should be skipped
        // This is just a placeholder to ensure the module compiles
        assert!(true, "MKV feature is disabled, tests skipped");
    }
}