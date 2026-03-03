#[cfg(feature = "with_mkv")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if a file path was provided
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <mkv_file_path>", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    println!("Decoding MKV file: {}", file_path);

    // Open the file
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    // Check if it's an MKV file
    match magnum::is_mkv_stream(&mut reader) {
        Ok(true) => println!("✓ Valid MKV file detected"),
        Ok(false) => {
            eprintln!("✗ Not a valid MKV file");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("✗ Error checking file format: {}", e);
            std::process::exit(1);
        }
    }

    // Create MKV source
    match magnum::create_mkv_source(reader) {
        Ok(source) => {
            println!("✓ MKV source created successfully");
            println!("  Sample rate: {} Hz", source.metadata.sample_rate);
            println!("  Channels: {}", source.metadata.channel_count);
            println!("  Output channels: {}", source.output_channels());
            
            // Note: The current implementation is a placeholder
            // In a complete implementation, you would iterate through samples:
            // for sample in source.take(1000) {
            //     // Process audio samples
            // }
            
            println!("✓ MKV container structure parsed successfully");
        }
        Err(e) => {
            eprintln!("✗ Failed to create MKV source: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(not(feature = "with_mkv"))]
fn main() {
    eprintln!("Error: This example requires the 'with_mkv' feature to be enabled.");
    eprintln!("Run with: cargo run --example decode_mkv --features with_mkv");
    std::process::exit(1);
}
