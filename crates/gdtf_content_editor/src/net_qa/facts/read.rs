//! Live editor resources a command's facts are sampled from.

use bevy::{ecs::system::SystemParam, prelude::*};

use super::{DraftInWorld, EditorFacts};
use crate::{
    EditorMode, EditorState,
    armor_form::ArmorDraft,
    attachment_form::AttachmentDraft,
    gang_form::GangDraft,
    injury_form::InjuryDraft,
    melee_weapon_form::MeleeWeaponDraft,
    net_qa::wire::{EditorModeNet, EditorPhaseNet},
    sprite_form::SpriteDraft,
    terrain_form::TerrainDraft,
    theme_form::ThemeDraft,
    weapon_form::WeaponDraft,
};

/// The editor state machine, the mode tab, and every form draft an availability check reads.
#[derive(SystemParam)]
pub(in crate::net_qa) struct EditorFactsParam<'w> {
    state:        Res<'w, State<EditorState>>,
    mode:         Option<Res<'w, EditorMode>>,
    terrain:      Option<Res<'w, TerrainDraft>>,
    theme:        Option<Res<'w, ThemeDraft>>,
    gang:         Option<Res<'w, GangDraft>>,
    armor:        Option<Res<'w, ArmorDraft>>,
    injury:       Option<Res<'w, InjuryDraft>>,
    sprite:       Option<Res<'w, SpriteDraft>>,
    attachment:   Option<Res<'w, AttachmentDraft>>,
    weapon:       Option<Res<'w, WeaponDraft>>,
    melee_weapon: Option<Res<'w, MeleeWeaponDraft>>,
}

impl EditorFactsParam<'_> {
    /// Read the editor's live phase, mode and draft presence into a facts value.
    #[must_use]
    pub(in crate::net_qa) fn sample(&self) -> EditorFacts {
        let mode = self
            .mode
            .as_ref()
            .map(|mode| EditorModeNet::from_mode(**mode));
        EditorFacts::new(
            EditorPhaseNet::from_state(self.state.get()),
            mode,
            self.draft_of(mode),
        )
    }

    // The Prefab tab is the canvas, not a form, so it names no draft to find.
    const fn draft_of(&self, mode: Option<EditorModeNet>) -> DraftInWorld {
        let present = match mode {
            Some(EditorModeNet::Terrain) => self.terrain.is_some(),
            Some(EditorModeNet::Theme) => self.theme.is_some(),
            Some(EditorModeNet::Gang) => self.gang.is_some(),
            Some(EditorModeNet::Armor) => self.armor.is_some(),
            Some(EditorModeNet::Injury) => self.injury.is_some(),
            Some(EditorModeNet::Sprite) => self.sprite.is_some(),
            Some(EditorModeNet::Attachment) => self.attachment.is_some(),
            Some(EditorModeNet::Weapon) => self.weapon.is_some(),
            Some(EditorModeNet::MeleeWeapon) => self.melee_weapon.is_some(),
            Some(EditorModeNet::Prefab) | None => false,
        };
        DraftInWorld::new(present)
    }
}
