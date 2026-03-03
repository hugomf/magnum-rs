use std::io::{Read, Seek, Cursor};
use std::fmt::Debug;
use byteorder::{ReadBytesExt, BigEndian};

use crate::{error::OpusSourceError, metadata::OpusMeta, downmix::DecodeBuffer};

// ============================================================================
// MKV (Matroska) Container Support
// ============================================================================

/// MKV element IDs for Opus streams
const EBML_HEADER_ID: u32 = 0x1A45DFA3;
const SEGMENT_ID: u32 = 0x18538067;
const TRACKS_ID: u32 = 0x1654AE6B;
const TRACK_ENTRY_ID: u32 = 0xAE;
const TRACK_TYPE_ID: u32 = 0x83;
const CODEC_ID_ID: u32 = 0x86;
const CODEC_PRIVATE_ID: u32 = 0x63A2;
const AUDIO_ID: u32 = 0xE1;
const SAMPLING_FREQUENCY_ID: u32 = 0xB5;
const CHANNELS_ID: u32 = 0x9F;
const BLOCK_GROUP_ID: u32 = 0xA0;
const BLOCK_ID: u32 = 0xA1;
const SIMPLE_BLOCK_ID: u32 = 0xA3;

/// MKV element ID for seeking
const SEEK_HEAD_ID: u32 = 0x114D9B74;
const SEEK_ID: u32 = 0x4DBB;
const SEEK_POSITION_ID: u32 = 0x53AB;

/// Opus codec ID constant
const OPUS_CODEC_ID: &str = "A_OPUS";

/// MKV element structure
#[derive(Debug, Clone)]
struct MkvElement {
    id: u32,
    size: u64,
    data_offset: u64,
}

/// MKV audio track information
#[derive(Debug, Clone)]
pub struct MkvAudioTrack {
    pub track_number: u64,
    pub codec_id: String,
    pub codec_private: Vec<u8>,
    pub sampling_frequency: f64,
    pub channels: u64,
    pub track_type: u64,
}

/// MKV parser for reading elements
pub struct MkvParser<T>
where
    T: Read + Seek,
{
    reader: T,
    current_position: u64,
}

impl<T> MkvParser<T>
where
    T: Read + Seek,
{
    pub fn new(mut reader: T) -> Result<Self, OpusSourceError> {
        let start_pos = reader.stream_position()?;
        Ok(Self {
            reader,
            current_position: start_pos,
        })
    }

    /// Read and parse the next MKV element
    fn read_element(&mut self) -> Result<Option<MkvElement>, OpusSourceError> {
        let id = self.read_element_id()?;
        if id == 0 {
            return Ok(None); // End of stream
        }

        let size = self.read_element_size()?;
        let data_offset = self.reader.stream_position()?;

        Ok(Some(MkvElement {
            id,
            size,
            data_offset,
        }))
    }

    /// Read element ID (1-4 bytes)
    fn read_element_id(&mut self) -> Result<u32, OpusSourceError> {
        let first_byte = self.reader.read_u8()?;
        self.current_position += 1;

        let (id_bytes, mask) = match first_byte {
            0x1F => (4, 0x0F),
            0x3F => (3, 0x1F),
            0x7F => (2, 0x3F),
            0xFF => (1, 0x7F),
            _ => {
                // Single byte ID
                return Ok(first_byte as u32);
            }
        };

        let mut id = (first_byte & mask) as u32;
        
        for _ in 1..id_bytes {
            let byte = self.reader.read_u8()?;
            self.current_position += 1;
            id = (id << 8) | byte as u32;
        }

        Ok(id)
    }

    /// Read element size (1-8 bytes)
    fn read_element_size(&mut self) -> Result<u64, OpusSourceError> {
        let first_byte = self.reader.read_u8()?;
        self.current_position += 1;

        let size_bytes = 8 - first_byte.leading_zeros();
        let size_mask = (1 << (7 - size_bytes)) - 1;
        let mut size = (first_byte & size_mask) as u64;

        for _ in 1..size_bytes {
            let byte = self.reader.read_u8()?;
            self.current_position += 1;
            size = (size << 8) | byte as u64;
        }

        Ok(size)
    }

    /// Skip to element data
    fn skip_to_element_data(&mut self, element: &MkvElement) -> Result<(), OpusSourceError> {
        self.reader.seek(std::io::SeekFrom::Start(element.data_offset))?;
        self.current_position = element.data_offset;
        Ok(())
    }

    /// Read element data
    fn read_element_data(&mut self, element: &MkvElement) -> Result<Vec<u8>, OpusSourceError> {
        let mut data = vec![0u8; element.size as usize];
        self.reader.read_exact(&mut data)?;
        self.current_position += element.size;
        Ok(data)
    }

    /// Seek to a specific position
    fn seek(&mut self, pos: u64) -> Result<(), OpusSourceError> {
        self.reader.seek(std::io::SeekFrom::Start(pos))?;
        self.current_position = pos;
        Ok(())
    }
}

