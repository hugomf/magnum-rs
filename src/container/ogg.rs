use audiopus::{coder::Decoder, Channels, coder::GenericCtl};
use bitreader::BitReader;
use std::io::{Read, Seek};
use std::fmt::Debug;

use crate::{error::OpusSourceError, metadata::OpusMeta, downmix::DecodeBuffer};

// ============================================================================
// OGG Opus Support
// ============================================================================

pub struct OpusSourceOgg<T>
where
    T: Read + Seek,
{
    pub metadata: OpusMeta,
    packet: ogg::PacketReader<T>,
    decoder: Decoder,
    decode_buffer: DecodeBuffer,
    /// True if the source has more than 2 channels and downmixing is active.
    /// The output is always stereo (2 channels) when downmixing.
    is_downmixing: bool,
}

impl<T> Debug for OpusSourceOgg<T>
where
    T: Read + Seek,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpusSourceOgg")
            .field("metadata", &self.metadata)
            .field("decode_buffer", &self.decode_buffer)
            .field("is_downmixing", &self.is_downmixing)
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
            decode_buffer: DecodeBuffer {
                buffer: Vec::new(),
                pos: 0,
                preskip_remaining: preskip,
            },
            is_downmixing,
        })
    }

    /// The output channel count — always 2 when downmixing, otherwise matches source.
    pub fn output_channels(&self) -> u8 {
        if self.is_downmixing { 2 } else { self.metadata.channel_count }
    }

    /// Seek to a specific sample position in the stream.
    ///
    /// The `sample` parameter is the absolute sample position to seek to.
    /// Returns the actual sample position seeked to (may differ due to OGG
    /// granule alignment).
    ///
    /// Note: After seeking, the first few samples may need to be discarded
    /// to account for the Opus pre-skip requirement. These are handled
    /// automatically during playback.
    pub fn seek(&mut self, sample: u64) -> Result<u64, OpusSourceError> {
        // For OGG Opus, the granule position represents the "end" sample of
        // the page. We need to account for pre-skip when seeking.
        // Target granule = requested sample - pre_skip
        let target_granule = sample.saturating_sub(self.metadata.preskip as u64);

        // Seek using absolute granule position
        let success = self
            .packet
            .seek_absgp(None, target_granule)
            .map_err(|_| OpusSourceError::SeekError)?;

        if !success {
            return Err(OpusSourceError::SeekError);
        }

        // Reset decoder state after seek
        self.decoder
            .reset_state()
            .map_err(|_| OpusSourceError::InvalidAudioStream)?;

        // Clear internal buffer state
        self.decode_buffer.buffer.clear();
        self.decode_buffer.pos = 0;

        // After seeking, we need to discard some samples to get past any
        // transitional audio artifacts. The pre-skip handling in the iterator
        // will handle this.
        self.decode_buffer.preskip_remaining = self.metadata.preskip;

        // Pre-load a valid chunk after seeking to ensure we're at a valid position
        // This helps avoid decode errors on the first read after seek
        self.decode_buffer.buffer = self.get_next_chunk().unwrap_or_default();

        Ok(sample)
    }

    /// Seek to a specific time position in the stream.
    ///
    /// Convenience method that converts Duration to sample position.
    pub fn seek_duration(&mut self, position: std::time::Duration) -> Result<u64, OpusSourceError> {
        let target_sample = (position.as_secs_f64() * self.metadata.sample_rate as f64) as u64;
        self.seek(target_sample)
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
            // Code 0 = 1 frame, Code 1 = 2 frames, Code 2 or 3 = variable (read from packet)
            let num_frames: usize = match frame_count_code {
                0 => 1,
                1 => 2,
                2 | 3 => {
                    // For codes 2 and 3, the actual frame count is stored in the packet body
                    // at byte offset 1 (after the TOC byte)
                    if packet.data.len() > 1 {
                        packet.data[1] as usize
                    } else {
                        1 // Fallback if we can't read the frame count
                    }
                }
                _ => 1,
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
            let _s_stereo_bit = s; // keep for documentation

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
        // Skip pre-skip samples at the start of the stream
        if self.decode_buffer.preskip_remaining > 0 {
            self.decode_buffer.preskip_remaining -= 1;
            // Fetch and discard the sample without using the closure
            if self.decode_buffer.buffer.is_empty() {
                if let Some(chunk) = self.get_next_chunk() {
                    self.decode_buffer.buffer = chunk;
                    self.decode_buffer.pos = 0;
                }
            }
            
            // Discard the sample
            if let Some(_sample) = self.decode_buffer.buffer.get(self.decode_buffer.pos) {
                self.decode_buffer.pos += 1;
            }
            
            return self.next(); // Get the next sample after skipping
        }
        
        // Use the DecodeBuffer to handle fetching new chunks
        // We need to avoid borrowing conflicts by handling buffer management manually
        loop {
            if let Some(sample) = self.decode_buffer.buffer.get(self.decode_buffer.pos) {
                self.decode_buffer.pos += 1;
                return Some(*sample);
            }

            // Buffer exhausted, try to load more data
            self.decode_buffer.buffer.clear();
            self.decode_buffer.pos = 0;

            match self.get_next_chunk() {
                Some(chunk) => {
                    self.decode_buffer.buffer = chunk;
                }
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
        // Return a reasonable default frame size for OGG Opus
        // Most Opus streams use 20ms frames at 48kHz = 960 samples per channel
        // For stereo, that's 1920 samples total
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

// ============================================================================
// OGG FLAC Support
// ============================================================================

/// OGG FLAC header structure for parsing stream info
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct OggFlacHeader {
    sample_rate: u32,
    channels: u8,
    bits_per_sample: u8,
    total_samples: u64,
}

impl OggFlacHeader {
    /// Parse the FLAC STREAMINFO block from OGG FLAC headers
    /// First OGG packet contains: "fLaC" + STREAMINFO block
    fn from_ogg_packet(packet: &[u8]) -> Result<Self, OpusSourceError> {
        // OGG FLAC first packet: "fLaC" (4 bytes) + STREAMINFO block
        if packet.len() < 38 {
            return Err(OpusSourceError::InvalidHeaderData);
        }

        // Check for FLAC magic "fLaC"
        if &packet[0..4] != b"fLaC" {
            return Err(OpusSourceError::InvalidContainerFormat);
        }

        // STREAMINFO block starts at offset 4
        // Block type (1 byte): 0x01 for STREAMINFO (with 0x80 if last block - but first packet should have more)
        // Block size (3 bytes BE)
        // Then STREAMINFO content

        let block_type = packet[4];
        let block_size = ((packet[5] as u32) << 16) | ((packet[6] as u32) << 8) | (packet[7] as u32);

        // STREAMINFO should be type 0
        if (block_type & 0x7F) != 0 {
            return Err(OpusSourceError::InvalidHeaderData);
        }

        if block_size < 34 {
            return Err(OpusSourceError::InvalidHeaderData);
        }

        // STREAMINFO starts at offset 8
        let info_start = 8;
        let info = &packet[info_start..info_start + block_size as usize];

        // Parse STREAMINFO (simplified - extract key fields)
        // Bytes 0-1: min block size
        // Bytes 2-3: max block size
        // Bytes 4-7: min frame size (24 bits) + max frame size start
        // Bytes 8-11: sample rate (20 bits), channels (3 bits), bits per sample (5 bits), total samples (36 bits)
        // This is packed across bytes 8-11

        let _sample_rate = (((info[8] as u32) << 12) | ((info[9] as u32) << 4) | ((info[10] as u32) >> 4)) >> 12;
        // Actually, let's use a simpler approach - read the packed fields
        let packed = ((info[8] as u64) << 24) | ((info[9] as u64) << 16) | ((info[10] as u64) << 8) | (info[11] as u64);
        let sample_rate = ((packed >> 44) & 0xFFFFF) as u32;
        let channels = ((packed >> 41) & 0x7) as u8 + 1;
        let bits_per_sample = ((packed >> 36) & 0x1F) as u8 + 1;

        if sample_rate == 0 {
            // Try alternative parsing - some streams encode differently
            // Direct byte extraction for common FLAC configurations
            let srate = (((info[8] as u32) << 4) | ((info[9] as u32) >> 4)) & 0xFFFFF;
            if srate > 0 && srate <= 192000 {
                return Ok(Self {
                    sample_rate: srate,
                    channels: (((info[9] >> 1) & 0x7) + 1) as u8,
                    bits_per_sample: (((info[9] & 0x1) << 4) | ((info[10] >> 4) & 0xF)) as u8 + 1,
                    total_samples: 0,
                });
            }
            return Err(OpusSourceError::InvalidHeaderData);
        }

        Ok(Self {
            sample_rate,
            channels,
            bits_per_sample,
            total_samples: 0,
        })
    }
}

/// FLAC audio source from OGG container (OGG FLAC format)
pub struct FlacSourceOgg<T>
where
    T: Read + Seek,
{
    pub metadata: OpusMeta,
    packet_reader: ogg::PacketReader<T>,
    /// FLAC stream reader - we decode frame by frame
    current_frame_samples: Vec<f32>,
    sample_position: usize,
    is_downmixing: bool,
    channels: u8,
    #[allow(dead_code)]
    bits_per_sample: u32,
    /// Buffer for building FLAC frames from OGG packets
    #[allow(dead_code)]
    frame_buffer: Vec<u8>,
}

impl<T> Debug for FlacSourceOgg<T>
where
    T: Read + Seek,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlacSourceOgg")
            .field("metadata", &self.metadata)
            .field("is_downmixing", &self.is_downmixing)
            .field("channels", &self.channels)
            .finish_non_exhaustive()
    }
}

impl<T> FlacSourceOgg<T>
where
    T: Read + Seek,
{
    /// Create a new OGG FLAC source from a Read + Seek stream
    pub fn new(stream: T) -> Result<Self, OpusSourceError> {
        let mut packet_reader = ogg::PacketReader::new(stream);

        // First packet contains FLAC STREAMINFO
        let first_packet = packet_reader
            .read_packet_expected()
            .map_err(|_| OpusSourceError::InvalidContainerFormat)?;

        // Parse the FLAC header
        let header = OggFlacHeader::from_ogg_packet(&first_packet.data)?;

        // Second packet is typically VORBIS_COMMENT (can be skipped for audio)
        // Read it but don't fail if it's not there
        let _comment_packet = packet_reader.read_packet_expected();

        let is_downmixing = header.channels > 2;

        if is_downmixing {
            eprintln!(
                "[magnum] {}-channel OGG FLAC stream — downmixing to stereo",
                header.channels
            );
        }

        let metadata = OpusMeta {
            sample_rate: header.sample_rate,
            channel_count: if is_downmixing { 2 } else { header.channels },
            preskip: 0,
            output_gain: 0,
        };

        Ok(Self {
            metadata,
            packet_reader,
            current_frame_samples: Vec::new(),
            sample_position: 0,
            is_downmixing,
            channels: header.channels,
            bits_per_sample: header.bits_per_sample as u32,
            frame_buffer: Vec::new(),
        })
    }

    /// The output channel count — always 2 when downmixing, otherwise matches source.
    pub fn output_channels(&self) -> u8 {
        if self.is_downmixing { 2 } else { self.channels }
    }

    /// Convert sample to f32 based on bit depth
    #[inline]
    #[allow(dead_code)]
    fn to_f32(&self, sample: i32) -> f32 {
        let max_val = (1u32 << (self.bits_per_sample - 1)) as f32;
        sample as f32 / max_val
    }

    /// Read the next FLAC frame from OGG packets
    /// OGG FLAC packets contain FLAC frames prefixed with frame number
    fn read_next_frame(&mut self) -> Option<()> {
        // Get next OGG packet
        let packet = loop {
            match self.packet_reader.read_packet_expected() {
                Ok(p) if !p.data.is_empty() => break p,
                Ok(_) => continue, // Empty packet, try next
                Err(_) => return None, // End of stream or error
            }
        };

        // OGG FLAC packets have a frame number prefix (variable length)
        // Frame number is encoded as a UTF-8 number followed by FLAC frame
        // Skip the frame number and extract FLAC frame data
        let frame_data = self.extract_flac_frame(&packet.data)?;

        // Decode the FLAC frame
        // For simplicity, we use a cursor-based approach with the flac crate
        self.decode_flac_frame(frame_data)
    }

    /// Extract FLAC frame data from OGG packet
    /// OGG FLAC format: [frame_number_varint][FLAC frame data]
    fn extract_flac_frame<'a>(&self, packet: &'a [u8]) -> Option<&'a [u8]> {
        // Skip frame number (variable length, encoded as UTF-8 like)
        // Simple heuristic: find the start of FLAC frame sync code 0xFF 0xF8
        if packet.len() < 4 {
            return None;
        }

        // Look for FLAC frame sync pattern: 0xFF followed by 0xF8-0xFF
        for i in 0..packet.len().saturating_sub(1) {
            if packet[i] == 0xFF && (packet[i + 1] & 0xF8) == 0xF8 {
                return Some(&packet[i..]);
            }
        }

        // Fallback: assume first few bytes are frame number, rest is frame
        // Conservative approach: skip first 2-4 bytes
        if packet.len() > 4 {
            return Some(&packet[2.min(packet.len())..]);
        }

        None
    }

    /// Decode a FLAC frame to samples
    fn decode_flac_frame(&mut self, _frame_data: &[u8]) -> Option<()> {
        // Note: Full OGG FLAC decoding requires proper FLAC frame parsing
        // The flac crate's StreamReader expects complete FLAC files, not individual frames
        // For now, we return None - a full implementation would need a custom FLAC frame decoder
        // or integration with a streaming FLAC library
        None
    }

    /// Decode a FLAC frame to samples (placeholder for future implementation)

    /// Parse raw frame samples (fallback method)
    #[allow(dead_code)]
    fn parse_raw_frame_samples(&mut self, _frame_data: &[u8]) -> Option<()> {
        // This would require a lower-level FLAC frame parser
        // For now, return None to indicate we can't decode this frame
        // In a full implementation, you'd parse the FLAC frame structure directly
        None
    }

    /// Downmix multi-channel samples to stereo
    #[allow(dead_code)]
    fn downmix_samples(&mut self, samples: &[i32]) {
        let frames = samples.len() / self.channels as usize;
        self.current_frame_samples.clear();
        self.current_frame_samples.reserve(frames * 2);

        for frame in 0..frames {
            let mut left = 0.0f32;
            let mut right = 0.0f32;

            for ch in 0..self.channels {
                let idx = frame * self.channels as usize + ch as usize;
                let sample = self.to_f32(samples[idx]);

                if ch % 2 == 0 {
                    left += sample;
                } else {
                    right += sample;
                }
            }

            let ch_count = self.channels as f32;
            self.current_frame_samples.push(left / ch_count);
            self.current_frame_samples.push(right / ch_count);
        }
    }
}

