use std::io::Cursor;
use magnum::container::flac::FlacSource;

#[test]
fn test_flac_source_error_handling() {
    // Test error handling with completely invalid data
    let mock_data = vec![0u8; 1024]; // Completely invalid FLAC
    let cursor = Cursor::new(mock_data);
    
    // With completely invalid data, we expect an error
    let result = FlacSource::new(cursor);
    assert!(result.is_err(), "Expected error for invalid FLAC data");
}

#[test]
fn test_flac_source_with_insufficient_data() {
    // Test with data that's too short to be valid FLAC
    let mock_data = vec![0u8; 10];
    let cursor = Cursor::new(mock_data);
    
    let result = FlacSource::new(cursor);
    assert!(result.is_err(), "Expected error for insufficient FLAC data");
}

#[test]
fn test_flac_source_with_wrong_magic() {
    // Test with wrong magic bytes
    let mut mock_data = vec![0u8; 100];
    mock_data[0..4].copy_from_slice(b"FLAC"); // Wrong magic - should be "fLaC"
    let cursor = Cursor::new(mock_data);
    
    let result = FlacSource::new(cursor);
    assert!(result.is_err(), "Expected error for wrong magic bytes");
}
