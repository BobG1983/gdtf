//! Reading one file for a line that hands the frame clock back to Bevy.

use std::{fs, path::Path};

/// Bevy's time-update-strategy resource. Assembled with a variant where it is used, so
/// this file never matches its own needle.
pub(crate) const TYPE_NAME: &str = "TimeUpdateStrategy";

const VARIANT: &str = "Automatic";

/// The variant path this guard forbids anywhere in the tracked Rust tree.
pub(crate) fn needle() -> String {
    format!("{TYPE_NAME}::{VARIANT}")
}

/// Every line of `path` writing the needle, as `path:line` with a 1-based line number. A
/// file that is not UTF-8 holds no such line.
pub(crate) fn offending_lines(root: &Path, path: &str) -> Vec<String> {
    let Ok(text) = fs::read_to_string(root.join(path)) else {
        return Vec::new();
    };
    let needle = needle();
    text.lines()
        .enumerate()
        .filter(|(_, line)| line.contains(&needle))
        .map(|(index, _)| format!("{path}:{}", index + 1))
        .collect()
}
