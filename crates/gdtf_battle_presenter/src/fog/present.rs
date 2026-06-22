//! The [`present_fog`] writer system: the public seam that desaturates the rendered
//! terrain layer (GTW-348) and hard-cuts actor sprites from the sim's
//! [`SquadVisibility`](gdtf_battle_sim::SquadVisibility).

use bevy::prelude::*;
use gdtf_battle_sim::{
    CellLevel, Faction, FactionRelation, Level, LifeState, PlayerFaction, Position,
    SquadVisibility, is_ganger_visible,
};

use crate::{GangerSprite, GangerSprites, TerrainFogMaterial, TerrainSprite};

/// The [`TerrainFogMaterial`] saturation for a squad-VISIBLE cell — full colour (the atlas
/// tile's own pixels read through unchanged).
const VISIBLE_SATURATION: f32 = 1.0;

/// The [`TerrainFogMaterial`] saturation for a squad-EXPLORED cell — full GREYSCALE
/// (GTW-348: EXPLORED renders at the SAME brightness but with its colour removed, so
/// colour-loss is the memory cue, not brightness-loss). `0.0` mixes the tile fully toward
/// its BT.709 luminance in the shader.
const EXPLORED_SATURATION: f32 = 0.0;

/// The fog treatment a `(cell, level)` resolves to, for a terrain tile the fog modulates.
///
/// A named view-domain decision (no bare tuple / option): the three states map to the
/// three rendered treatments. UNSEEN hides the tile; the other two show it at a
/// [`TerrainFogMaterial`] saturation (VISIBLE full colour, EXPLORED full greyscale).
pub(super) enum CellFog {
    /// Squad-VISIBLE — shown at full colour ([`VISIBLE_SATURATION`]).
    Visible,
    /// Squad-EXPLORED (not VISIBLE) — shown at full-brightness greyscale
    /// ([`EXPLORED_SATURATION`]).
    Explored,
    /// UNSEEN — hidden (the dark clear colour reads through).
    Unseen,
}

impl CellFog {
    /// Resolve the fog treatment for `key` from the squad sets (VISIBLE wins over
    /// EXPLORED; neither is UNSEEN).
    pub(super) fn resolve(squad: &SquadVisibility, key: &CellLevel) -> Self {
        if squad.is_cell_visible(key) {
            Self::Visible
        } else if squad.is_cell_explored(key) {
            Self::Explored
        } else {
            Self::Unseen
        }
    }
}

/// `Update` ([`PresenterSystems::Draw`](crate::PresenterSystems), ordered
/// `.after(draw_static_battlefield).after(swap_destroyed_cover)` and after the ganger
/// storey-filter writers): the presenter FOG WRITER — the documented public seam GTW-342
/// exposes.
///
/// It is the VIEW arm of the squad fog (`docs/combat/visibility.md`): the sim owns the
/// three [`SquadVisibility`] states and recomputes them; this system READS them through
/// the pure seams and MODULATES the already-drawn layer in place — it never owns fog and
/// never repaints from a snapshot (the rendered layer IS the fog mask).
///
/// # Terrain (per-cell desaturate)
///
/// For every [`TerrainSprite`] on the active level it sets the tile's
/// [`TerrainFogMaterial`] `saturation` + [`Visibility`] by the fog state of the marker's
/// [`CellLevel`](gdtf_battle_sim::CellLevel) key: VISIBLE → full colour (`saturation` 1.0,
/// shown); EXPLORED → FULL-brightness GREYSCALE (`saturation` 0.0 — colour-loss as the
/// memory cue, GTW-348, shown); UNSEEN → [`Visibility::Hidden`]. It MUTATES the existing
/// material in place via [`Assets::get_mut`](bevy::asset::Assets::get_mut) (which re-uploads
/// the uniform next frame) — never despawn + respawn (the UI-mutate-not-respawn convention).
///
/// # Actors (per-entity hard-cut)
///
/// For every mapped ganger sprite it sets the sprite's [`Visibility`] as the COMPOSITION
/// of the slice's storey fact (`pos.z == active`) AND the fog fact — so it is the single
/// final writer of the actor [`Visibility`] (it re-reads, never crosses, the slice's
/// storey filter): a PLAYER-faction ganger is always shown (storey permitting); an ENEMY
/// ganger is a HARD CUT — [`Visibility::Inherited`] iff its cell is squad-VISIBLE
/// ([`is_ganger_visible`]), no fade / no last-known ghost; a CORPSE (a
/// [`Downed`](LifeState::Downed) / [`Dead`](LifeState::Dead) ganger) is shown iff its
/// cell is squad-VISIBLE. The actor's COLOUR stays the ganger-tint writers' domain
/// (`docs/combat/visibility.md` actor = Visibility hard-cut); fog only sets the actor
/// flag.
///
/// Param-only (`bevy-traps.md` #7): the read [`Res`]ources
/// ([`SquadVisibility`] / the optional [`PlayerFaction`] / [`GangerSprites`] map /
/// [`ActiveLevel`](crate::ActiveLevel)), the terrain materials store
/// ([`ResMut<Assets<TerrainFogMaterial>>`] — GTW-348, the terrain arm drives each tile's
/// saturation in place), the [`TerrainSprite`] / [`MeshMaterial2d`] terrain query, the
/// sim-ganger [`Position`] / [`Faction`] / [`LifeState`] query, and the actor-sprite
/// [`Visibility`] query. It takes no [`Commands`] — every change is an in-place mutate.
#[expect(
    clippy::too_many_arguments,
    reason = "the genuine read params (the resources + the materials store) plus the three \
              disjoint queries the fog writer needs; grouping into a SystemParam bundle would \
              not reduce the count and would obscure the per-arg docs"
)]
pub fn present_fog(
    squad: Res<SquadVisibility>,
    player: Option<Res<PlayerFaction>>,
    sprites: Res<GangerSprites>,
    active: Res<crate::ActiveLevel>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    terrain: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Visibility,
    )>,
    gangers: Query<(Entity, &Position, &Faction, &LifeState)>,
    mut actors: Query<&mut Visibility, (With<GangerSprite>, Without<TerrainSprite>)>,
) {
    present_terrain_fog(&squad, &mut materials, terrain);
    present_actor_fog(
        &squad,
        player.as_deref().copied(),
        **active,
        &sprites,
        &gangers,
        &mut actors,
    );
}

