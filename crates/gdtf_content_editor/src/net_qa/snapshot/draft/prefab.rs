//! The PREFAB mode's field snapshot (GTW-805).

use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::fields::{count_field, debug_field};
use crate::{CurrentEditLevel, EditorMap};

/// The PREFAB mode's fields.
///
/// PREFAB is the one mode whose "draft" is not a form record but the painted map itself
/// ([`EditorMap`]) plus the storey the author is painting on, so its snapshot reports those:
/// how many cells are painted, and which storey the canvas is showing. The drawable extent
/// and the active paint tile are the SESSION's, and are reported by that topic rather than
/// duplicated here.
pub(super) fn prefab_fields(
    map: &EditorMap,
    level: Option<&CurrentEditLevel>,
) -> Vec<EditorDraftFieldView> {
    vec![
        count_field("painted_cells", map.painted_count()),
        debug_field("current_level", &level),
    ]
}
