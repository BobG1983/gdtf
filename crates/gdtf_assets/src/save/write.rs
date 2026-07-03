//! [`write_ron_pretty`] — the ONE serialize → mkdir → write chain (GTW-577 C2) — and the
//! shared [`RonSaveError`] it (and the pure [`serialize_ron_pretty`] half) fail with.
//!
//! The error lives WITH the write seam (GTW-577 P10), not in a central types file: the
//! per-form save errors (`SavePrefabError` / `SaveTerrainError` / `SaveThemeError`) keep
//! their bespoke domain-validation variants and WRAP this error for the serialize/write
//! tail, so the two tail `Display` arms exist exactly once — here.

use std::path::Path;

use serde::Serialize;

/// Why the shared RON save tail failed — serialize-failed or write-failed (GTW-577 C2).
///
/// The variants carry the rendered failure MESSAGE rather than the typed
/// [`ron::Error`] / [`std::io::Error`] cause: the per-form save errors that wrap this one
/// derive `Clone` + `PartialEq` + `Eq` (their tests compare rejections structurally), and
/// neither underlying error type is `Clone` or `Eq` — the same trade-off the per-form
/// `Serialize(String)` / `Write(String)` variants made before they collapsed onto this
/// seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RonSaveError {
    /// Serializing the payload to pretty RON failed.
    Serialize(String),
    /// Writing the serialized RON to disk failed (directory creation or the file write —
    /// a permissions error / missing volume / io failure).
    Write(String),
}

impl std::fmt::Display for RonSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(err) => write!(f, "failed to serialize the RON payload: {err}"),
            Self::Write(err) => write!(f, "failed to write the RON file: {err}"),
        }
    }
}

impl std::error::Error for RonSaveError {}

/// Serialize `value` to pretty-printed RON text — the pure half of the seam, shared by
/// [`write_ron_pretty`] and by the standalone per-form serializers (the live RON preview +
/// the in-memory round-trip tests use these WITHOUT touching the filesystem, so this half
/// compiles in every profile).
///
/// # Errors
///
/// [`RonSaveError::Serialize`] wrapping the underlying RON serialization error.
pub fn serialize_ron_pretty<T: Serialize>(value: &T) -> Result<String, RonSaveError> {
    ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|err| RonSaveError::Serialize(err.to_string()))
}

/// Serialize `value` to pretty RON and WRITE it to `path`, creating the parent directory
/// if absent — the ONE serialize → `create_dir_all` → `fs::write` chain (GTW-577 C2)
/// every editor-side saver delegates to.
///
/// `#[cfg(debug_assertions)]`-gated (the gang-save module-gate precedent): the fs-write
/// path never compiles into a release binary. The callers are all themselves debug-only
/// save paths; the pure [`serialize_ron_pretty`] half stays ungated for previews / tests.
///
/// # Errors
///
/// [`RonSaveError::Serialize`] from the serialization; [`RonSaveError::Write`] from the
/// directory creation or the file write.
#[cfg(debug_assertions)]
pub fn write_ron_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), RonSaveError> {
    let serialized = serialize_ron_pretty(value)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| RonSaveError::Write(err.to_string()))?;
    }
    std::fs::write(path, serialized).map_err(|err| RonSaveError::Write(err.to_string()))
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{RonSaveError, serialize_ron_pretty, write_ron_pretty};

    /// A tiny payload for the round-trip — fixture data, not a shipped schema.
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    struct Payload {
        name:  String,
        count: u32,
    }

    fn payload() -> Payload {
        Payload {
            name:  "round trip".to_owned(),
            count: 7,
        }
    }

    /// GTW-577 acceptance (3) — the `TempDir` round-trip: `write_ron_pretty` creates the
    /// missing nested directory, writes pretty RON, and the bytes deserialize back to an
    /// equal value.
    #[test]
    fn write_ron_pretty_round_trips_through_a_temp_dir() {
        let Ok(root) = tempfile::tempdir() else {
            unreachable!("a TempDir is creatable on the test host");
        };
        // A NESTED path so the create_dir_all half is exercised, not just fs::write.
        let path = root.path().join("nested").join("dir").join("payload.ron");

        let written = write_ron_pretty(&path, &payload());
        assert_eq!(written, Ok(()), "the write chain succeeds into a fresh dir");

        let Ok(bytes) = std::fs::read_to_string(&path) else {
            unreachable!("the written file is readable back");
        };
        let reloaded = ron::de::from_str::<Payload>(&bytes);
        assert_eq!(
            reloaded.ok(),
            Some(payload()),
            "the pretty RON deserializes back to the written value"
        );
    }

    /// A write into an impossible directory (the parent is a FILE) fails with the typed
    /// [`RonSaveError::Write`] — never a panic.
    #[test]
    fn write_failure_is_the_typed_write_error() {
        let Ok(root) = tempfile::tempdir() else {
            unreachable!("a TempDir is creatable on the test host");
        };
        let blocker = root.path().join("blocker");
        assert!(std::fs::write(&blocker, "occupied").is_ok());

        // `blocker` is a file, so `blocker/child.ron` cannot have its dir created.
        let path = blocker.join("child.ron");
        let written = write_ron_pretty(&path, &payload());
        assert!(
            matches!(written, Err(RonSaveError::Write(_))),
            "a blocked directory surfaces as RonSaveError::Write: {written:?}"
        );
    }

    /// The pure serialize half emits pretty (multi-line) RON — the human-editable shape
    /// the shipped `.ron` content uses.
    #[test]
    fn serialize_half_is_pretty_printed() {
        let Ok(text) = serialize_ron_pretty(&payload()) else {
            unreachable!("a plain struct serializes");
        };
        assert!(
            text.contains('\n'),
            "pretty config emits multi-line RON: {text}"
        );
    }
}