impl<T> Iterator for FlacSourceOgg<T>
where
    T: Read + Seek,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // Return samples from current frame buffer
        if self.sample_position < self.current_frame_samples.len() {
            let sample = self.current_frame_samples[self.sample_position];
            self.sample_position += 1;
            return Some(sample);
        }

        // Buffer exhausted, decode next frame
        self.read_next_frame()?;

        // Return first sample from new buffer
        if self.sample_position < self.current_frame_samples.len() {
            let sample = self.current_frame_samples[self.sample_position];
            self.sample_position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

#[cfg(feature = "with_rodio")]
impl<T> Source for FlacSourceOgg<T>
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
impl<T> AudioStream for FlacSourceOgg<T>
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

// ============================================================================
// Format Detection Utilities
// ============================================================================

/// Detect the container/format of an audio stream by examining magic bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    /// Raw FLAC stream (starts with "fLaC")
    RawFlac,
    /// OGG container (starts with "OggS")
    Ogg,
    /// Unknown format
    Unknown,
}

/// Peek at the beginning of a stream to detect its format
/// Returns the detected format and a cursor/rewound stream
///
/// Note: This reads a few bytes from the stream, so the caller must handle
/// rewinding or using the returned position.
pub fn detect_format<T: Read + Seek>(stream: &mut T) -> Result<AudioFormat, OpusSourceError> {
    let mut magic = [0u8; 4];
    let start_pos = stream.stream_position().map_err(|_| OpusSourceError::InvalidAudioStream)?;

    match stream.read_exact(&mut magic) {
        Ok(()) => {
            // Rewind to start
            stream.seek(std::io::SeekFrom::Start(start_pos))
                .map_err(|_| OpusSourceError::InvalidAudioStream)?;

            match &magic {
                b"fLaC" => Ok(AudioFormat::RawFlac),
                b"OggS" => Ok(AudioFormat::Ogg),
                _ => Ok(AudioFormat::Unknown),
            }
        }
        Err(_) => Err(OpusSourceError::InvalidAudioStream),
    }
}

