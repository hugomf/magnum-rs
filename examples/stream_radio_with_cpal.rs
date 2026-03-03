//! Example demonstrating how to play an OGG Opus internet radio stream through speakers using cpal
//!
//! Run with: cargo run --example stream_radio_with_cpal --features "with_ogg"
//!
//! This example demonstrates:
//! - Streaming audio from a network URL
//! - Playing live OGG Opus streams (like Icecast) through audio output
//! - Using a custom Read + Seek wrapper for network streams
//! - Low-level audio playback using cpal (cross-platform audio library)

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::traits::{Split, Consumer, Producer};

/// A wrapper around a network stream that implements Read + Seek
/// Note: Seek is limited for live streams - only forward seeking is supported
#[derive(Debug)]
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
    println!("Magnum Opus Radio Stream Player (cpal)");
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
    
    // Set up cpal audio output
    let host = cpal::default_host();
    let device = host.default_output_device()
        .expect("No audio output device available");
    
    println!("\nAudio device: {}", device.name().unwrap_or_default());
    
    // Get the default output config
    let config = device.default_output_config()?;
    println!("Default output config: {:?}", config);
    
    // Use the stream's sample rate and channels
    let sample_rate = cpal::SampleRate(source.metadata.sample_rate);
    let channels = source.output_channels() as u16;
    
    let output_config = cpal::StreamConfig {
        channels: channels as cpal::ChannelCount,
        sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };
    
    println!("Using output config: {:?}", output_config);
    
    // Create a ring buffer for audio samples
    let ring_capacity = 48000 * 2 * 2; // 2 seconds of stereo f32
    let ring = ringbuf::HeapRb::<f32>::new(ring_capacity);
    let (mut producer, mut consumer) = ring.split();
    
    // Wrap the source in a mutex for thread-safe access
    let source = Arc::new(Mutex::new(source));
    let source_clone = source.clone();
    
    // Spawn a thread to continuously decode and fill the ring buffer
    std::thread::spawn(move || {
        loop {
            let mut src = source_clone.lock().unwrap();
            match src.next() {
                Some(sample) => {
                    drop(src);
                    while producer.try_push(sample).is_err() {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
                None => break,
            }
        }
    });
    
    // Fill the buffer initially
    println!("\nBuffering audio...");
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Build the audio stream
    let err_fn = |err| eprintln!("Audio stream error: {}", err);
    
    let audio_stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &output_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                for sample in data.iter_mut() {
                    *sample = consumer.try_pop().unwrap_or(0.0);
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_output_stream(
            &output_config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                for sample in data.iter_mut() {
                    let s = consumer.try_pop().unwrap_or(0.0);
                    *sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                }
            },
            err_fn,
            None,
        )?,
        cpal::SampleFormat::U16 => device.build_output_stream(
            &output_config,
            move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                for sample in data.iter_mut() {
                    let s = consumer.try_pop().unwrap_or(0.0);
                    *sample = ((s.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16;
                }
            },
            err_fn,
            None,
        )?,
        _ => panic!("Unsupported sample format"),
    };
    
    // Start playing
    println!("\n🎵 Playing jazz stream... Press Ctrl+C to stop\n");
    audio_stream.play()?;
    
    // Keep the main thread alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("Playing...");
    }
}
