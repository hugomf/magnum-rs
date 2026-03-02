use audiopus::{coder::Decoder, Channels};
use bitreader::BitReader;
use std::io::Seek;
use std::{fmt::Debug, io::Read};

use crate::{error::OpusSourceError, metadata::OpusMeta};

pub struct OpusSourceOgg<T>
where
    T: Read + Seek,
{
    pub metadata: OpusMeta,
    packet: ogg::PacketReader<T>,
    decoder: Decoder,
    buffer: Vec<f32>,
    buffer_pos: usize,
}

impl<T> Debug for OpusSourceOgg<T>
where
    T: Read + Seek,
{
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T> OpusSourceOgg<T>
where
    T: Read + Seek,
{
    pub fn new(file: T) -> Result<Self, OpusSourceError> {
        let mut packet = ogg::PacketReader::new(file);
        let id_header = packet.read_packet_expected()?.data;
        let comment_header = packet.read_packet_expected()?.data;

        let metadata = OpusMeta::with_headers(id_header, comment_header)?;

        let decoder = Decoder::new(
            audiopus::SampleRate::Hz48000,
            if metadata.channel_count == 1 {
                Channels::Mono
            } else {
                Channels::Stereo
            },
        )
        .unwrap();

        Ok(Self {
            metadata,
            packet,
            decoder,
            buffer: vec![],
            buffer_pos: 0,
        })
    }

    /// Read the next Ogg packet.
    /// Return Some(packet), or None if at end of stream or IO error.
    fn get_next_packet(&mut self) -> Option<ogg::Packet> {
        loop {
            match self.packet.read_packet_expected() {
                Ok(packet) => {
                    // Ignore invalid but harmless 0-byte packets that some 
                    // encoders sometimes generate at the end of a stream.
                    if !packet.data.is_empty() {
                        return Some(packet);
                    }
                    // Continue to next packet
                }
                Err(ogg::OggReadError::ReadError(_)) => {
                    // IO error - return None
                    return None;
                }
                Err(ogg::OggReadError::NoCapturePatternFound) => {
                    // End of stream - return None
                    return None;
                }
                Err(_) => {
                    // Other errors (InvalidData, HashMismatch, InvalidStreamStructVer) - return None
                    return None;
                }
            }
        }
    }

    /* FRAME SIZE Reference
    +-----------------------+-----------+-----------+-------------------+
    | Configuration         | Mode      | Bandwidth | Frame Sizes       |
    | Number(s)             |           |           |                   |
    +-----------------------+-----------+-----------+-------------------+
    | 0...3                 | SILK-only | NB        | 10, 20, 40, 60 ms |
    |                       |           |           |                   |
    | 4...7                 | SILK-only | MB        | 10, 20, 40, 60 ms |
    |                       |           |           |                   |
    | 8...11                | SILK-only | WB        | 10, 20, 40, 60 ms |
    |                       |           |           |                   |
    | 12...13               | Hybrid    | SWB       | 10, 20 ms         |
    |                       |           |           |                   |
    | 14...15               | Hybrid    | FB        | 10, 20 ms         |
    |                       |           |           |                   |
    | 16...19               | CELT-only | NB        | 2.5, 5, 10, 20 ms |
    |                       |           |           |                   |
    | 20...23               | CELT-only | WB        | 2.5, 5, 10, 20 ms |
    |                       |           |           |                   |
    | 24...27               | CELT-only | SWB       | 2.5, 5, 10, 20 ms |
    |                       |           |           |                   |
    | 28...31               | CELT-only | FB        | 2.5, 5, 10, 20 ms |
    +-----------------------+-----------+-----------+-------------------+
     */

    fn get_next_chunk(&mut self) -> Option<Vec<f32>> {
        if let Some(packet) = self.get_next_packet() {
            let mut toc = BitReader::new(&packet.data[0..1]);
            let c = toc.read_u8(5).unwrap();
            let s = toc.read_u8(1).unwrap();
            //let f = toc.read_u8(2).unwrap();

            // In milliseconds
            let frame_size = {
                match c {
                    0 | 4 | 8 | 12 | 14 | 18 | 22 | 26 | 30 => 10.0,
                    1 | 5 | 9 | 13 | 15 | 19 | 23 | 27 | 31 => 20.0,
                    2 | 6 | 10 => 40.0,
                    3 | 7 | 11 => 60.0,
                    16 | 20 | 24 | 28 => 2.5,
                    17 | 21 | 25 | 29 => 5.0,
                    _ => panic!("Unsupported frame size"),
                }
            };

            let mut output_buf: Vec<f32> = vec![
                0.0;
                (self.metadata.sample_rate / (1000.0 / frame_size) as u32
                    * if s == 0 { 1 } else { 2 })
                    as usize
            ];

            // Decode the Opus packet (output_buf is filled in-place)
            // Returns Ok(sample_count) or Err
            if self.decoder.decode_float(Some(&packet.data), &mut output_buf, false).is_ok() {
                Some(output_buf)
            } else {
                None // Decode error - return None gracefully
            }
        } else {
            None
        }
    }
}

impl<T> Iterator for OpusSourceOgg<T>
where
    T: Read + Seek,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // If we're out of data (or haven't started) then load a chunk of data into our buffer
        if self.buffer.is_empty() {
            if let Some(chunk) = self.get_next_chunk() {
                self.buffer = chunk;
                self.buffer_pos = 0;
            }
        }
        
        // If we have data in the buffer, return the next sample
        if let Some(sample) = self.buffer.get(self.buffer_pos) {
            self.buffer_pos += 1;
            return Some(*sample);
        }
        
        // Buffer exhausted, try to load more data
        self.buffer.clear();
        self.buffer_pos = 0;
        
        if let Some(chunk) = self.get_next_chunk() {
            self.buffer = chunk;
            if let Some(sample) = self.buffer.get(self.buffer_pos) {
                self.buffer_pos += 1;
                return Some(*sample);
            }
        }
        
        None
    }
}

#[cfg(feature = "with_rodio")]
use rodio::source::Source;

#[cfg(feature = "with_rodio")]
impl<T> Source for OpusSourceOgg<T>
where
    T: Read + Seek,
{
    fn current_span_len(&self) -> Option<usize> {
        Some(240)
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        std::num::NonZero::new(self.metadata.channel_count as u16).unwrap()
    }

    fn sample_rate(&self) -> std::num::NonZero<u32> {
        std::num::NonZero::new(48_000).unwrap()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

#[cfg(feature = "with_kira")]
use kira::audio_stream::AudioStream;

#[cfg(feature = "with_kira")]
impl<T> AudioStream for OpusSourceOgg<T>
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
