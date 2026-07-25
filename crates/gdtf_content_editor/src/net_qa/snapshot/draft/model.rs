//! [`EditorDraftModel`] — the per-mode authoring drafts the draft topic reads (GTW-805).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_qa_protocol::view::EditorDraftFieldView;

use super::{
    armor::armor_fields, attachment::attachment_fields, gang::gang_fields, injury::injury_fields,
    melee::melee_fields, prefab::prefab_fields, sprite::sprite_fields, terrain::terrain_fields,
    theme::theme_fields, weapon::weapon_fields,
};
use crate::{
    ArmorDraft, AttachmentDraft, CurrentEditLevel, EditorMap, EditorMode, GangDraft, InjuryDraft,
    MeleeWeaponDraft, SpriteDraft, TerrainDraft, ThemeDraft, WeaponDraft, WeightingDraft,
};

/// Every authoring draft the Workbench's ten modes edit, as optional reads.
///
/// One [`SystemParam`] rather than ten router parameters — a fn-system's parameter list is
/// capped at 16, and the drafts are one concern (bevy-traps). Every field is
/// `Option<Res<…>>` because the drafts are state-scoped to `Editing` (bevy-traps #1): during
/// the editor's `Load` pass they do not exist, and reading them unguarded would panic.
#[derive(SystemParam)]
pub(in crate::net_qa) struct EditorDraftModel<'w> {
    /// The TERRAIN form's draft.
    terrain:    Option<Res<'w, TerrainDraft>>,
    /// The THEME form's draft.
    theme:      Option<Res<'w, ThemeDraft>>,
    /// The PREFAB mode's painted map (its "draft").
    map:        Option<Res<'w, EditorMap>>,
    /// The storey the PREFAB canvas is painting on.
    level:      Option<Res<'w, CurrentEditLevel>>,
    /// The GANG form's draft.
    gang:       Option<Res<'w, GangDraft>>,
    /// The ARMOR form's draft.
    armor:      Option<Res<'w, ArmorDraft>>,
    /// The INJURY form's def draft.
    injury:     Option<Res<'w, InjuryDraft>>,
    /// The INJURY form's weighting-table draft (the mode's second record).
    weighting:  Option<Res<'w, WeightingDraft>>,
    /// The SPRITE form's draft.
    sprite:     Option<Res<'w, SpriteDraft>>,
    /// The ATTACHMENT form's draft.
    attachment: Option<Res<'w, AttachmentDraft>>,
    /// The ranged WEAPON form's draft.
    weapon:     Option<Res<'w, WeaponDraft>>,
    /// The MELEE-WEAPON form's draft.
    melee:      Option<Res<'w, MeleeWeaponDraft>>,
}

impl EditorDraftModel<'_> {
    /// Whether `mode`'s draft is present — the availability half, asked before a topic is
    /// offered so the editor never advertises a draft it cannot produce.
    ///
    /// A wildcard-free `match`: a new authoring mode must say which resource backs it.
    pub(in crate::net_qa) const fn has(&self, mode: EditorMode) -> bool {
        match mode {
            EditorMode::Terrain => self.terrain.is_some(),
            EditorMode::Theme => self.theme.is_some(),
            EditorMode::Prefab => self.map.is_some(),
            EditorMode::Gang => self.gang.is_some(),
            EditorMode::Armor => self.armor.is_some(),
            EditorMode::Injury => self.injury.is_some(),
            EditorMode::Sprite => self.sprite.is_some(),
            EditorMode::Attachment => self.attachment.is_some(),
            EditorMode::Weapon => self.weapon.is_some(),
            EditorMode::MeleeWeapon => self.melee.is_some(),
        }
    }

    /// `mode`'s draft fields, or [`None`] when that mode's draft resource is absent.
    ///
    /// The same wildcard-free `match` as [`has`](Self::has), so the two can never disagree
    /// about which resource a mode reads.
    pub(in crate::net_qa) fn fields(&self, mode: EditorMode) -> Option<Vec<EditorDraftFieldView>> {
        match mode {
            EditorMode::Terrain => self.terrain.as_deref().map(terrain_fields),
            EditorMode::Theme => self.theme.as_deref().map(theme_fields),
            EditorMode::Prefab => self
                .map
                .as_deref()
                .map(|map| prefab_fields(map, self.level.as_deref())),
            EditorMode::Gang => self.gang.as_deref().map(gang_fields),
            EditorMode::Armor => self.armor.as_deref().map(armor_fields),
            EditorMode::Injury => self
                .injury
                .as_deref()
                .map(|draft| injury_fields(draft, self.weighting.as_deref())),
            EditorMode::Sprite => self.sprite.as_deref().map(sprite_fields),
            EditorMode::Attachment => self.attachment.as_deref().map(attachment_fields),
            EditorMode::Weapon => self.weapon.as_deref().map(weapon_fields),
            EditorMode::MeleeWeapon => self.melee.as_deref().map(melee_fields),
        }
    }
}
