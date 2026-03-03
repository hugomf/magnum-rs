/// Downmix coefficients for converting N-channel Opus audio to stereo (L, R).
///
/// Based on RFC 7845 Opus channel mapping family 0 and 1.
/// Each entry is `(left_coeffs, right_coeffs)` where the slices are indexed
/// by source channel and the values are linear gain factors that sum to <= 1.0.
///
/// Channel order per RFC 7845 mapping family 0:
///   1ch:  M
///   2ch:  L R
///   3ch:  L C R
///   4ch:  L R BL BR
///   5ch:  L C R BL BR
///   6ch:  L C R BL BR LFE
///   7ch:  L C R BL BR SL SR
///   8ch:  L C R BL BR LFE SL SR
///
/// LFE (low-frequency effects) is excluded from the downmix — it would require
/// a highpass filter to blend correctly into a stereo mix, and most content
/// sounds better without it.
#[allow(dead_code)]
pub(crate) const DOWNMIX: [(&[f32], &[f32]); 8] = [
    // 1ch — mono: both channels get the mono signal
    (&[1.0], &[1.0]),

    // 2ch — stereo: pass through
    (&[1.0, 0.0], &[0.0, 1.0]),

    // 3ch — L C R: center blended at -3dB (0.707) into both sides
    //   L_out = L + 0.707*C,  R_out = R + 0.707*C  (normalized by 1/1.707)
    (&[0.586, 0.414, 0.0], &[0.0, 0.414, 0.586]),

    // 4ch — L R BL BR: rear channels blended at -3dB
    //   L_out = L + 0.707*BL,  R_out = R + 0.707*BR  (normalized by 1/1.707)
    (&[0.586, 0.0, 0.414, 0.0], &[0.0, 0.586, 0.0, 0.414]),

    // 5ch — L C R BL BR
    //   L_out = L + 0.707*C + 0.707*BL  (normalized by 1/2.414)
    //   R_out = R + 0.707*C + 0.707*BR
    (&[0.414, 0.293, 0.0, 0.293, 0.0],
     &[0.0,   0.293, 0.414, 0.0, 0.293]),

    // 6ch — L C R BL BR LFE (index 5 = LFE, excluded)
    (&[0.414, 0.293, 0.0, 0.293, 0.0, 0.0],
     &[0.0,   0.293, 0.414, 0.0, 0.293, 0.0]),

    // 7ch — L C R BL BR SL SR
    //   L_out = L + 0.707*C + 0.707*BL + 0.707*SL  (normalized by 1/3.121)
    //   R_out = R + 0.707*C + 0.707*BR + 0.707*SR
    (&[0.320, 0.226, 0.0,   0.226, 0.0,   0.226, 0.0  ],
     &[0.0,   0.226, 0.320, 0.0,   0.226, 0.0,   0.226]),

    // 8ch — L C R BL BR LFE SL SR (index 5 = LFE, excluded)
    (&[0.320, 0.226, 0.0,   0.226, 0.0,   0.0, 0.226, 0.0  ],
     &[0.0,   0.226, 0.320, 0.0,   0.226, 0.0, 0.0,   0.226]),
];

/// Apply the downmix matrix for `channel_count` channels to a decoded PCM buffer,
/// producing an interleaved stereo output (L, R, L, R, ...).
///
/// `input` is interleaved PCM with `channel_count` channels.
/// Returns a new Vec<f32> with stereo interleaved samples.
#[allow(dead_code)]
pub(crate) fn downmix_to_stereo(input: &[f32], channel_count: u8) -> Vec<f32> {
    debug_assert!(channel_count >= 1 && channel_count <= 8);
    let n = channel_count as usize;

    // Stereo passthrough — no allocation needed, clone directly
    if n == 2 {
        return input.to_vec();
    }

    // Mono — duplicate to both channels
    if n == 1 {
        let mut out = Vec::with_capacity(input.len() * 2);
        for &s in input {
            out.push(s);
            out.push(s);
        }
        return out;
    }

    let (left_coeffs, right_coeffs) = DOWNMIX[n - 1];
    let frames = input.len() / n;
    let mut out = Vec::with_capacity(frames * 2);

    for frame in input.chunks_exact(n) {
        let l: f32 = frame.iter().zip(left_coeffs.iter()).map(|(s, c)| s * c).sum();
        let r: f32 = frame.iter().zip(right_coeffs.iter()).map(|(s, c)| s * c).sum();
        out.push(l);
        out.push(r);
    }

    out
}

