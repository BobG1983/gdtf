//! Errors while reading or deserializing RON assets.

use std::fmt;

/// Failure loading a RON asset from disk bytes.
#[derive(Debug)]
pub enum RonLoadError {
    /// Could not read the file bytes.
    Read(ReadError),
    /// Bytes were not valid RON for the expected type.
    Deserialize(RonDeError),
}

/// I/O failure while reading asset bytes.
#[derive(Debug)]
pub struct ReadError(std::io::Error);

impl ReadError {
    /// Wrap an I/O error.
    #[must_use]
    pub const fn new(cause: std::io::Error) -> Self {
        Self(cause)
    }
}

impl core::ops::Deref for ReadError {
    type Target = std::io::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// RON deserialize failure with span info.
#[derive(Debug)]
pub struct RonDeError(ron::error::SpannedError);

impl RonDeError {
    /// Wrap a spanned RON error.
    #[must_use]
    pub const fn new(cause: ron::error::SpannedError) -> Self {
        Self(cause)
    }
}

impl core::ops::Deref for RonDeError {
    type Target = ron::error::SpannedError;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for RonLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(err) => write!(f, "could not read RON asset bytes: {}", err.0),
            Self::Deserialize(err) => write!(f, "could not deserialize RON asset: {}", err.0),
        }
    }
}

impl std::error::Error for RonLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read(err) => Some(&err.0),
            Self::Deserialize(err) => Some(&err.0),
        }
    }
}
