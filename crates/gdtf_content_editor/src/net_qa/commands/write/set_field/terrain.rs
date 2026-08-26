//! The Terrain form's own field arm, written through the draft its kind segment writes.

use crate::{
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorFieldNet, TerrainKindNet},
    },
    terrain_form::TerrainDraft,
};

/// Write one Terrain field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut TerrainDraft,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::Kind(kind) => {
            draft.set_kind(kind.to_choice());
            Ok(EditorFieldNet::Kind(TerrainKindNet::from_choice(
                draft.kind(),
            )))
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}
