use crate::error::OpusSourceError;
use byteorder::{ByteOrder, LittleEndian};

#[derive(Debug)]
pub struct OpusMeta {
    pub sample_rate: u32,
    pub channel_count: u8,
    pub preskip: u16,
    pub output_gain: i16,
}

impl OpusMeta {
    pub fn with_headers(
        id_header: Vec<u8>,
        comment_header: Vec<u8>,
    ) -> Result<Self, OpusSourceError> {
        // Validate id_header length
        if id_header.len() < 19 {
            return Err(OpusSourceError::InvalidHeaderData);
        }

        // Check magic bytes for id header
        if id_header[0..8] != *b"OpusHead" {
            return Err(OpusSourceError::InvalidHeaderData);
        }

        let _version = id_header[8];
        let channels = id_header[9];
        let preskip = LittleEndian::read_u16(&id_header[10..12]);
        let _pre_enc_sample_rate = LittleEndian::read_u32(&id_header[12..16]);
        let output_gain = LittleEndian::read_i16(&id_header[16..18]);
        let _channel_mapping_family = id_header[18];

        // Validate comment_header length
        if comment_header.len() < 8 {
            return Err(OpusSourceError::InvalidHeaderData);
        }

        // Check magic bytes for comment header
        if comment_header[0..8] != *b"OpusTags" {
            return Err(OpusSourceError::InvalidHeaderData);
        }

        // Parse vendor string length safely
        if comment_header.len() < 12 {
            return Err(OpusSourceError::InvalidHeaderData);
        }
        
        let vs_len = LittleEndian::read_u32(&comment_header[8..12]);
        
        // Validate vendor string bounds
        if comment_header.len() < (12 + vs_len as usize) {
            return Err(OpusSourceError::InvalidHeaderData);
        }
        
        let vstring = &comment_header[12..12 + vs_len as usize];
        let _vstring = String::from_utf8(vstring.to_vec())
            .map_err(|_| OpusSourceError::InvalidHeaderData)?;
        
        let nts = 12 + vs_len as usize;
        if comment_header.len() < (nts + 4) {
            return Err(OpusSourceError::InvalidHeaderData);
        }
        
        let _num_tags = LittleEndian::read_u32(&comment_header[nts..nts + 4]);

        Ok(Self {
            sample_rate: 48_000,
            channel_count: channels,
            preskip,
            output_gain,
        })
    }
}
