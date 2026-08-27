//! The Terrain draft's two tick-box lists: one toggle each, and no other operation.

use gdtf_battle_sim::terrain::facing::TerrainFacing;

use crate::{
    net_qa::{
        commands::write::form_fault::{FormWriteFault, NOT_AN_EMPLACEMENT},
        wire::{
            EditorListMemberNet, EditorListNet, EditorListOpNet, TerrainFacingNet, TerrainTagNet,
        },
    },
    terrain_form::{TerrainDraft, TerrainKindChoice},
};

const TOGGLE_ONLY: &str = "the Terrain form draws this list as a row of tick boxes, so Toggle is the only operation \
     it offers";

// The member the toggle must carry for the list it names.
fn wrong_member(list: EditorListNet) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "that member does not belong to {list:?}, so the toggle names no tick box the form draws"
    ))
}

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

/// The members of the named list, as the reply reads them back.
pub(super) fn members(draft: &TerrainDraft, list: EditorListNet) -> Vec<EditorListMemberNet> {
    match list {
        EditorListNet::TerrainTags => draft
            .tags()
            .iter()
            .map(|tag| EditorListMemberNet::TerrainTag(TerrainTagNet::from_tag(*tag)))
            .collect(),
        EditorListNet::EntrySides => draft
            .entry_sides()
            .iter()
            .map(|side| EditorListMemberNet::EntrySide(TerrainFacingNet::from_facing(*side)))
            .collect(),
        EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => Vec::new(),
    }
}

// The member a toggle carries, for the two lists the form draws as tick boxes.
fn toggled_member(op: EditorListOpNet) -> Result<EditorListMemberNet, FormWriteFault> {
    let EditorListOpNet::Toggle(member) = op else {
        return Err(FormWriteFault::bad(TOGGLE_ONLY.to_owned()));
    };
    Ok(member)
}

fn toggle_entry_side(draft: &mut TerrainDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    let EditorListMemberNet::EntrySide(side) = toggled_member(op)? else {
        return Err(wrong_member(EditorListNet::EntrySides));
    };
    if draft.kind() != TerrainKindChoice::Emplacement {
        return Err(FormWriteFault::Gated(NOT_AN_EMPLACEMENT));
    }
    let sides = toggled(draft, side);
    draft.set_entry_sides(sides);
    Ok(())
}

fn toggle_tag(draft: &mut TerrainDraft, op: EditorListOpNet) -> Result<(), FormWriteFault> {
    let EditorListMemberNet::TerrainTag(tag) = toggled_member(op)? else {
        return Err(wrong_member(EditorListNet::TerrainTags));
    };
    draft.toggle_tag(tag.to_tag());
    Ok(())
}

/// Apply one operation to the entry-sides or the tags list.
pub(super) fn apply(
    draft: &mut TerrainDraft,
    list: EditorListNet,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    match list {
        EditorListNet::TerrainTags => toggle_tag(draft, op),
        EditorListNet::EntrySides => toggle_entry_side(draft, op),
        EditorListNet::AttachmentEffects
        | EditorListNet::SpriteFrames
        | EditorListNet::InjuryEffects
        | EditorListNet::MeleeWeaponFightModes
        | EditorListNet::MeleeWeaponSlots
        | EditorListNet::MeleeWeaponAttachments
        | EditorListNet::GangMembers
        | EditorListNet::WeaponFireModes
        | EditorListNet::WeaponSlots
        | EditorListNet::WeaponAttachments
        | EditorListNet::FieldImmuneArmorTypes
        | EditorListNet::WeightingBucket(_) => Err(FormWriteFault::ForeignArm),
    }
}