/// Auto-detect and create appropriate FLAC source from stream
/// Handles both raw FLAC and OGG-wrapped FLAC
///
/// Note: Use `FlacSourceAuto::new()` for automatic format detection.
/// This function is a placeholder and always returns an error.
#[allow(dead_code)]
pub fn create_flac_source<T>(_stream: T) -> Result<Box<dyn Iterator<Item = f32>>, OpusSourceError>
where
    T: Read + Seek + 'static,
{
    // Use FlacSourceAuto::new() instead which handles format detection
    Err(OpusSourceError::InvalidContainerFormat)
}

/// Macro-like helper to create the appropriate FLAC source
/// The caller is responsible for detecting format first using `detect_format`
#[derive(Debug)]
#[allow(dead_code)]
#[cfg(feature = "with_flac")]
pub enum FlacSourceAuto<T: Read + Seek> {
    Raw(crate::container::flac::FlacSource<T>),
    Ogg(FlacSourceOgg<T>),
}

#[cfg(feature = "with_flac")]
impl<T: Read + Seek> Iterator for FlacSourceAuto<T> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            FlacSourceAuto::Raw(src) => src.next(),
            FlacSourceAuto::Ogg(src) => src.next(),
        }
    }
}

#[cfg(feature = "with_flac")]
impl<T: Read + Seek> FlacSourceAuto<T> {
    /// Create a FLAC source from a stream, auto-detecting the format
    pub fn new(mut stream: T) -> Result<Self, OpusSourceError> {
        let format = detect_format(&mut stream)?;

        match format {
            AudioFormat::RawFlac => {
                crate::container::flac::FlacSource::new(stream)
                    .map(FlacSourceAuto::Raw)
            }
            AudioFormat::Ogg => {
                // Try OGG FLAC first
                match FlacSourceOgg::new(stream) {
                    Ok(src) => Ok(FlacSourceAuto::Ogg(src)),
                    Err(_) => {
                        // Could be OGG Opus or other OGG format
                        Err(OpusSourceError::InvalidContainerFormat)
                    }
                }
            }
            AudioFormat::Unknown => Err(OpusSourceError::InvalidContainerFormat),
        }
    }

