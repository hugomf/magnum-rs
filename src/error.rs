use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpusSourceError {
    #[error("Audio stream is not Opus format")]
    InvalidAudioStream,
    #[error("Invalid container format")]
    InvalidContainerFormat,
    #[error("Invalid header data")]
    InvalidHeaderData,
    #[error("Seek operation failed")]
    SeekError,
    #[cfg(feature = "with_ogg")]
    #[error("{0}")]
    OggHeaderError(#[from] ogg::OggReadError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// OggReadError