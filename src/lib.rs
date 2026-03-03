pub mod container;
pub(crate) mod downmix;
pub mod error;
pub mod metadata;

#[cfg(feature = "with_flac")]
pub use container::flac::FlacSource;

#[cfg(feature = "with_ogg")]
pub use container::ogg::{OpusSourceOgg, FlacSourceOgg, FlacSourceAuto, AudioFormat, detect_format};

#[cfg(feature = "with_rodio")]
pub use rodio; // Re-export rodio so examples can use the same version

#[cfg(feature = "with_kira")]
pub use kira; // Re-export kira for consistency