/// Desaturate every active-level terrain tile by its cell's fog state (the terrain arm of
/// [`present_fog`]).
///
/// Drives each tile's [`TerrainFogMaterial`] `saturation` in place via
/// [`Assets::get_mut`](bevy::asset::Assets::get_mut): VISIBLE → full colour, EXPLORED →
/// full-brightness greyscale (GTW-348). UNSEEN hides via [`Visibility::Hidden`]; the two
/// shown states restore [`Visibility::Inherited`] (so a cell that re-enters sight from
/// UNSEEN shows again). Setting the value unconditionally each run (not only on change)
/// keeps the tile consistent after a saturation flip with no compounding — the material is
/// the single source, never a stacked modulate.
fn present_terrain_fog(
    squad: &SquadVisibility,
    materials: &mut Assets<TerrainFogMaterial>,
    mut terrain: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Visibility,
    )>,
) {
    for (marker, mat_handle, mut visibility) in &mut terrain {
        match CellFog::resolve(squad, &marker.at) {
            CellFog::Visible => {
                if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                    material.saturation = VISIBLE_SATURATION;
                }
                *visibility = Visibility::Inherited;
            }
            CellFog::Explored => {
                if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                    material.saturation = EXPLORED_SATURATION;
                }
                *visibility = Visibility::Inherited;
            }
            CellFog::Unseen => {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// Hard-cut every mapped actor sprite by composing the slice storey fact AND the fog fact
/// (the actor arm of [`present_fog`]).
///
/// For each live sim ganger, look its presenter sprite up through [`GangerSprites`] and
/// set that sprite's [`Visibility`] to [`Visibility::Inherited`] iff it is on the active
/// storey AND fog shows it, else [`Visibility::Hidden`]. The fog fact:
///
/// * a PLAYER-faction, [`Alive`](LifeState::Alive) ganger is always shown
///   ([`FactionRelation::OwnSquad`] is trivially visible);
/// * an ENEMY [`Alive`](LifeState::Alive) ganger is shown iff its cell is squad-VISIBLE
///   (the [`FactionRelation::Other`] hard cut — no fade, no ghost);
/// * a CORPSE (a [`Downed`](LifeState::Downed) / [`Dead`](LifeState::Dead) ganger, of
///   either faction) is shown iff its cell is squad-VISIBLE — a downed/dead body is not a
///   live observer, so it is fog-gated like an enemy regardless of faction.
fn present_actor_fog(
    squad: &SquadVisibility,
    player: Option<PlayerFaction>,
    active: Level,
    sprites: &GangerSprites,
    gangers: &Query<(Entity, &Position, &Faction, &LifeState)>,
    actors: &mut Query<&mut Visibility, (With<GangerSprite>, Without<TerrainSprite>)>,
) {
    for (entity, pos, faction, life) in gangers {
        let Some(sprite) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = actors.get_mut(sprite) else {
            continue;
        };
        let on_active_storey = pos.z == i32::from(*active);
        let key: CellLevel = **pos;
        let shown_by_fog = is_ganger_visible(squad, &key, actor_relation(player, *faction, *life));
        *visibility = if on_active_storey && shown_by_fog {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// The squad-fog [`FactionRelation`] a ganger sprite is gated by.
///
/// A live ([`Alive`](LifeState::Alive)) PLAYER-faction ganger is
/// [`FactionRelation::OwnSquad`] (always shown); everything else — an ENEMY ganger, or a
/// CORPSE of either faction (a downed/dead body is no longer a member of the seeing
/// squad) — is [`FactionRelation::Other`], shown only on a squad-VISIBLE cell. `player`
/// is [`None`] when no [`PlayerFaction`] is resident (a focused harness), in which case
/// every ganger is treated as [`FactionRelation::Other`] (fog-gated) — fail-closed.
pub(super) fn actor_relation(
    player: Option<PlayerFaction>,
    faction: Faction,
    life: LifeState,
) -> FactionRelation {
    let is_player = player.is_some_and(|p| *p == faction);
    if is_player && life.is_active() {
        FactionRelation::OwnSquad
    } else {
        FactionRelation::Other
    }
}
