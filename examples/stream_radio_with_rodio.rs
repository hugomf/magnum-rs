//! Example demonstrating how to play an OGG Opus internet radio stream through speakers using Rodio
//!
//! Run with: cargo run --example stream_radio --features "with_ogg,with_rodio"
//!
//! This example demonstrates:
//! - Streaming audio from a network URL
//! - Playing live OGG Opus streams (like Icecast) through audio output
//! - Using a custom Read + Seek wrapper for network streams
//! - Audio playback using Rodio 0.22

use std::io::{self, Read, Seek, SeekFrom};

/// A wrapper around a network stream that implements Read + Seek
/// Note: Seek is limited for live streams - only forward seeking is supported
struct NetworkStream {
    response: reqwest::blocking::Response,
    position: u64,
    buffer: Vec<u8>,
    buffer_pos: usize,
}

impl NetworkStream {
    fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        println!("Connecting to stream: {}", url);
        let response = reqwest::blocking::get(url)?;
        
        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }
        
        println!("Connected successfully!");
        println!("Content-Type: {:?}", response.headers().get("content-type"));
        
        Ok(Self {
            response,
            position: 0,
            buffer: Vec::with_capacity(65536),
            buffer_pos: 0,
        })
    }
    
    fn fill_buffer(&mut self) -> io::Result<usize> {
        self.buffer.clear();
        self.buffer_pos = 0;
        
        let mut temp_buf = vec![0u8; 65536];
        match self.response.read(&mut temp_buf) {
            Ok(n) if n > 0 => {
                self.buffer.extend_from_slice(&temp_buf[..n]);
                Ok(n)
            }
            Ok(_) => Ok(0),
            Err(e) => Err(e),
        }
    }
}

impl Read for NetworkStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.buffer_pos >= self.buffer.len() {
            let bytes_read = self.fill_buffer()?;
            if bytes_read == 0 {
                return Ok(0);
            }
        }
        
        let available = self.buffer.len() - self.buffer_pos;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + to_read]);
        self.buffer_pos += to_read;
        self.position += to_read as u64;
        
        Ok(to_read)
    }
}

impl Seek for NetworkStream {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(n) => {
                if n < self.position {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Cannot seek backward in live stream"
                    ));
                }
                
                let to_skip = n - self.position;
                let mut discard = vec![0u8; 8192];
                let mut remaining = to_skip;
                
                while remaining > 0 {
                    let chunk = remaining.min(discard.len() as u64) as usize;
                    let read = self.read(&mut discard[..chunk])?;
                    if read == 0 {
                        break;
                    }
                    remaining -= read as u64;
                }
                
                Ok(self.position)
            }
            SeekFrom::End(_) => {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Cannot seek from end in live stream"
                ))
            }
            SeekFrom::Current(n) => {
                if n < 0 {
                    Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Cannot seek backward in live stream"
                    ))
                } else {
                    self.seek(SeekFrom::Start(self.position + n as u64))
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Magnum Opus Radio Stream Player (Rodio)");
    println!("========================================\n");
    
    let stream_url = "https://icecast.walmradio.com:8443/jazz_opus";
    
    // Connect to the stream
    let stream = NetworkStream::new(stream_url)?;
    
    // Create the Opus decoder
    println!("Initializing Opus decoder...");
    let source = magnum::container::ogg::OpusSourceOgg::new(stream)?;
    
    println!("\nStream info:");
    println!("  Channels: {}", source.metadata.channel_count);
    println!("  Sample Rate: {} Hz", source.metadata.sample_rate);
    println!("  Output Channels: {}", source.output_channels());
    
    // Set up Rodio audio output using the new 0.22 API
    // Use magnum's re-export of rodio to ensure the same Source trait
    println!("\nOpening audio device...");
    let device_sink = magnum::rodio::DeviceSinkBuilder::open_default_sink()
        .expect("Failed to open default audio device");
    
    // Add the source to the mixer
    // The Source trait is implemented when with_rodio feature is enabled
    println!("🎵 Playing jazz stream... Press Ctrl+C to stop\n");
    device_sink.mixer().add(source);
    
    // Keep the main thread alive while audio plays
    // For a live stream, we loop indefinitely
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("Playing...");
    }
}
