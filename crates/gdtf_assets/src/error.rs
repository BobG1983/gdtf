//! The RON loader's typed error and its underlying-cause newtypes.

use std::fmt;

/// The typed error a [`RonAssetLoader`](crate::RonAssetLoader) can fail with.
///
/// A malformed or unreadable file fails the load with one of these variants —
/// never a panic. Bevy records the failure on the asset's load state (it
/// becomes `Failed`) and surfaces the error in the asset events, so a consumer
/// can react to a bad file rather than crashing on it.
#[derive(Debug)]
pub enum RonLoadError {
    /// The file bytes could not be read from the asset source.
    Read(ReadError),
    /// The bytes were read but did not deserialize into the target type as RON.
    Deserialize(RonDeError),
}

/// The underlying I/O error from reading a RON file's bytes off the asset source.
///
/// A named newtype over [`std::io::Error`] so the domain error
/// [`RonLoadError::Read`] carries a typed cause rather than a bare std type.
#[derive(Debug)]
pub struct ReadError(pub(crate) std::io::Error);

impl core::ops::Deref for ReadError {
    type Target = std::io::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The underlying RON deserialization error (with source span) from parsing a
/// RON file into the target type.
///
/// A named newtype over [`ron::error::SpannedError`] so the domain error
/// [`RonLoadError::Deserialize`] carries a typed cause rather than a bare
/// foreign type.
#[derive(Debug)]
pub struct RonDeError(pub(crate) ron::error::SpannedError);

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
