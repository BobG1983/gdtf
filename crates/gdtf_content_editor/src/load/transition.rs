//! `Update` (during [`EditorState::Load`](crate::EditorState)): leave `Load` for
//! `Editing` once all registries + the tile-role table are present.

use bevy::prelude::*;
use gdtf_battle_presenter::TileRoles;
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

use crate::EditorState;

/// Transitions [`EditorState::Load`] → [`EditorState::Editing`] once the legacy
/// [`WeaponRegistry`] / [`ArmorRegistry`], the (GTW-487) NEW [`TerrainDefRegistry`] /
/// [`UuidThemeRegistry`], the (GTW-636) GANG mode's [`GangRegistry`] /
/// [`MeleeWeaponRegistry`], the (GTW-654) INJURY mode's [`InjuryRegistry`] /
/// [`InjuryTables`] pair, the (GTW-663) [`SpriteDefRegistry`], and the (GTW-495)
/// [`TileRoles`] are all inserted (each seam
/// resolve inserts its resource on success OR on its const-default / empty failure fallback —
/// GTW-579 C4b — so this is reached even on a bad asset folder: the no-strand guarantee).
/// The game's `GdtfTheme` is NOT gated on — the egui shell styles itself, so the editor
/// resolves no theme (GTW-625; the GTW-579 AC2 amendment).
/// Takes them as `Option<Res<…>>` (bundled in [`GateResources`]) and only sets the next state
/// when all are present, so it never panics on an absent resource (`bevy-traps.md` #1).
pub(crate) fn transition_to_editing(gate: GateResources, mut next: ResMut<NextState<EditorState>>) {
    if gate.all_present() {
        next.set(EditorState::Editing);
    }
}

/// The ten resolved-resource borrows the transition gates on, bundled into one
/// `#[derive(SystemParam)]` (the shell's `PrefabParams` pattern) so the gate system's
/// signature stays legible as families accrue. Every field is `Option` — each resource
/// arrives only once its seam resolve (or fallback) fires (bevy-traps #1).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GateResources<'w> {
    /// The ranged-weapons registry (`WeaponsFamily`).
    weapons:       Option<Res<'w, WeaponRegistry>>,
    /// The armor registry (`ArmorFamily`).
    armor:         Option<Res<'w, ArmorRegistry>>,
    /// The UUID-keyed terrain defs (`TerrainDefsFamily` — GTW-487).
    terrain_defs:  Option<Res<'w, TerrainDefRegistry>>,
    /// The UUID-keyed theme defs (`ThemeDefsFamily` — GTW-487).
    theme_defs:    Option<Res<'w, UuidThemeRegistry>>,
    /// The gangs registry (`GangsFamily` — GTW-636).
    gangs:         Option<Res<'w, GangRegistry>>,
    /// The melee-weapons registry (`MeleeWeaponsFamily` — GTW-636).
    melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    /// The injury-def registry (the bespoke injuries pass — GTW-654).
    injuries:      Option<Res<'w, InjuryRegistry>>,
    /// The built injury weighting tables (the injuries pass's second resource —
    /// GTW-654; both are published atomically by the shared builder).
    injury_tables: Option<Res<'w, InjuryTables>>,
    /// The sprite-def registry (`SpriteDefsFamily` — GTW-663).
    sprite_defs:   Option<Res<'w, SpriteDefRegistry>>,
    /// The presenter tile-role table (the GTW-564 hot-RON chain — GTW-495).
    tile_roles:    Option<Res<'w, TileRoles>>,
}

impl GateResources<'_> {
    /// Whether EVERY gate resource has resolved (or fallen back) — the release condition.
    const fn all_present(&self) -> bool {
        self.weapons.is_some()
            && self.armor.is_some()
            && self.terrain_defs.is_some()
            && self.theme_defs.is_some()
            && self.gangs.is_some()
            && self.melee_weapons.is_some()
            && self.injuries.is_some()
            && self.injury_tables.is_some()
            && self.sprite_defs.is_some()
            && self.tile_roles.is_some()
    }
}
