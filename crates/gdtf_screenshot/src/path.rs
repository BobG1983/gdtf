//! Output path helpers for screenshot captures.

use std::path::PathBuf;

use bevy::prelude::*;

/// Destination path for a screenshot PNG.
#[derive(Resource, Clone, Debug, PartialEq, Eq, Deref)]
pub struct CapturePath(PathBuf);

impl CapturePath {
    /// Wrap an owned path.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

/// Parse an optional path string into a [`CapturePath`].
/// Returns `None` when the value is missing, empty, or only whitespace.
#[must_use]
pub fn parse_shot_path(value: Option<&str>) -> Option<CapturePath> {
    value
        .map(|raw| raw.trim().to_owned())
        .filter(|trimmed| !trimmed.is_empty())
        .map(|trimmed| CapturePath::new(PathBuf::from(trimmed)))
}

#[cfg(test)]
mod tests {
    use super::parse_shot_path;

    #[test]
    fn path_gate_accepts_non_empty_and_rejects_blank() {
        assert!(parse_shot_path(Some("/abs/out.png")).is_some());
        assert!(parse_shot_path(Some("  /trim/out.png  ")).is_some());
        assert!(parse_shot_path(Some("")).is_none());
        assert!(parse_shot_path(Some("   ")).is_none());
        assert!(parse_shot_path(None).is_none());
    }

    #[test]
    fn path_gate_trims_surrounding_whitespace() {
        let parsed = parse_shot_path(Some("  /trim/out.png  "));
        assert_eq!(
            parsed.map(|path| path.to_string_lossy().into_owned()),
            Some("/trim/out.png".to_owned())
        );
    }
}