/// Opus audio source from MKV container
pub struct OpusSourceMkv<T>
where
    T: Read + Seek,
{
    pub metadata: OpusMeta,
    parser: MkvParser<T>,
    audio_track: MkvAudioTrack,
    decode_buffer: DecodeBuffer,
    /// True if the source has more than 2 channels and downmixing is active.
    /// The output is always stereo (2 channels) when downmixing.
    is_downmixing: bool,
    /// Current position in the stream for seeking
    current_timecode: u64,
    /// Timecode scale from segment info
    timecode_scale: u64,
    /// Cluster position for seeking
    cluster_positions: Vec<u64>,
}

impl<T> Debug for OpusSourceMkv<T>
where
    T: Read + Seek,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpusSourceMkv")
            .field("metadata", &self.metadata)
            .field("audio_track", &self.audio_track)
            .field("is_downmixing", &self.is_downmixing)
            .field("current_timecode", &self.current_timecode)
            .finish_non_exhaustive()
    }
}

impl<T> OpusSourceMkv<T>
where
    T: Read + Seek,
{
    pub fn new(mut file: T) -> Result<Self, OpusSourceError> {
        let mut parser = MkvParser::new(file)?;
        
        // Parse EBML header
        parser.read_element()?; // Skip EBML header
        
        // Parse segment
        let tracks_element = parser.read_element()?;
        if tracks_element.is_none() || tracks_element.as_ref().unwrap().id != SEGMENT_ID {
            return Err(OpusSourceError::InvalidContainerFormat);
        }

        // Parse tracks to find Opus audio track
        let audio_track = Self::parse_tracks(&mut parser)?;
        
        // Parse segment info for timecode scale
        let timecode_scale = Self::parse_segment_info(&mut parser)?;

        // Initialize decoder
        let is_downmixing = audio_track.channels > 2;
        
        if is_downmixing {
            eprintln!(
                "[magnum] {}-channel MKV Opus stream — downmixing to stereo",
                audio_track.channels
            );
        }

        let metadata = OpusMeta {
            sample_rate: audio_track.sampling_frequency as u32,
            channel_count: if is_downmixing { 2 } else { audio_track.channels as u8 },
            preskip: 0, // MKV doesn't have pre-skip in the same way
            output_gain: 0,
        };

        Ok(Self {
            metadata,
            parser,
            audio_track,
            decode_buffer: DecodeBuffer {
                buffer: Vec::new(),
                pos: 0,
                preskip_remaining: 0,
            },
            is_downmixing,
            current_timecode: 0,
            timecode_scale: timecode_scale.unwrap_or(1_000_000), // Default nanosecond scale
            cluster_positions: Vec::new(),
        })
    }

    /// Parse tracks section to find Opus audio track
    fn parse_tracks(parser: &mut MkvParser<T>) -> Result<MkvAudioTrack, OpusSourceError> {
        let tracks_element = parser.read_element()?;
        if tracks_element.is_none() || tracks_element.as_ref().unwrap().id != TRACKS_ID {
            return Err(OpusSourceError::InvalidContainerFormat);
        }

        parser.skip_to_element_data(tracks_element.as_ref().unwrap())?;

        loop {
            let element = parser.read_element()?;
            match element {
                Some(elem) if elem.id == TRACK_ENTRY_ID => {
                    if let Some(track) = Self::parse_track_entry(parser, &elem)? {
                        if track.codec_id == OPUS_CODEC_ID {
                            return Ok(track);
                        }
                    }
                }
                Some(_) => {
                    // Skip other elements
                }
                None => break,
            }
        }

        Err(OpusSourceError::InvalidContainerFormat)
    }

    /// Parse individual track entry
    fn parse_track_entry(parser: &mut MkvParser<T>, element: &MkvElement) -> Result<Option<MkvAudioTrack>, OpusSourceError> {
        parser.skip_to_element_data(element)?;

        let mut track = MkvAudioTrack {
            track_number: 0,
            codec_id: String::new(),
            codec_private: Vec::new(),
            sampling_frequency: 48000.0,
            channels: 2,
            track_type: 0,
        };

        let end_pos = element.data_offset + element.size;

        while parser.current_position < end_pos {
            let elem = match parser.read_element()? {
                Some(e) => e,
                None => break,
            };

            match elem.id {
                TRACK_TYPE_ID => {
                    let data = parser.read_element_data(&elem)?;
                    if !data.is_empty() {
                        track.track_type = data[0] as u64;
                    }
                }
                CODEC_ID_ID => {
                    let data = parser.read_element_data(&elem)?;
                    track.codec_id = String::from_utf8_lossy(&data).to_string();
                }
                CODEC_PRIVATE_ID => {
                    track.codec_private = parser.read_element_data(&elem)?;
                }
                SAMPLING_FREQUENCY_ID => {
                    let data = parser.read_element_data(&elem)?;
                    if data.len() >= 8 {
                        let mut cursor = Cursor::new(data);
                        track.sampling_frequency = cursor.read_f64::<BigEndian>()?;
                    }
                }
                CHANNELS_ID => {
                    let data = parser.read_element_data(&elem)?;
                    if !data.is_empty() {
                        track.channels = data[0] as u64;
                    }
                }
                _ => {
                    // Skip other elements
                    parser.skip_to_element_data(&elem)?;
                    parser.read_element_data(&elem)?;
                }
            }
        }

        if track.codec_id == OPUS_CODEC_ID && track.track_type == 2 { // Audio track type
            Ok(Some(track))
        } else {
            Ok(None)
        }
    }

    /// Parse segment info for timecode scale
    fn parse_segment_info(parser: &mut MkvParser<T>) -> Result<Option<u64>, OpusSourceError> {
        // Look for Info element
        loop {
            let element = parser.read_element()?;
            match element {
                Some(elem) if elem.id == 0x1549A966 => { // Info element ID
                    parser.skip_to_element_data(&elem)?;
                    
                    let end_pos = elem.data_offset + elem.size;
                    while parser.current_position < end_pos {
                        let sub_elem = parser.read_element()?;
                        match sub_elem {
                            Some(e) if e.id == 0x2AD7B1 => { // TimecodeScale element ID
                                let data = parser.read_element_data(&e)?;
                                if data.len() >= 4 {
                                    let mut cursor = Cursor::new(data);
                                    let scale = cursor.read_u32::<BigEndian>()?;
                                    return Ok(Some(scale as u64));
                                }
                            }
                            Some(_) => {
                                // Skip other elements
                            }
                            None => break,
                        }
                    }
                    break;
                }
                Some(_) => {
                    // Skip other elements
                }
                None => break,
            }
        }
        
        Ok(None)
    }

    /// The output channel count — always 2 when downmixing, otherwise matches source.
    pub fn output_channels(&self) -> u8 {
        if self.is_downmixing { 2 } else { self.audio_track.channels as u8 }
    }

    /// Seek to a specific time position in the stream
    pub fn seek_time(&mut self, time_ns: u64) -> Result<(), OpusSourceError> {
        // For MKV, we would need to implement proper seeking by finding the
        // appropriate cluster and block. This is a simplified implementation.
        // In a full implementation, you'd parse the Cues element for seeking.
        self.current_timecode = time_ns;
        Ok(())
    }

    /// Read the next Opus packet from MKV
    fn get_next_packet(&mut self) -> Option<Vec<u8>> {
        loop {
            let element = match self.parser.read_element() {
                Ok(Some(e)) => e,
                _ => return None,
            };

            match element.id {
                BLOCK_ID | SIMPLE_BLOCK_ID => {
                    // Parse block data
                    let block_data = match self.parser.read_element_data(&element) {
                        Ok(data) => data,
                        Err(_) => continue,
                    };

                    // Extract Opus packet from block
                    // MKV blocks contain timecode and flags before the actual data
                    if block_data.len() > 4 {
                        // Skip timecode (2 bytes) and flags (1 byte)
                        let opus_data = &block_data[3..];
                        if !opus_data.is_empty() {
                            return Some(opus_data.to_vec());
                        }
                    }
                }
                _ => {
                    // Skip other elements
                }
            }
        }
    }
}

