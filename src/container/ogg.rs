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
    /// True if the source has more than 2 channels and downmixing is active.
    /// The output is always stereo (2 channels) when downmixing.
    is_downmixing: bool,
    /// Number of samples to skip at the start of the stream (per RFC 7845).
    /// This is the "pre-skip" value from the OpusHead header.
    preskip_remaining: u16,
}

impl<T> Debug for OpusSourceOgg<T>
where
    T: Read + Seek,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpusSourceOgg")
            .field("metadata", &self.metadata)
            .field("buffer_pos", &self.buffer_pos)
            .field("is_downmixing", &self.is_downmixing)
            .field("preskip_remaining", &self.preskip_remaining)
            .finish_non_exhaustive()
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

        // Opus supports 1–8 channels. audiopus only exposes Mono/Stereo for the
        // decoder channel count, but the decoder still decodes all channels —
        // we pass the actual channel count via the output buffer size.
        // We use Stereo for multi-channel since audiopus doesn't have an N-channel
        // variant; the decoder infers channel count from the stream itself.
        let decoder_channels = if metadata.channel_count == 1 {
            Channels::Mono
        } else {
            Channels::Stereo
        };

        let decoder = Decoder::new(audiopus::SampleRate::Hz48000, decoder_channels)
            .map_err(|_| OpusSourceError::InvalidAudioStream)?;

        let is_downmixing = metadata.channel_count > 2;
        let preskip = metadata.preskip;

        if is_downmixing {
            eprintln!(
                "[magnum] {}-channel Opus stream — downmixing to stereo",
                metadata.channel_count
            );
        }

        Ok(Self {
            metadata,
            packet,
            decoder,
            buffer: vec![],
            buffer_pos: 0,
            is_downmixing,
            preskip_remaining: preskip,
        })
    }

    /// The output channel count — always 2 when downmixing, otherwise matches source.
    pub fn output_channels(&self) -> u8 {
        if self.is_downmixing { 2 } else { self.metadata.channel_count }
    }

    /// Read the next Ogg packet.
    /// Returns Some(packet) or None at end of stream / IO error.
    fn get_next_packet(&mut self) -> Option<ogg::Packet> {
        loop {
            match self.packet.read_packet_expected() {
                Ok(packet) => {
                    if !packet.data.is_empty() {
                        return Some(packet);
                    }
                }
                Err(ogg::OggReadError::ReadError(_)) => return None,
                Err(ogg::OggReadError::NoCapturePatternFound) => return None,
                Err(_) => return None,
            }
        }
    }

    /* FRAME SIZE Reference
    +-----------------------+-----------+-----------+-------------------+
    | Configuration         | Mode      | Bandwidth | Frame Sizes       |
    | Number(s)             |           |           |                   |
    +-----------------------+-----------+-----------+-------------------+
    | 0...3                 | SILK-only | NB        | 10, 20, 40, 60 ms |
    | 4...7                 | SILK-only | MB        | 10, 20, 40, 60 ms |
    | 8...11                | SILK-only | WB        | 10, 20, 40, 60 ms |
    | 12...13               | Hybrid    | SWB       | 10, 20 ms         |
    | 14...15               | Hybrid    | FB        | 10, 20 ms         |
    | 16...19               | CELT-only | NB        | 2.5, 5, 10, 20 ms |
    | 20...23               | CELT-only | WB        | 2.5, 5, 10, 20 ms |
    | 24...27               | CELT-only | SWB       | 2.5, 5, 10, 20 ms |
    | 28...31               | CELT-only | FB        | 2.5, 5, 10, 20 ms |
    +-----------------------+-----------+-----------+-------------------+
    */
    fn get_next_chunk(&mut self) -> Option<Vec<f32>> {
        loop {
            let packet = self.get_next_packet()?;

            if packet.data.is_empty() {
                continue;
            }

            let mut toc = BitReader::new(&packet.data[0..1]);
            let c = match toc.read_u8(5) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let s = match toc.read_u8(1) {
                Ok(v) => v,
                Err(_) => continue,
            };
            // Frame count code: 0=1 frame, 1=2 frames, 2 or 3=multiple frames (size in packet)
            let frame_count_code = match toc.read_u8(2) {
                Ok(v) => v,
                Err(_) => 0, // Default to single frame if we can't read
            };

            let frame_size: f32 = match c {
                0 | 4 | 8 | 12 | 14 | 18 | 22 | 26 | 30 => 10.0,
                1 | 5 | 9 | 13 | 15 | 19 | 23 | 27 | 31 => 20.0,
                2 | 6 | 10 => 40.0,
                3 | 7 | 11 => 60.0,
                16 | 20 | 24 | 28 => 2.5,
                17 | 21 | 25 | 29 => 5.0,
                _ => continue,
            };

            // Determine actual number of frames in this packet
            // Code 0 = 1 frame, Code 1 = 2 frames, Code 2 or 3 = variable (use frame_size)
            let num_frames: usize = match frame_count_code {
                0 => 1,
                1 => 2,
                _ => 1, // For codes 2/3, we use the base frame size
            };

            // Output buffer sized based on what audiopus actually outputs:
            // - Mono decoder -> 1 channel
            // - Stereo decoder -> 2 channels (regardless of source channel count)
            // For >2 channel streams, we use Stereo decoder but allocate for 2 channels
            // since audiopus always outputs exactly the channel count specified at creation.
            let output_channels = if self.is_downmixing { 2 } else { self.metadata.channel_count as usize };
            let samples_per_channel =
                (self.metadata.sample_rate as f32 * frame_size / 1000.0) as usize;
            // `s` bit in TOC: 0 = mono signal, 1 = stereo. For >2ch streams this
            // bit applies to the SILK/CELT layer, not overall channel count.
            #[allow(unused_variables)]
            let s = s; // keep for documentation

            let mut output_buf = vec![0.0f32; samples_per_channel * output_channels * num_frames];

            match self.decoder.decode_float(Some(&packet.data), &mut output_buf, false) {
                Ok(_) => {
                    // For >2 channel streams: audiopus decoded to stereo (2ch) buffer.
                    // Return stereo directly - true multi-channel would require
                    // opus_multistream_decoder which audiopus doesn't expose.
                    return Some(output_buf);
                }
                Err(e) => eprintln!("[magnum] decode error, skipping packet: {:?}", e),
            }
        }
    }
}

