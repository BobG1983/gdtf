//! The [`present_fog`] writer system: the public seam that modulates the rendered
//! terrain layer and hard-cuts actor sprites from the sim's
//! [`SquadVisibility`](gdtf_battle_sim::SquadVisibility).

use bevy::prelude::*;
use gdtf_battle_sim::{
    CellLevel, CombatTuning, Faction, FactionRelation, Level, LifeState, PlayerFaction, Position,
    SquadVisibility, is_ganger_visible,
};

use crate::{GangerSprite, GangerSprites, TerrainSprite};

/// Build the identity terrain modulate — full colour, the atlas tile's own pixels read
/// through unchanged.
///
/// A VISIBLE cell renders at full colour, so its [`TerrainSprite`] is modulated by white
/// (the multiplicative identity for a sprite tint). [`Color`] is framework plumbing; this
/// is the CHOICE the fog applies on a squad-VISIBLE cell.
pub(super) const fn visible_modulate() -> Color {
    Color::WHITE
}

/// Build the EXPLORED terrain modulate — the identity colour's RGB scaled by `dim`, with
/// the alpha left at full.
///
/// An EXPLORED (seen-before, not-now) cell renders DIMMED: its RGB is multiplied by
/// `explored_dim` while alpha is untouched (`docs/combat/visibility.md` §"Tunables" — "RGB
/// × this, alpha untouched"). Derived from the [`visible_modulate`] identity each run so
/// the dim never compounds across updates. `dim` is the dimensionless
/// [`ExploredDim`](gdtf_battle_sim::CombatTuning) factor read from tuning, never a
/// presenter literal.
pub(super) fn explored_modulate(dim: f32) -> Color {
    // Scale the identity white's RGB by the tunable dim; keep alpha at the identity's
    // full opacity (the "alpha untouched" rule).
    let base = visible_modulate().to_srgba();
    Color::srgb(base.red * dim, base.green * dim, base.blue * dim)
}

/// The fog treatment a `(cell, level)` resolves to, for a sprite the fog modulates.
///
/// A named view-domain decision (no bare tuple / option): the three states map to the
/// three rendered treatments. UNSEEN hides the sprite; the other two show it with a
/// modulate colour.
pub(super) enum CellFog {
    /// Squad-VISIBLE — shown at full identity colour.
    Visible,
    /// Squad-EXPLORED (not VISIBLE) — shown dimmed by `explored_dim`.
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
/// # Terrain (per-cell modulate)
///
/// For every [`TerrainSprite`] on the active level it sets the sprite's
/// [`Sprite::color`] + [`Visibility`] by the fog state of the marker's
/// [`CellLevel`](gdtf_battle_sim::CellLevel) key: VISIBLE → full identity
/// ([`Color::WHITE`], shown); EXPLORED → RGB × `explored_dim` (read off
/// [`CombatTuning`], alpha untouched, shown); UNSEEN →
/// [`Visibility::Hidden`]. The colour is always re-derived from the identity base so the
/// EXPLORED dim never compounds, and it MUTATES the existing sprite — never despawn +
/// respawn (the UI-mutate-not-respawn convention).
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
/// ([`SquadVisibility`] / [`CombatTuning`] / the optional [`PlayerFaction`] /
/// [`GangerSprites`] map), the [`TerrainSprite`] modulate query, the sim-ganger
/// [`Position`] / [`Faction`] / [`LifeState`] query, and the actor-sprite
/// [`Visibility`] query. It takes no [`Commands`] — every change is an in-place mutate.
#[expect(
    clippy::too_many_arguments,
    reason = "the genuine read params (four resources) plus the three disjoint queries \
              the fog writer needs; grouping into a SystemParam bundle would not reduce \
              the count and would obscure the per-arg docs"
)]
pub fn present_fog(
    squad: Res<SquadVisibility>,
    tuning: Res<CombatTuning>,
    player: Option<Res<PlayerFaction>>,
    sprites: Res<GangerSprites>,
    active: Res<crate::ActiveLevel>,
    mut terrain: Query<(&TerrainSprite, &mut Sprite, &mut Visibility)>,
    gangers: Query<(Entity, &Position, &Faction, &LifeState)>,
    mut actors: Query<&mut Visibility, (With<GangerSprite>, Without<TerrainSprite>)>,
) {
    let dim = *tuning.explored_dim;
    present_terrain_fog(&squad, dim, &mut terrain);
    present_actor_fog(
        &squad,
        player.as_deref().copied(),
        **active,
        &sprites,
        &gangers,
        &mut actors,
    );
}

/// Modulate every active-level [`TerrainSprite`] by its cell's fog state (the terrain arm
/// of [`present_fog`]).
///
/// Re-derives the colour from the identity base each run (no compounding) and mutates the
/// sprite in place. UNSEEN hides via [`Visibility::Hidden`]; the two shown states restore
/// [`Visibility::Inherited`] (so a cell that re-enters sight from UNSEEN shows again).
fn present_terrain_fog(
    squad: &SquadVisibility,
    dim: f32,
    terrain: &mut Query<(&TerrainSprite, &mut Sprite, &mut Visibility)>,
) {
    for (marker, mut sprite, mut visibility) in terrain {
        match CellFog::resolve(squad, &marker.at) {
            CellFog::Visible => {
                sprite.color = visible_modulate();
                *visibility = Visibility::Inherited;
            }
            CellFog::Explored => {
                sprite.color = explored_modulate(dim);
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
