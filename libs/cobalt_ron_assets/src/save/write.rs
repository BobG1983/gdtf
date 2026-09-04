//! RON pretty-print save helpers and their errors.
use std::path::Path;

use serde::Serialize;

/// Failure while serializing or writing a RON file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RonSaveError {
    /// serde/ron failed to encode the value.
    Serialize(String),
    /// Filesystem write or directory create failed.
    Write(String),
    /// The workspace root search found no marker, so there is no assets root to write under.
    NoWorkspaceRoot,
}

impl std::fmt::Display for RonSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(err) => write!(f, "failed to serialize the RON payload: {err}"),
            Self::Write(err) => write!(f, "failed to write the RON file: {err}"),
            Self::NoWorkspaceRoot => write!(
                f,
                "found no `Cargo.lock` or `[workspace]` manifest above the crate, so there is no \
                 assets root to write under"
            ),
        }
    }
}

impl std::error::Error for RonSaveError {}

/// Pretty-print `value` as RON text.
///
/// # Errors
///
/// Returns [`RonSaveError::Serialize`] when serde/ron cannot encode `value`.
pub fn serialize_ron_pretty<T: Serialize>(value: &T) -> Result<String, RonSaveError> {
    ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|err| RonSaveError::Serialize(err.to_string()))
}

/// Serialize `value` and write it to `path`. Creates parent directories when needed.
///
/// # Errors
///
/// Returns [`RonSaveError::Serialize`] on encode failure, or [`RonSaveError::Write`] if creating directories or writing the file fails.
pub fn write_ron_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), RonSaveError> {
    let serialized = serialize_ron_pretty(value)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| RonSaveError::Write(err.to_string()))?;
    }
    std::fs::write(path, serialized).map_err(|err| RonSaveError::Write(err.to_string()))
}
