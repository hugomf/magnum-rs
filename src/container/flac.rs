use std::io::{Read, Seek};
use std::fmt::Debug;

use crate::{error::OpusSourceError, metadata::OpusMeta};

/// A FLAC audio source that can decode FLAC files from any Read + Seek source.
/// 
/// This implementation uses the flac crate for decoding and supports both
/// mono and multi-channel audio with automatic downmixing to stereo when needed.
pub struct FlacSource<T>
where
    T: Read + Seek,
{
    pub metadata: OpusMeta,
    /// FLAC stream reader
    stream: flac::StreamReader<T>,
    /// Current samples being decoded
    samples: Vec<f32>,
    /// Current position in samples buffer
    position: usize,
    /// True if the source has more than 2 channels and downmixing is active.
    pub is_downmixing: bool,
    /// Number of channels in the FLAC stream
    channels: u8,
    /// Bits per sample
    bits_per_sample: u32,
}

impl<T> Debug for FlacSource<T>
where
    T: Read + Seek,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlacSource")
            .field("metadata", &self.metadata)
            .field("is_downmixing", &self.is_downmixing)
            .field("channels", &self.channels)
            .finish_non_exhaustive()
    }
}

impl<T> FlacSource<T>
where
    T: Read + Seek,
{
    pub fn new(stream: T) -> Result<Self, OpusSourceError> {
        // Create FLAC stream reader
        let stream = flac::StreamReader::<T>::new(stream)
            .map_err(|_| OpusSourceError::InvalidAudioStream)?;
        
        // Get stream info
        let info = stream.info();
        let sample_rate = info.sample_rate;
        let channels = info.channels as u8;
        let bits_per_sample = info.bits_per_sample as u32;
        
        let is_downmixing = channels > 2;
        
        if is_downmixing {
            eprintln!(
                "[magnum] {}-channel FLAC stream — downmixing to stereo",
                channels
            );
        }
        
        // Create metadata struct
        let metadata = OpusMeta {
            sample_rate,
            channel_count: if is_downmixing { 2 } else { channels },
            preskip: 0,
            output_gain: 0,
        };

        Ok(Self {
            metadata,
            stream,
            samples: Vec::new(),
            position: 0,
            is_downmixing,
            channels,
            bits_per_sample,
        })
    }

    /// The output channel count — always 2 when downmixing, otherwise matches source.
    pub fn output_channels(&self) -> u8 {
        if self.is_downmixing { 2 } else { self.channels }
    }
    
    /// Convert sample to f32
    #[inline]
    fn to_f32(&self, sample: i32) -> f32 {
        let max_val = (1u32 << (self.bits_per_sample - 1)) as f32;
        sample as f32 / max_val
    }
    
    /// Decode more samples from the stream
    fn decode_more(&mut self) -> Option<()> {
        // Use a more controlled approach to prevent hanging
        let mut decoded = Vec::new();
        let mut frame_count = 0;
        let max_frames = 10; // Limit frames per call to prevent hanging
        
        // Try to decode a limited number of frames
        let mut iter = self.stream.iter::<i16>();
        
        while frame_count < max_frames {
            match iter.next() {
                Some(sample) => {
                    decoded.push(sample as i32);
                    // Limit the number of samples to prevent memory issues
                    if decoded.len() >= 4096 {
                        break;
                    }
                }
                None => {
                    // End of stream or error
                    break;
                }
            }
            frame_count += 1;
        }
        
        if decoded.is_empty() {
            return None;
        }
        
        if self.is_downmixing {
            // Downmix to stereo
            let frames = decoded.len() / self.channels as usize;
            let mut new_samples = Vec::with_capacity(frames * 2);
            
            for frame in 0..frames {
                let mut left = 0.0f32;
                let mut right = 0.0f32;
                
                for ch in 0..self.channels {
                    let idx = frame * self.channels as usize + ch as usize;
                    if idx < decoded.len() {
                        let sample = self.to_f32(decoded[idx]);
                        
                        if ch % 2 == 0 {
                            left += sample;
                        } else {
                            right += sample;
                        }
                    }
                }
                
                // Average
                let ch_count = self.channels as f32;
                new_samples.push(left / ch_count);
                new_samples.push(right / ch_count);
            }
            
            self.samples = new_samples;
        } else {
            // No downmixing
            self.samples = decoded.iter().map(|&s| self.to_f32(s)).collect();
        }
        
        self.position = 0;
        
        if self.samples.is_empty() {
            None
        } else {
            Some(())
        }
    }
}

impl<T> Iterator for FlacSource<T>
where
    T: Read + Seek,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Return samples from buffer
        if self.position < self.samples.len() {
            let sample = self.samples[self.position];
            self.position += 1;
            return Some(sample);
        }
        
        // Buffer empty, try to decode more
        self.decode_more()?;
        
        // Return first sample from new buffer
        if !self.samples.is_empty() {
            let sample = self.samples[0];
            self.position = 1;
            Some(sample)
        } else {
            None
        }
    }
}

#[cfg(feature = "with_rodio")]
use rodio::source::Source;

#[cfg(feature = "with_rodio")]
impl<T> Source for FlacSource<T>
where
    T: Read + Seek,
{
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        unsafe { std::num::NonZero::new_unchecked(self.output_channels() as u16) }
    }

    fn sample_rate(&self) -> std::num::NonZero<u32> {
        std::num::NonZero::new(self.metadata.sample_rate).unwrap()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

#[cfg(feature = "with_kira")]
use kira::audio_stream::AudioStream;

#[cfg(feature = "with_kira")]
impl<T> AudioStream for FlacSource<T>
where
    T: 'static + Read + Seek + Send + Debug,
{
    fn next(&mut self, _dt: f64) -> kira::Frame {
        match self.output_channels() {
            1 => {
                let s = Iterator::next(self).unwrap_or(0.0);
                kira::Frame { left: s, right: s }
            }
            _ => {
                let l = Iterator::next(self).unwrap_or(0.0);
                let r = Iterator::next(self).unwrap_or(0.0);
                kira::Frame { left: l, right: r }
            }
        }
    }
}
