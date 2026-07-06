//! The [`present_fog`] writer system: the public seam that desaturates the rendered
//! terrain layer (GTW-348) and hard-cuts actor sprites from the sim's
//! [`SquadVisibility`](gdtf_battle_sim::SquadVisibility).

use bevy::prelude::*;
use gdtf_battle_sim::{
    CellLevel, Faction, FactionRelation, Level, LifeState, PlayerFaction, Position,
    SquadVisibility, is_ganger_visible,
};

use crate::{Brightness, GangerSprite, GangerSprites, TerrainFogMaterial, TerrainSprite, ViewMode};

/// The [`TerrainFogMaterial`] saturation for a squad-VISIBLE cell — full colour (the atlas
/// tile's own pixels read through unchanged).
const VISIBLE_SATURATION: f32 = 1.0;

/// The [`TerrainFogMaterial`] saturation for a squad-EXPLORED cell — full GREYSCALE
/// (GTW-348: EXPLORED renders at the SAME brightness but with its colour removed, so
/// colour-loss is the memory cue, not brightness-loss). `0.0` mixes the tile fully toward
/// its BT.709 luminance in the shader.
const EXPLORED_SATURATION: f32 = 0.0;

/// The [`Brightness`] a terrain tile on a LOWER drawn storey renders at (GTW-519 — the
/// UFO:EU / `OpenXcom` multi-level darken): a single tunable dim `< 1.0`, so a storey BELOW
/// the active view level draws visibly darker than the full-bright active storey.
///
/// The ONE knob for the storey-depth darken axis. It is a SEPARATE axis from the fog
/// [`saturation`](TerrainFogMaterial::saturation) (the EXPLORED colour-loss): a lower
/// EXPLORED tile is BOTH greyscaled (saturation) AND dimmed (this brightness), the two
/// composing in the shader (grey-mix, then scale). It is deliberately NOT the DEPRECATED
/// `explored_dim` (the fog EXPLORED cue is colour-loss, never dimmed —
/// `docs/combat/visibility.md`); this dim expresses storey DEPTH, not fog state. `0.55` is a
/// legible-but-clearly-recessed dim, in-engine adjustable in the GTW-388 QA pass.
const LOWER_STOREY_BRIGHTNESS: Brightness = Brightness::new(0.55);

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