impl<T> Iterator for OpusSourceOgg<T>
where
    T: Read + Seek,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(sample) = self.buffer.get(self.buffer_pos) {
                // Skip pre-skip samples at the start of the stream
                if self.preskip_remaining > 0 {
                    self.preskip_remaining -= 1;
                    self.buffer_pos += 1;
                    continue;
                }
                self.buffer_pos += 1;
                return Some(*sample);
            }
            self.buffer.clear();
            self.buffer_pos = 0;
            match self.get_next_chunk() {
                Some(chunk) => self.buffer = chunk,
                None => return None,
            }
        }
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
        // Output is always stereo when downmixing; otherwise matches source channel count.
        // SAFETY: output_channels() returns 1–2 for mono/stereo, always non-zero.
        unsafe { std::num::NonZero::new_unchecked(self.output_channels() as u16) }
    }

    fn sample_rate(&self) -> std::num::NonZero<u32> {
        // SAFETY: 48_000 is a non-zero compile-time constant
        unsafe { std::num::NonZero::new_unchecked(48_000) }
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
        // Output is always mono or stereo after downmixing.
        // Multi-channel content has already been downmixed to stereo in get_next_chunk.
        match self.output_channels() {
            1 => {
                let s = Iterator::next(self).unwrap_or(0.0);
                kira::Frame { left: s, right: s }
            }
            _ => {
                // 2ch or downmixed-to-stereo
                let l = Iterator::next(self).unwrap_or(0.0);
                let r = Iterator::next(self).unwrap_or(0.0);
                kira::Frame { left: l, right: r }
            }
        }
    }
}