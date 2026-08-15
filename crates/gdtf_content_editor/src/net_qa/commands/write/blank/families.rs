//! Replace one form's draft with that form's own blank-draft constructor.

use crate::{
    EditorMode, armor_form::ArmorDraft, attachment_form::AttachmentDraft, gang_form::GangDraft,
    injury_form::InjuryDraft, melee_weapon_form::MeleeWeaponDraft, net_qa::forms::EditorForms,
    sprite_form::SpriteDraft, weapon_form::WeaponDraft,
};

/// Whether the mode's draft was blanked, or its resource was absent from the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::net_qa::commands::write::blank) enum BlankResult {
    /// The draft was replaced with a blank one.
    Blanked,
    /// The draft resource was not in the world.
    NoDraft,
}

/// Blank the seven drafts whose form draws a New button; every other mode is refused earlier.
pub(in crate::net_qa::commands::write::blank) fn blank_draft(
    mode: EditorMode,
    forms: &mut EditorForms<'_>,
) -> BlankResult {
    let blanked = match mode {
        EditorMode::Gang => forms.gang.as_deref_mut().map(blank_gang),
        EditorMode::Armor => forms.armor.as_deref_mut().map(blank_armor),
        EditorMode::Injury => forms.injury.as_deref_mut().map(blank_injury),
        EditorMode::Sprite => forms.sprite.as_deref_mut().map(blank_sprite),
        EditorMode::Attachment => forms.attachment.as_deref_mut().map(blank_attachment),
        EditorMode::Weapon => forms.weapon.as_deref_mut().map(blank_weapon),
        EditorMode::MeleeWeapon => forms.melee_weapon.as_deref_mut().map(blank_melee_weapon),
        EditorMode::Terrain | EditorMode::Theme | EditorMode::Prefab => None,
    };
    match blanked {
        Some(()) => BlankResult::Blanked,
        None => BlankResult::NoDraft,
    }
}

pub(in crate::net_qa::commands::write::blank) fn blank_gang(draft: &mut GangDraft) {
    *draft = GangDraft::new_gang();
}

pub(in crate::net_qa::commands::write::blank) fn blank_armor(draft: &mut ArmorDraft) {
    *draft = ArmorDraft::new_armor();
}

pub(in crate::net_qa::commands::write::blank) fn blank_injury(draft: &mut InjuryDraft) {
    *draft = InjuryDraft::new_injury();
}

pub(in crate::net_qa::commands::write::blank) fn blank_sprite(draft: &mut SpriteDraft) {
    *draft = SpriteDraft::new_sprite();
}

pub(in crate::net_qa::commands::write::blank) fn blank_attachment(draft: &mut AttachmentDraft) {
    *draft = AttachmentDraft::new_attachment();
}

pub(in crate::net_qa::commands::write::blank) fn blank_weapon(draft: &mut WeaponDraft) {
    *draft = WeaponDraft::new_weapon();
}

pub(in crate::net_qa::commands::write::blank) fn blank_melee_weapon(draft: &mut MeleeWeaponDraft) {
    *draft = MeleeWeaponDraft::new_melee_weapon();
}
