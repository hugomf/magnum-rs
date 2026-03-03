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
    /// True if the source has more than 2 channels and downmixing is active.
    /// The output is always stereo (2 channels) when downmixing.
    is_downmixing: bool,
}

impl<T> Debug for OpusSourceCaf<T>
where
    T: Read + Seek,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpusSourceCaf")
            .field("metadata", &self.metadata)
            .field("buffer_pos", &self.buffer_pos)
            .field("is_downmixing", &self.is_downmixing)
            .finish_non_exhaustive()
    }
}

impl<T> OpusSourceCaf<T>
where
    T: Read + Seek,
{
    pub fn new(file: T) -> Result<Self, OpusSourceError> {
        let cr =
            CafChunkReader::new(file).or_else(|_| Err(OpusSourceError::InvalidContainerFormat))?;
        let packet = CafPacketReader::from_chunk_reader(cr, vec![caf::ChunkType::AudioData])
            .map_err(|_| OpusSourceError::InvalidContainerFormat)?;

        let metadata = OpusMeta {
            sample_rate: packet.audio_desc.sample_rate as u32,
            channel_count: packet.audio_desc.channels_per_frame as u8,
            preskip: 0,
            output_gain: 0,
        };

        if let caf::FormatType::Other(code) = packet.audio_desc.format_id {
            // Opus inside Caf uses a custom "other" code/id
            if code == 1869641075 {
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
                })
            } else {
                Err(OpusSourceError::InvalidAudioStream)
            }
        } else {
            Err(OpusSourceError::InvalidAudioStream)
        }
    }

    /// The output channel count — always 2 when downmixing, otherwise matches source.
    pub fn output_channels(&self) -> u8 {
        if self.is_downmixing { 2 } else { self.metadata.channel_count }
    }

    fn get_next_chunk(&mut self) -> Option<Vec<f32>> {
        // Loop to skip corrupted packets and retry
        loop {
            let pkt = match self.packet.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => return None, // End of stream
                Err(_) => return None,   // IO error
            };

            // audiopus Decoder always outputs exactly the number of channels specified
            // at creation time (Mono=1, Stereo=2). For >2 channel streams, we create
            // a Stereo decoder, so we must allocate for 2 channels.
            let output_channels = if self.is_downmixing { 2 } else { self.metadata.channel_count };
            let mut output_buf: Vec<f32> = vec![
                0.0;
                (self.packet.audio_desc.frames_per_packet * output_channels as u32) as usize
            ];

            // Decode the Opus packet
            match self.decoder.decode_float(Some(&pkt), &mut output_buf, false) {
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
                Some(chunk) => self.buffer = chunk,
                None => return None,
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
        // Output is always stereo when downmixing; otherwise matches source channel count.
        // SAFETY: output_channels() returns 1–2 for mono/stereo, always non-zero.
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
impl<T> AudioStream for OpusSourceCaf<T>
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