use gdtf_assets::sanitize_file_stem;

#[must_use]
pub(crate) fn theme_dir(display_name: &str) -> String {
    let slug = sanitize_file_stem(display_name);
    if slug.is_empty() {
        "unknown_theme".to_owned()
    } else {
        slug.to_string()
    }
}
