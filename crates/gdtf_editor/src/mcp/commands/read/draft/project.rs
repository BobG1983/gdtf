//! Project one mode's draft through its own conversion, then the one RON serializer.

use cobalt_ron_assets::serialize_ron_pretty;
use gdtf_battle_sim::terrain::def::TerrainUuid;
use serde::Serialize;

use super::drafts::FormDrafts;
use crate::{
    EditorMode, armor_form, attachment_form, field_form, gang_form, injury_form,
    mcp::wire::{EditorDraftOutcomeNet, EditorDraftRonNet, EditorSaveFaultNet},
    melee_weapon_form,
    save_record::EditorSaveFault,
    sprite_form,
    terrain_form::{self, TerrainDraft},
    theme_form, weapon_form,
};

/// What reading the active mode's draft produced.
pub(super) enum DraftProjection {
    /// The draft projected, or reported why it will not.
    Made(EditorDraftOutcomeNet),
    /// That mode's draft resource is not in the world.
    Missing,
    /// Prefab carries no draft; its state is the map and the session.
    NoDraft,
}

// The one serialisation, the same call `write_ron_pretty` opens with, so the text is the file.
fn ron_of<T: Serialize>(value: &T) -> EditorDraftOutcomeNet {
    match serialize_ron_pretty(value) {
        Ok(text) => EditorDraftOutcomeNet::Ron(EditorDraftRonNet::new(text)),
        Err(fault) => not_savable(EditorSaveFault::from(fault)),
    }
}

fn not_savable(fault: EditorSaveFault) -> EditorDraftOutcomeNet {
    EditorDraftOutcomeNet::NotSavable(EditorSaveFaultNet::from_fault(&fault))
}

// The read-only key route the terrain preview pane takes; the save path's `ensure_uuid` writes.
fn terrain(draft: &TerrainDraft) -> EditorDraftOutcomeNet {
    let key = draft.uuid().unwrap_or_else(TerrainUuid::nil);
    match terrain_form::draft_to_terrain_def(draft, key) {
        Ok(def) => ron_of(&def),
        Err(fault) => not_savable(EditorSaveFault::from(fault)),
    }
}

/// Project `mode`'s draft, or say why there is nothing to project.
pub(super) fn project(mode: EditorMode, drafts: &FormDrafts<'_>) -> DraftProjection {
    match mode {
        EditorMode::Prefab => DraftProjection::NoDraft,
        EditorMode::Terrain => made(drafts.terrain.as_deref().map(terrain)),
        EditorMode::Theme => made(
            drafts
                .theme
                .as_deref()
                .map(|draft| ron_of(&theme_form::draft_to_theme_def(draft, draft.key()))),
        ),
        EditorMode::Gang => made(
            drafts
                .gang
                .as_deref()
                .map(|draft| ron_of(&gang_form::draft_to_roster(draft).1)),
        ),
        EditorMode::Armor => made(
            drafts
                .armor
                .as_deref()
                .map(|draft| ron_of(&armor_form::draft_to_spec(draft).1)),
        ),
        EditorMode::Injury => made(
            drafts
                .injury
                .as_deref()
                .map(|draft| ron_of(&injury_form::draft_to_def(draft).1)),
        ),
        EditorMode::Sprite => made(
            drafts
                .sprite
                .as_deref()
                .map(|draft| ron_of(&sprite_form::draft_to_sprite_def(draft).1)),
        ),
        EditorMode::Attachment => made(
            drafts
                .attachment
                .as_deref()
                .map(|draft| ron_of(&attachment_form::draft_to_attachment_spec(draft).1)),
        ),
        EditorMode::Weapon => made(
            drafts
                .weapon
                .as_deref()
                .map(|draft| ron_of(&weapon_form::draft_to_weapon_spec(draft).1)),
        ),
        EditorMode::Field => made(
            drafts
                .field
                .as_deref()
                .map(|draft| ron_of(&field_form::draft_to_field(draft).1)),
        ),
        EditorMode::MeleeWeapon => made(
            drafts
                .melee_weapon
                .as_deref()
                .map(|draft| ron_of(&melee_weapon_form::draft_to_melee_weapon_spec(draft).1)),
        ),
    }
}

// An absent draft resource is `Missing`, never a silent empty projection.
fn made(outcome: Option<EditorDraftOutcomeNet>) -> DraftProjection {
    outcome.map_or(DraftProjection::Missing, DraftProjection::Made)
}