/// `Update` ([`PresenterSystems::Compose`](crate::PresenterSystems) — the fog-composition
/// stage, chained strictly after the `Scene` stage that holds the terrain draw, the
/// destruction swaps, and the ganger storey-filter writers, GTW-623): the presenter FOG
/// WRITER — the documented public seam GTW-342 exposes.
///
/// It is the VIEW arm of the squad fog (`docs/combat/visibility.md`): the sim owns the
/// three [`SquadVisibility`] states and recomputes them; this system READS them through
/// the pure seams and MODULATES the already-drawn layer in place — it never owns fog and
/// never repaints from a snapshot (the rendered layer IS the fog mask).
///
/// # Terrain (per-cell desaturate + per-storey darken)
///
/// For every DRAWN [`TerrainSprite`] (the whole `[0..=active]` band since GTW-519, not just
/// the active storey) it sets the tile's [`TerrainFogMaterial`] over TWO orthogonal axes that
/// COMPOSE in the shader:
///
/// * `saturation` + [`Visibility`] by the marker's fog STATE (GTW-348): VISIBLE → full colour
///   (`saturation` 1.0, shown); EXPLORED → FULL-brightness GREYSCALE (`saturation` 0.0 —
///   colour-loss as the memory cue, shown); UNSEEN → [`Visibility::Hidden`]. This drives ALL
///   drawn storeys, so a lower-storey EXPLORED cell still desaturates and a lower-storey
///   UNSEEN cell still hides.
/// * `brightness` by the tile's storey DEPTH (GTW-519 — the UFO:EU multi-level darken): the
///   ACTIVE view storey is [`Brightness::FULL`] (full-bright), a LOWER drawn storey is
///   [`LOWER_STOREY_BRIGHTNESS`] (`< 1.0`, dimmed). A SEPARATE axis from the fog saturation —
///   they multiply (grey-mix, THEN scale), so a lower EXPLORED tile is greyscaled AND dimmed.
///
/// It MUTATES the existing material in place via
/// [`Assets::get_mut`](bevy::asset::Assets::get_mut) (which re-uploads the uniform next frame)
/// — never despawn + respawn (the UI-mutate-not-respawn convention).
///
/// # Actors (per-entity hard-cut)
///
/// For every mapped ganger sprite it sets the sprite's [`Visibility`] as the COMPOSITION
/// of the slice's DRAWN-BAND storey fact (GTW-520 — the storey lies within `0..=active`, via
/// the shared [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate the
/// spawn / move / level-filter sites also use) AND the fog fact — so it is the single final
/// writer of the actor [`Visibility`] (it re-reads, never crosses, the slice's band filter): a
/// PLAYER-faction ganger is always shown (band permitting — including on a LOWER drawn storey);
/// an ENEMY ganger is a HARD CUT — [`Visibility::Inherited`] iff its cell is squad-VISIBLE
/// ([`is_ganger_visible`]), no fade / no last-known ghost; a CORPSE (a
/// [`Downed`](LifeState::Downed) / [`Dead`](LifeState::Dead) ganger) is shown iff its
/// cell is squad-VISIBLE. GTW-520 widened ONLY the storey axis (active-storey hard cut →
/// drawn-band membership); the FOG predicate is unchanged, so an UNSEEN enemy on a lower drawn
/// storey is still HIDDEN even though its storey is drawn. The actor's COLOUR stays the
/// ganger-tint writers' domain (`docs/combat/visibility.md` actor = Visibility hard-cut); fog
/// only sets the actor flag.
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
    view: Res<ViewMode>,
    mut materials: ResMut<Assets<TerrainFogMaterial>>,
    terrain: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Visibility,
    )>,
    gangers: Query<(Entity, &Position, &Faction, &LifeState)>,
    mut actors: Query<&mut Visibility, (With<GangerSprite>, Without<TerrainSprite>)>,
) {
    present_terrain_fog(&squad, **active, &mut materials, terrain);
    // GTW-521: the actor arm is the SINGLE FINAL Visibility writer, so it must compose the
    // ViewMode-widened storey band (else a player ganger on an upper storey re-appears via
    // apply_active_level_filter but is re-hidden here in FullView). The fog fact is unchanged.
    present_actor_fog(
        &squad,
        player.as_deref().copied(),
        **active,
        *view,
        &sprites,
        &gangers,
        &mut actors,
    );
}