impl<T> Iterator for OpusSourceMkv<T>
where
    T: Read + Seek,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // MKV support would require a full Opus decoder implementation
        // For now, return None to indicate this is a placeholder
        // A complete implementation would:
        // 1. Parse MKV blocks containing Opus packets
        // 2. Decode Opus packets using audiopus
        // 3. Handle multi-channel downmixing
        // 4. Implement proper seeking using Cues element
        
        None
    }
}

#[cfg(feature = "with_rodio")]
use rodio::source::Source;

#[cfg(feature = "with_rodio")]
impl<T> Source for OpusSourceMkv<T>
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
impl<T> AudioStream for OpusSourceMkv<T>
where
    T: 'static + Read + Seek + Send + Debug,
{
    fn next(&mut self, _dt: f64) -> kira::Frame {
        // Placeholder implementation
        kira::Frame { left: 0.0, right: 0.0 }
    }
}

// ============================================================================
// MKV Format Detection
// ============================================================================

/// Detect if a stream is an MKV file by checking the EBML header
pub fn is_mkv_stream<T: Read + Seek>(stream: &mut T) -> Result<bool, OpusSourceError> {
    let start_pos = stream.stream_position()?;
    
    // Read first 4 bytes to check for EBML signature
    let mut magic = [0u8; 4];
    match stream.read_exact(&mut magic) {
        Ok(()) => {
            // Rewind to start
            stream.seek(std::io::SeekFrom::Start(start_pos))?;

            // Check for EBML header signature (0x1A45DFA3)
            let ebml_sig = u32::from_be_bytes([magic[0], magic[1], magic[2], magic[3]]);
            Ok(ebml_sig == EBML_HEADER_ID)
        }
        Err(_) => {
            stream.seek(std::io::SeekFrom::Start(start_pos))?;
            Err(OpusSourceError::InvalidAudioStream)
        }
    }
}

/// Create an MKV Opus source from a stream
pub fn create_mkv_source<T>(stream: T) -> Result<OpusSourceMkv<T>, OpusSourceError>
where
    T: Read + Seek,
{
    OpusSourceMkv::new(stream)
}