//! The error lives WITH the write chain (GTW-577 P10), not in a central types file: the
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RonSaveError {
        Serialize(String),
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

pub fn serialize_ron_pretty<T: Serialize>(value: &T) -> Result<String, RonSaveError> {
    ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
        .map_err(|err| RonSaveError::Serialize(err.to_string()))
}

/// `#[cfg(debug_assertions)]`-gated (the gang-save module-gate precedent): the fs-write
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

                #[test]
    fn write_ron_pretty_round_trips_through_a_temp_dir() {
        let Ok(root) = tempfile::tempdir() else {
            unreachable!("a TempDir is creatable on the test host");
        };
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

            #[test]
    fn write_failure_is_the_typed_write_error() {
        let Ok(root) = tempfile::tempdir() else {
            unreachable!("a TempDir is creatable on the test host");
        };
        let blocker = root.path().join("blocker");
        assert!(std::fs::write(&blocker, "occupied").is_ok());

        let path = blocker.join("child.ron");
        let written = write_ron_pretty(&path, &payload());
        assert!(
            matches!(written, Err(RonSaveError::Write(_))),
            "a blocked directory surfaces as RonSaveError::Write: {written:?}"
        );
    }

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