/// A decode buffer that handles fetching new chunks when empty.
/// This is shared between OGG and CAF implementations.
#[derive(Debug)]
pub(crate) struct DecodeBuffer {
    pub buffer: Vec<f32>,
    pub pos: usize,
    /// Number of samples to skip at the start (pre-skip handling)
    pub preskip_remaining: u16,
}

impl DecodeBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            pos: 0,
            preskip_remaining: 0,
        }
    }

    /// Get the next sample, fetching a new chunk if the buffer is exhausted.
    /// Returns None when there are no more samples.
    #[allow(dead_code)]
    pub fn next_sample<F>(&mut self, mut fetch: F) -> Option<f32>
    where
        F: FnMut() -> Option<Vec<f32>>,
    {
        loop {
            if let Some(&sample) = self.buffer.get(self.pos) {
                self.pos += 1;
                return Some(sample);
            }

            // Buffer exhausted, try to load more data
            self.buffer.clear();
            self.pos = 0;

            match fetch() {
                Some(chunk) => {
                    self.buffer = chunk;
                }
                None => return None,
            }
        }
    }
}

impl Default for DecodeBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downmix_mono() {
        // Mono: [L] -> [L, L]
        let input: Vec<f32> = vec![1.0, 2.0, 3.0];
        let output = downmix_to_stereo(&input, 1);
        assert_eq!(output.len(), 6); // 3 samples * 2 channels
        assert_eq!(output, vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);
    }

    #[test]
    fn test_downmix_stereo() {
        // Stereo: [L, R] -> [L, R] (passthrough)
        let input: Vec<f32> = vec![1.0, 2.0, 3.0, 4.0];
        let output = downmix_to_stereo(&input, 2);
        assert_eq!(output.len(), 4);
        assert_eq!(output, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_downmix_3ch() {
        // 3-channel (L, C, R): center at -3dB
        let input: Vec<f32> = vec![1.0, 1.0, 1.0, 2.0, 2.0, 2.0]; // 2 frames
        let output = downmix_to_stereo(&input, 3);
        // Should have 2 frames * 2 channels = 4 samples
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_downmix_6ch() {
        // 6-channel (5.1): L, C, R, BL, BR, LFE
        let input: Vec<f32> = vec![1.0; 12]; // 2 frames * 6 channels
        let output = downmix_to_stereo(&input, 6);
        assert_eq!(output.len(), 4); // 2 frames * 2 channels
    }

    #[test]
    fn test_decode_buffer_next_sample() {
        let mut buf = DecodeBuffer::new();
        
        // Buffer is empty, should call fetch
        let result = buf.next_sample(|| Some(vec![1.0, 2.0, 3.0]));
        assert_eq!(result, Some(1.0));
        
        // Should return subsequent samples
        assert_eq!(buf.next_sample(|| panic!("should not fetch")), Some(2.0));
        assert_eq!(buf.next_sample(|| panic!("should not fetch")), Some(3.0));
        
        // Buffer exhausted, should fetch new chunk
        assert_eq!(buf.next_sample(|| Some(vec![10.0, 11.0])), Some(10.0));
    }

    #[test]
    fn test_decode_buffer_empty() {
        let mut buf = DecodeBuffer::new();
        
        // No more data
        let result = buf.next_sample(|| None);
        assert_eq!(result, None);
    }
}
