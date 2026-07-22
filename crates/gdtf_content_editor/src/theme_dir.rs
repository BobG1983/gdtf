//! The ONE per-theme DIRECTORY naming policy (GTW-634 C2) — shared by the
//! terrain-def saver (`assets/content/terrain/<theme>/`) and the prefab saver
//! (`assets/content/maps/<theme>/<size>/`), which carried identical
//! copies before GTW-634.
//!
//! This is deliberately NOT merged with the theme form's `slugify`: that
//! sibling's empty-slug behaviour diverges ON PURPOSE (an empty slug is a
//! `SaveThemeError::EmptyName` at the caller — the theme's own name MUST name
//! its directory), while here an empty slug falls back to the still-unique
//! `unknown_theme` bucket so a save never targets the folder root. Two
//! explicit policies, never a mode-flagged one (the GTW-577 P9 bound).

use gdtf_assets::sanitize_file_stem;

/// The `snake_case` directory name for a theme, derived from its human
/// display name (`"Industrial Hive"` → `industrial_hive`) — the slug filter
/// is the shared [`sanitize_file_stem`] helper (GTW-577 C1). NOT a closed-enum
/// match (the UUID model has no closed theme enum). A name that slugifies to
/// NOTHING falls back to the (still-unique) `unknown_theme` bucket, so a save
/// never targets the containing folder's root.
#[must_use]
pub(crate) fn theme_dir(display_name: &str) -> String {
    let slug = sanitize_file_stem(display_name);
    if slug.is_empty() {
        "unknown_theme".to_owned()
    } else {
        slug.to_string()
    }
}
