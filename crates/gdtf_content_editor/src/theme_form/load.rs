//! Where a theme def was read from, for the draft a load fills.

use gdtf_assets::{ContentMemberKey, ContentSourcePath, ContentSourcePaths};
use gdtf_battle_sim::level::ThemeUuid;
use gdtf_content_families::ThemeDefsFamily;

/// The file a theme def was read from, as the family recorded it under the def's key.
///
/// The theme family keys its source paths by the def's hyphenated UUID text, so both
/// load sites ask for one the same way.
#[must_use]
pub fn theme_source(
    sources: Option<&ContentSourcePaths<ThemeDefsFamily>>,
    key: ThemeUuid,
) -> Option<ContentSourcePath> {
    sources?
        .path(&ContentMemberKey::new((*key).to_string()))
        .cloned()
}
