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
pub struct ReadError(std::io::Error);

impl ReadError {
    /// Wrap the underlying I/O error the loader hit reading a RON file's bytes.
    ///
    /// The constructor for the private inner [`std::io::Error`] (no-bare-types
    /// rule 5): the [`RonAssetLoader`](crate::RonAssetLoader) builds a
    /// [`RonLoadError::Read`] through this rather than a `ReadError(err)` tuple
    /// literal, so the wrapped cause is reached only through [`Deref`](core::ops::Deref).
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

/// The underlying RON deserialization error (with source span) from parsing a
/// RON file into the target type.
///
/// A named newtype over [`ron::error::SpannedError`] so the domain error
/// [`RonLoadError::Deserialize`] carries a typed cause rather than a bare
/// foreign type.
#[derive(Debug)]
pub struct RonDeError(ron::error::SpannedError);

impl RonDeError {
    /// Wrap the underlying RON deserialization error (with source span) the loader
    /// hit parsing a RON file into the target type.
    ///
    /// The constructor for the private inner [`ron::error::SpannedError`]
    /// (no-bare-types rule 5): the [`RonAssetLoader`](crate::RonAssetLoader) builds a
    /// [`RonLoadError::Deserialize`] through this rather than a `RonDeError(err)` tuple
    /// literal, so the wrapped cause is reached only through [`Deref`](core::ops::Deref).
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
