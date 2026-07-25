//! The draft-field row builders shared by every mode's snapshot (GTW-805).

use core::fmt::Debug;

use gdtf_qa_protocol::view::{
    EditorDraftFieldNameNet, EditorDraftFieldValueNet, EditorDraftFieldView,
};

/// One field row from a name and an already-rendered value.
pub(super) fn text_field(name: &str, value: impl Into<String>) -> EditorDraftFieldView {
    EditorDraftFieldView::new(
        EditorDraftFieldNameNet::new(name.to_owned()),
        EditorDraftFieldValueNet::new(value.into()),
    )
}

/// One field row whose value is rendered through the domain value's own [`Debug`] form.
///
/// The ten authoring forms hold ten unrelated record shapes over the sim's own newtypes;
/// rendering each through the type it already derives keeps the readout deterministic
/// without mirroring the whole content schema into per-field wire types.
pub(super) fn debug_field<T: Debug + ?Sized>(name: &str, value: &T) -> EditorDraftFieldView {
    text_field(name, format!("{value:?}"))
}

/// One field row reporting how many entries a list-shaped draft field holds.
pub(super) fn count_field(name: &str, count: usize) -> EditorDraftFieldView {
    text_field(name, count.to_string())
}
