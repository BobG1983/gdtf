//! Sanitize author-facing names into safe file stems.

use bevy::prelude::Deref;

/// Filesystem-safe stem derived from a display name.
#[derive(Clone, Debug, PartialEq, Eq, Deref)]
pub struct FileStem(String);

impl std::fmt::Display for FileStem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Lowercase, trim, map spaces/dashes to `_`, drop non-alphanumeric (except `_`).
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

    #[test]
    fn separators_fold_to_underscores() {
        assert_eq!(sanitize_file_stem("Entry Room").as_str(), "entry_room");
        assert_eq!(sanitize_file_stem("Sump-Waste 2").as_str(), "sump_waste_2");
    }

    #[test]
    fn trims_and_lowercases() {
        assert_eq!(
            sanitize_file_stem("  Industrial Hive  ").as_str(),
            "industrial_hive"
        );
    }

    #[test]
    fn path_hostile_characters_are_dropped() {
        assert_eq!(sanitize_file_stem("../../Evil Gang!").as_str(), "evil_gang");
        assert_eq!(sanitize_file_stem(r"a\b/c").as_str(), "abc");
        assert_eq!(sanitize_file_stem("name.ron").as_str(), "nameron");
    }

    #[test]
    fn empty_and_all_hostile_names_yield_the_empty_stem() {
        assert!(sanitize_file_stem("").is_empty());
        assert!(sanitize_file_stem("   ").is_empty());
        assert!(sanitize_file_stem("!!!///...").is_empty());
    }
}
