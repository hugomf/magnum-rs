use audiopus::{coder::Decoder, Channels};
use caf::{CafChunkReader, CafPacketReader};
use std::io::Seek;
use std::{fmt::Debug, io::Read};

use crate::{error::OpusSourceError, metadata::OpusMeta};

pub struct OpusSourceCaf<T>
where
    T: Read + Seek,
{
    pub metadata: OpusMeta,
    packet: CafPacketReader<T>,
    decoder: Decoder,
    buffer: Vec<f32>,
    buffer_pos: usize,
}

impl<T> Debug for OpusSourceCaf<T>
where
    T: Read + Seek,
{
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T> OpusSourceCaf<T>
where
    T: Read + Seek,
{
    pub fn new(file: T) -> Result<Self, OpusSourceError> {
        let cr =
            CafChunkReader::new(file).or_else(|_| Err(OpusSourceError::InvalidContainerFormat))?;
        let packet =
            CafPacketReader::from_chunk_reader(cr, vec![caf::ChunkType::AudioData]).unwrap();

        let metadata = OpusMeta {
            sample_rate: packet.audio_desc.sample_rate as u32,
            channel_count: packet.audio_desc.channels_per_frame as u8,
            preskip: 0,
            output_gain: 0,
        };

        if let caf::FormatType::Other(code) = packet.audio_desc.format_id {
            // Opus inside Caf uses a custom "other" code/id
            if code == 1869641075 {
                let decoder = Decoder::new(
                    audiopus::SampleRate::Hz48000,
                    if packet.audio_desc.channels_per_frame == 1 {
                        Channels::Mono
                    } else {
                        Channels::Stereo
                    },
                )
                .map_err(|_| OpusSourceError::InvalidAudioStream)?;

                Ok(Self {
                    metadata,
                    packet,
                    decoder,
                    buffer: vec![],
                    buffer_pos: 0,
                })
            } else {
                Err(OpusSourceError::InvalidAudioStream)
            }
        } else {
            Err(OpusSourceError::InvalidAudioStream)
        }
    }

    fn get_next_chunk(&mut self) -> Option<Vec<f32>> {
        // Loop to skip corrupted packets and retry
        loop {
            let pkt = match self.packet.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return None, // End of stream
                Err(_) => return None,   // IO error
            };

            let mut output_buf: Vec<f32> = vec![
                0.0;
                (self.packet.audio_desc.frames_per_packet * self.metadata.channel_count as u32)
                    as usize
            ];

            // Decode the Opus packet
            if self.decoder.decode_float(Some(&pkt), &mut output_buf, false).is_ok() {
                return Some(output_buf);
            }
            // Decode failed - continue to next packet
        }
    }
}

impl<T> Iterator for OpusSourceCaf<T>
where
    T: Read + Seek,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // If we have data in the buffer, return the next sample
            if let Some(sample) = self.buffer.get(self.buffer_pos) {
                self.buffer_pos += 1;
                return Some(*sample);
            }
            
            // Buffer exhausted, try to load more data
            self.buffer.clear();
            self.buffer_pos = 0;
            
            match self.get_next_chunk() {
                Some(chunk) => {
                    self.buffer = chunk;
                    // Try to return first sample from new chunk
                    if let Some(sample) = self.buffer.get(self.buffer_pos) {
                        self.buffer_pos += 1;
                        return Some(*sample);
                    }
                    // Empty chunk - continue loop to get next
                }
                None => return None, // End of stream
            }
        }
    }
}

#[cfg(feature = "with_rodio")]
use rodio::source::Source;

#[cfg(feature = "with_rodio")]
impl<T> Source for OpusSourceCaf<T>
where
    T: Read + Seek,
{
    fn current_span_len(&self) -> Option<usize> {
        Some(self.packet.audio_desc.frames_per_packet as usize)
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        std::num::NonZero::new(self.metadata.channel_count as u16).unwrap()
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
impl<T> AudioStream for OpusSourceCaf<T>
where
    T: 'static + Read + Seek + Send + Debug,
{
    fn next(&mut self, _dt: f64) -> kira::Frame {
        match self.metadata.channel_count {
            1 => {
                let l = Iterator::next(self);
                let sl = if let Some(n) = l { n } else { 0.0 };
                kira::Frame {
                    left: sl,
                    right: sl,
                }
            }
            2 => {
                let l = Iterator::next(self);
                let r = Iterator::next(self);
                let sl = if let Some(n) = l { n } else { 0.0 };
                let sr = if let Some(n) = r { n } else { 0.0 };
                kira::Frame {
                    left: sl,
                    right: sr,
                }
            }
            _ => unimplemented!("Only mono and stereo are supported"),
        }
    }
}