/// Modulate every DRAWN terrain tile by TWO orthogonal axes — its cell's fog STATE
/// (saturation) and its storey DEPTH (brightness) — the terrain arm of [`present_fog`]
/// (GTW-348 + GTW-519).
///
/// Drives each tile's [`TerrainFogMaterial`] IN PLACE via
/// [`Assets::get_mut`](bevy::asset::Assets::get_mut) over BOTH knobs, which COMPOSE in the
/// shader (grey-mix by saturation, THEN scale by brightness — never one replacing the other):
///
/// * `saturation` (the fog COLOUR-LOSS axis, GTW-348): VISIBLE → full colour, EXPLORED →
///   full-brightness greyscale. UNSEEN hides via [`Visibility::Hidden`]; the two shown states
///   restore [`Visibility::Inherited`] (so a cell that re-enters sight from UNSEEN shows
///   again). This drives ALL drawn storeys — a lower-storey EXPLORED cell still desaturates,
///   a lower-storey UNSEEN cell still hides (C5).
/// * `brightness` (the storey-DEPTH darken axis, GTW-519): the tile on the ACTIVE view storey
///   is [`Brightness::FULL`] (full-bright); a tile on a LOWER drawn storey is
///   [`LOWER_STOREY_BRIGHTNESS`] (`< 1.0`, dimmed) — the UFO:EU multi-level look. Applied to
///   BOTH shown states (a lower VISIBLE tile is dimmed too), so depth reads regardless of fog
///   state. `active` is the [`ActiveLevel`](crate::ActiveLevel)'s [`Level`]; a tile's storey
///   is its marker `at.z` (`CellLevel` `Deref<Target = IVec3>`).
///
/// Setting BOTH values unconditionally each run (not only on change) keeps the tile
/// consistent after a fog flip / level cycle with no compounding — the material is the single
/// source, never a stacked modulate. An UNSEEN tile is only hidden (its material knobs are
/// left as-is — invisible, so they do not matter — and re-driven the run it becomes shown).
fn present_terrain_fog(
    squad: &SquadVisibility,
    active: Level,
    materials: &mut Assets<TerrainFogMaterial>,
    mut terrain: Query<(
        &TerrainSprite,
        &MeshMaterial2d<TerrainFogMaterial>,
        &mut Visibility,
    )>,
) {
    for (marker, mat_handle, mut visibility) in &mut terrain {
        // The storey-depth brightness (GTW-519): FULL on the active view storey, dimmed on a
        // lower drawn storey. Orthogonal to the fog saturation below — a lower EXPLORED tile
        // ends up greyscaled AND dimmed (the two axes compose in the shader).
        let brightness = if marker.at.z == i32::from(*active) {
            Brightness::FULL
        } else {
            LOWER_STOREY_BRIGHTNESS
        };
        match CellFog::resolve(squad, &marker.at) {
            CellFog::Visible => {
                if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                    material.saturation = VISIBLE_SATURATION;
                    material.brightness = brightness;
                }
                *visibility = Visibility::Inherited;
            }
            CellFog::Explored => {
                if let Some(mut material) = materials.get_mut(mat_handle.id()) {
                    material.saturation = EXPLORED_SATURATION;
                    material.brightness = brightness;
                }
                *visibility = Visibility::Inherited;
            }
            CellFog::Unseen => {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

/// Hard-cut every mapped actor sprite by composing the slice DRAWN-BAND storey fact AND the
/// fog fact (the actor arm of [`present_fog`]).
///
/// For each live sim ganger, look its presenter sprite up through [`GangerSprites`] and
/// set that sprite's [`Visibility`] to [`Visibility::Inherited`] iff its storey lies within
/// the DRAWN band `0..=active` (GTW-520 — the shared
/// [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate; so a lower drawn
/// storey passes, one strictly above active is culled) AND fog shows it, else
/// [`Visibility::Hidden`]. The fog fact (UNCHANGED by GTW-520 — only the storey axis widened):
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
    view: ViewMode,
    sprites: &GangerSprites,
    gangers: &Query<(Entity, &Position, &Faction, &LifeState)>,
    actors: &mut Query<&mut Visibility, (With<GangerSprite>, Without<TerrainSprite>)>,
) {
    // GTW-520 C4 / GTW-521: the SINGLE FINAL actor-Visibility writer consults the SAME
    // ViewMode-aware drawn-band predicate the ganger spawn / move / level-filter sites use, so it
    // cannot drift from them. Wrap the borrowed active [`Level`] back into an [`ActiveLevel`] to
    // reach the shared [`ActiveLevel::draws_storey`] predicate.
    let active = crate::ActiveLevel::new(active);
    for (entity, pos, faction, life) in gangers {
        let Some(sprite) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = actors.get_mut(sprite) else {
            continue;
        };
        // GTW-520 C1/C2: the STOREY axis widened from the on-active-storey hard cut to
        // drawn-band membership — a ganger on a LOWER drawn storey passes the storey test (and,
        // if shown by fog, is drawn), while one strictly ABOVE the active level is culled. The
        // storey is the canonical CellLevel::level accessor through Position's deref (GTW-565).
        let storey = pos.level();
        let in_drawn_band = active.draws_storey(storey, view);
        let key: CellLevel = **pos;
        // GTW-520 C3: the FOG hard-cut is PRESERVED unchanged — an enemy / corpse is still shown
        // only on a squad-VISIBLE cell (a player ganger is always shown by `actor_relation`).
        // Only the STOREY axis widened; the visibility predicate did NOT.
        let shown_by_fog = is_ganger_visible(squad, &key, actor_relation(player, *faction, *life));
        *visibility = if in_drawn_band && shown_by_fog {
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
