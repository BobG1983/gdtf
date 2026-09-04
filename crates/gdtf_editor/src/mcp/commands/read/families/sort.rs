//! Every registry is a `HashMap` underneath, so a family is ordered before it goes out.

use crate::mcp::wire::EditorFamilyEntryNet;

/// Order entries by rendered key, so two reads of one registry answer the same list.
pub(super) fn sorted_by_key(mut entries: Vec<EditorFamilyEntryNet>) -> Vec<EditorFamilyEntryNet> {
    entries.sort_by(|left, right| left.key().cmp(right.key()));
    entries
}
