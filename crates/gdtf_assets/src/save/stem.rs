//! The [`FileStem`] newtype and [`sanitize_file_stem`] — the ONE slug policy every
//! saver folds a display name through before it becomes an on-disk file name.

use bevy::prelude::Deref;

/// A sanitized file-name STEM — the `[a-z0-9_]`-only slug a saver joins with its own
/// extension and directory layout.
///
/// A named newtype over [`String`] (no-bare-types): a file stem is a domain value with an
/// invariant — it is ALWAYS the output of [`sanitize_file_stem`], never a raw user string.
/// The inner is PRIVATE and there is deliberately NO public constructor (rule 5, tightened):
/// the sanitizer is the only mint, so holding a `FileStem` IS the proof the value is
/// path-safe. Read it through the derived [`Deref`] (`is_empty()` / `as_str()` resolve on
/// the inner [`String`]) or [`Display`](std::fmt::Display) (path `format!` interpolation).
///
/// A stem MAY be empty (a name that sanitizes to nothing) — the POLICY for an empty stem is
/// the caller's (the prefab / terrain / theme savers reject with their `EmptyName` variant;
/// the gang saver substitutes a documented fallback stem).
#[derive(Clone, Debug, PartialEq, Eq, Deref)]
pub struct FileStem(String);

impl std::fmt::Display for FileStem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sanitize a free-form (user-entered) name into a [`FileStem`] — the ONE slug filter
/// (GTW-577 C1) the prefab / terrain / theme / gang savers all delegate to.
///
/// The documented policy, applied in order:
///
/// 1. trim surrounding whitespace;
/// 2. fold to ASCII lowercase;
/// 3. map word separators — space and dash — to `_`;
/// 4. DROP every remaining character outside `[a-z0-9_]` (so `.`, `/`, `\` and any other
///    path-hostile byte can never reach a file name: `"../../Evil"` → `"evil"`).
///
/// A name that sanitizes to NOTHING yields an empty stem — the caller decides whether that
/// is an `EmptyName` rejection or a fallback (see [`FileStem`]).
#[must_use]
pub fn sanitize_file_stem(raw: &str) -> FileStem {
    FileStem(
        raw.trim()
            .to_ascii_lowercase()
            .chars()
            .map(|c| if c == ' ' || c == '-' { '_' } else { c })
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::sanitize_file_stem;

    /// Spaces and dashes are word separators — they fold to `_` (policy step 3).
    #[test]
    fn separators_fold_to_underscores() {
        assert_eq!(sanitize_file_stem("Entry Room").as_str(), "entry_room");
        assert_eq!(sanitize_file_stem("Sump-Waste 2").as_str(), "sump_waste_2");
    }

    /// Surrounding whitespace trims and uppercase folds to lowercase (policy steps 1–2).
    #[test]
    fn trims_and_lowercases() {
        assert_eq!(
            sanitize_file_stem("  Industrial Hive  ").as_str(),
            "industrial_hive"
        );
    }

    /// Everything outside `[a-z0-9_]` is DROPPED (policy step 4) — including the
    /// path-hostile `.` / `/` / `\`, so a traversal-shaped name cannot escape its
    /// directory.
    #[test]
    fn path_hostile_characters_are_dropped() {
        assert_eq!(sanitize_file_stem("../../Evil Gang!").as_str(), "evil_gang");
        assert_eq!(sanitize_file_stem(r"a\b/c").as_str(), "abc");
        assert_eq!(sanitize_file_stem("name.ron").as_str(), "nameron");
    }

    /// A name of nothing but droppable characters sanitizes to the EMPTY stem — the
    /// documented caller-decides policy (`EmptyName` rejection or fallback).
    #[test]
    fn empty_and_all_hostile_names_yield_the_empty_stem() {
        assert!(sanitize_file_stem("").is_empty());
        assert!(sanitize_file_stem("   ").is_empty());
        assert!(sanitize_file_stem("!!!///...").is_empty());
    }
}
