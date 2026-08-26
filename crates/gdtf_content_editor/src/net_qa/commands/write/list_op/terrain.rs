//! The Terrain draft's entry-sides list: one toggle, and no other operation.

use gdtf_battle_sim::terrain::facing::TerrainFacing;
use gdtf_qa_protocol::command::RefusalNote;

use crate::{
    net_qa::{
        commands::write::form_fault::FormWriteFault,
        wire::{EditorListMemberNet, EditorListOpNet, TerrainFacingNet},
    },
    terrain_form::{TerrainDraft, TerrainKindChoice},
};

const NOT_AN_EMPLACEMENT: RefusalNote = RefusalNote::from_static(
    "the Terrain draft commits entry sides only while its kind is Emplacement, so this write \
     would silently do nothing",
);

// The sides the draft holds after `side` is added if absent, removed if present.
fn toggled(draft: &TerrainDraft, side: TerrainFacingNet) -> Vec<TerrainFacing> {
    let wanted = side.to_facing();
    let mut sides = draft.entry_sides().to_vec();
    if let Some(at) = sides.iter().position(|held| *held == wanted) {
        sides.remove(at);
    } else {
        sides.push(wanted);
    }
    sides
}

/// The entry sides the draft holds, as the reply reads them back.
pub(super) fn members(draft: &TerrainDraft) -> Vec<EditorListMemberNet> {
    draft
        .entry_sides()
        .iter()
        .map(|side| EditorListMemberNet::EntrySide(TerrainFacingNet::from_facing(*side)))
        .collect()
}

/// Apply one operation to the entry-sides list.
pub(super) fn apply(draft: &mut TerrainDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    let EditorListOpNet::Toggle(side) = op else {
        return Err(FormWriteFault::bad(
            "the entry-sides list is a row of tick boxes, so Toggle is the only operation it \
             offers"
                .to_owned(),
        ));
    };
    if draft.kind() != TerrainKindChoice::Emplacement {
        return Err(FormWriteFault::Gated(NOT_AN_EMPLACEMENT));
    }
    let sides = toggled(draft, side);
    draft.set_entry_sides(sides);
    Ok(())
}