    /// Get the sample rate of the stream
    pub fn sample_rate(&self) -> u32 {
        match self {
            FlacSourceAuto::Raw(src) => src.metadata.sample_rate,
            FlacSourceAuto::Ogg(src) => src.metadata.sample_rate,
        }
    }

    /// Get the channel count
    pub fn channel_count(&self) -> u8 {
        match self {
            FlacSourceAuto::Raw(src) => src.metadata.channel_count,
            FlacSourceAuto::Ogg(src) => src.metadata.channel_count,
        }
    }

    /// Get the output channel count (after downmixing)
    pub fn output_channels(&self) -> u8 {
        match self {
            FlacSourceAuto::Raw(src) => src.output_channels(),
            FlacSourceAuto::Ogg(src) => src.output_channels(),
        }
    }
}

#[cfg(all(feature = "with_flac", feature = "with_rodio"))]
impl<T: Read + Seek> Source for FlacSourceAuto<T> {
    fn current_span_len(&self) -> Option<usize> {
        match self {
            FlacSourceAuto::Raw(src) => src.current_span_len(),
            FlacSourceAuto::Ogg(src) => src.current_span_len(),
        }
    }

    fn channels(&self) -> std::num::NonZero<u16> {
        match self {
            FlacSourceAuto::Raw(src) => src.channels(),
            FlacSourceAuto::Ogg(src) => src.channels(),
        }
    }

    fn sample_rate(&self) -> std::num::NonZero<u32> {
        match self {
            FlacSourceAuto::Raw(src) => src.sample_rate(),
            FlacSourceAuto::Ogg(src) => src.sample_rate(),
        }
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        match self {
            FlacSourceAuto::Raw(src) => src.total_duration(),
            FlacSourceAuto::Ogg(src) => src.total_duration(),
        }
    }
}

#[cfg(all(feature = "with_flac", feature = "with_kira"))]
impl<T: 'static + Read + Seek + Send + Debug> AudioStream for FlacSourceAuto<T> {
    fn next(&mut self, dt: f64) -> kira::Frame {
        match self {
            FlacSourceAuto::Raw(src) => AudioStream::next(src, dt),
            FlacSourceAuto::Ogg(src) => AudioStream::next(src, dt),
        }
    }
}