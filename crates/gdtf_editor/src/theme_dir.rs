//! Theme display name to folder slug.

use cobalt_ron_assets::sanitize_file_stem;

/// Folder name under terrain/prefab trees for a theme display name.
#[must_use]
pub(crate) fn theme_dir(display_name: &str) -> String {
    let slug = sanitize_file_stem(display_name);
    if slug.is_empty() {
        "unknown_theme".to_owned()
    } else {
        slug.to_string()
    }
}
