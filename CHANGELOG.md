# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Multi-channel audio support (up to 8 channels) for both OGG and CAF containers
- Pre-skip sample handling per RFC 7845 for OGG files
- OGG multi-frame packet handling (TOC byte frame count code)
- Debug trait implementations for OpusSourceOgg and OpusSourceCaf
- Shared `downmix` module with downmix coefficients and helper functions

### Fixed
- Critical: Multi-channel decoder buffer sizing bug (audiopus outputs exactly the channels specified at creation)
- CAF error handling: replaced `.unwrap()` with proper error propagation
- CAF test fixtures: fixed invalid Opus packet data in test helpers

### Changed
- Extracted shared code to `src/downmix.rs` module
- Updated OpusMeta to derive Debug trait

## Test Coverage
- 6 unit tests in `src/downmix.rs`
- 14 integration tests in `tests/kira_test.rs`  
- 4 integration tests in `tests/rodio_test.rs`

Total: 24 tests

---

# Detailed Changes

## Multi-Channel Audio Support

The library now supports Opus files with 1-8 channels. For streams with more than 2 channels:
- The decoder uses Stereo mode (audiopus limitation)
- Output is stereo (2 channels)
- The `output_channels()` method correctly returns 2 for multichannel sources

## Pre-Skip Handling (RFC 7845)

Per RFC 7845 §2.4, Opus streams include "pre-skip" samples that must be discarded at the start of playback. This implementation:
- Reads the `preskip` value from the OpusHead header
- Tracks remaining pre-skip samples in `preskip_remaining`
- Skips these samples in the Iterator implementation

## OGG Multi-Frame Packets

The TOC byte in Opus packets contains frame count information in bits 1-0:
- Code 0: 1 frame
- Code 1: 2 frames
- Code 2/3: Variable size (handled as single frame)

## Code Refactoring

Created `src/downmix.rs` with:
- `DOWNMIX` constant: Channel mixing coefficients for 1-8 channel downmixing
- `downmix_to_stereo()` function: Applies downmix matrix to PCM buffer
- `DecodeBuffer` struct: Shared buffer management utility

## Breaking Changes

None - all changes are backward compatible improvements.
