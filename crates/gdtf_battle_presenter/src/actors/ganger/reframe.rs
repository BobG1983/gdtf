//! In-place atlas-index / tint refresh: the state-change reframe and the
//! hot-reload re-index.

use bevy::{ecs::lifecycle::RemovedComponents, prelude::*};
use gdtf_battle_sim::{
    ganger::{Aiming, Facing, Suppressed},
    prelude::{Faction, LifeState, Stance},
};

use super::{
    frame::atlas_index,
    roles::CharacterRoles,
    sprite_map::{GangerSprite, GangerSprites},
    tint::stance_aiming_tint,
};

/// The read-only ganger fields the reframe/re-tint read — the [`QueryData`] tuple,
/// factored out to keep the [`reframe_ganger_sprites`] query under the
/// `type_complexity` clippy gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type ReframeData = (
    Entity,
    &'static Faction,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    &'static LifeState,
    Option<&'static Suppressed>,
);

/// The "any of facing / stance / aiming / suppressed changed" [`QueryFilter`] driving the
/// reframe/re-tint, factored out for the same `type_complexity` reason as [`ReframeData`].
///
/// [`Changed<Suppressed>`](Suppressed) catches suppression being APPLIED (the sim inserts the
/// component — GTW-526 C2); a suppressed ganger's auto-stance drop (C5) ALSO trips
/// [`Changed<Stance>`], so the tint is doubly guaranteed to refresh on application. Suppression
/// being CLEARED is a component REMOVAL (the sim `remove`s it — C6), which `Changed` does NOT
/// observe, so [`reframe_ganger_sprites`] additionally drains
/// [`RemovedComponents<Suppressed>`](RemovedComponents) to un-tint a no-longer-suppressed ganger.
///
/// [`QueryFilter`]: bevy::ecs::query::QueryFilter
type ReframeChanged = Or<(
    Changed<Facing>,
    Changed<Stance>,
    Changed<Aiming>,
    Changed<Suppressed>,
)>;

/// `Update` (`PresenterSystems::Scene`): reframe / re-tint a ganger sprite whose
/// [`Facing`], [`Stance`], [`Aiming`], or [`Suppressed`] state changed.
///
/// For every ganger whose [`Facing`] / [`Stance`] / [`Aiming`] / [`Suppressed`] is [`Changed`],
/// look the presenter sprite up through [`GangerSprites`] and recompute its texture-atlas index
/// (facing reframe via the 8->4 map) and re-tint it (the stance / aiming / suppressed delta) in
/// place. The reframe always recomputes from the CURRENT facing; the aiming delta
/// brightens the sprite (an aimed ganger reads "ready"); the stance delta dims a prone
/// ganger (a flattened silhouette); a [`Suppressed`] ganger is desaturated + darkened
/// (GTW-526 C8 — it reads distinctly "pinned"). A [`Dead`](LifeState::Dead) ganger's sprite is
/// already despawned, so its lookup misses and is skipped.
///
/// Suppression being CLEARED is a component REMOVAL, which `Changed` does not observe, so this
/// system ALSO drains [`RemovedComponents<Suppressed>`](RemovedComponents) and re-tints each
/// just-cleared ganger — its (now `Suppressed`-less) [`ReframeData`] is read back through the
/// full-set `all` query, so `stance_aiming_tint` sees `suppressed = false` and the ganger
/// returns to its ordinary faction tint.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], the changed-state ganger query, the
/// full-set ganger query (for the removal re-read), the [`RemovedComponents<Suppressed>`], and
/// the presenter-sprite [`Sprite`] query.
pub fn reframe_ganger_sprites(
    sprites: Res<GangerSprites>,
    roles: Res<CharacterRoles>,
    changed: Query<ReframeData, ReframeChanged>,
    all: Query<ReframeData>,
    mut removed: RemovedComponents<Suppressed>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    // The APPLIED / stance / aim / facing path: every ganger whose reframe state Changed.
    for data in &changed {
        reframe_one(&sprites, &roles, &mut presenters, data);
    }
    // The CLEARED path: a suppression REMOVAL is not a `Changed`, so drain the removals and
    // re-tint each just-cleared ganger from its full-set data (now carrying no `Suppressed`,
    // so the tint drops back to the ordinary faction colour). A ganger whose sprite is already
    // gone (Dead) or whose entity despawned is skipped by the `all.get` / `sprite_for` misses.
    for entity in removed.read() {
        if let Ok(data) = all.get(entity) {
            reframe_one(&sprites, &roles, &mut presenters, data);
        }
    }
}

/// Reframe + re-tint one ganger's presenter sprite from its [`ReframeData`] — the shared body
/// of both [`reframe_ganger_sprites`] paths (the `Changed` loop and the suppression-removal
/// loop).
///
/// Looks the presenter sprite up through [`GangerSprites`], recomputes its facing-correct
/// atlas index, and re-tints it via [`stance_aiming_tint`] (the faction / life base modulated
/// by stance, aim, and the SUPPRESSED flag, which is `true` iff the ganger currently carries a
/// [`Suppressed`] component). A ganger with no mapped / present sprite (a Dead ganger's is
/// despawned) is skipped.
fn reframe_one(
    sprites: &GangerSprites,
    roles: &CharacterRoles,
    presenters: &mut Query<&mut Sprite, With<GangerSprite>>,
    data: (
        Entity,
        &Faction,
        &Facing,
        &Stance,
        &Aiming,
        &LifeState,
        Option<&Suppressed>,
    ),
) {
    let (entity, faction, facing, stance, aiming, life, suppressed) = data;
    let Some(presenter) = sprites.sprite_for(entity) else {
        return;
    };
    let Ok(mut sprite) = presenters.get_mut(presenter) else {
        return;
    };
    // Reframe to the facing-correct frame.
    if let Some(atlas) = sprite.texture_atlas.as_mut() {
        atlas.index = atlas_index(roles, *faction, *facing);
    }
    // Re-tint: the faction/life base, modulated by the stance + aiming + suppressed delta.
    sprite.color = stance_aiming_tint(*faction, *life, *stance, *aiming, suppressed.is_some());
}

/// `Update` (`PresenterSystems::Scene`, runs only on a [`CharacterRoles`] change):
/// RE-INDEX every mapped ganger sprite to the freshly-reloaded atlas indices (GTW-375 C4).
///
/// When the GTW-375 hot-reload (now the GTW-564 generic hot-RON redrive of the
/// [`CharacterRoles`] chain) re-resolves `character_roles.ron` it overwrites the resident
/// [`CharacterRoles`] resource (marking it changed). The atlas index a ganger draws is `faction_base + facing_frame`
/// ([`atlas_index`]) — the `faction_base` term is read STRUCTURALLY from [`CharacterRoles`],
/// so a re-saved table moves a faction's whole 4-frame actor run to a new base. The
/// per-field draw systems ([`spawn_ganger_sprites`](super::spawn_move::spawn_ganger_sprites) /
/// [`reframe_ganger_sprites`]) only
/// re-read that base on an `Added` / `Changed` sim event, NOT on a resource change — so
/// already-spawned, idle sprites would keep their OLD index without this system.
///
/// On `CharacterRoles::is_changed` (the resource-change witness, including the one-time
/// initial resolve, which is harmless — it re-stamps the same index) it walks every live
/// ganger, looks its presenter sprite up through [`GangerSprites`], and re-stamps
/// `atlas.index = atlas_index(&roles, faction, facing)` from the CURRENT sim state — exactly
/// the [`reframe_ganger_sprites`] index recompute, but driven by the table change rather than
/// a facing change. It RE-INDEXES ONLY — it does NOT re-tint (the life / stance / aiming tint
/// is owned by [`update_ganger_life_state`](super::death::update_ganger_life_state) /
/// [`reframe_ganger_sprites`] and must be preserved
/// across a character-sheet hot-reload).
///
/// Idempotent + battle-witness-free harmless: a re-index on an unchanged base re-stamps the
/// same value, so it is safe to run pre-battle / on the initial resolve (the Discovery's
/// "no battle-witness gate needed" note); the renderer registration gates it on
/// [`CharacterRoles`] existing (`bevy-traps.md` #1) so a `MinimalPlugins` headless app
/// without the table never runs it, and on `CharacterRoles::is_changed` so it does no
/// per-frame work. A [`Dead`](LifeState::Dead) ganger's sprite is already despawned, so its
/// lookup misses and is skipped.
///
/// Param-only (`bevy-traps.md` #7): [`Res<CharacterRoles>`], [`Res<GangerSprites>`], the live
/// ganger `(Entity, &Faction, &Facing)` query, and the presenter-sprite `Sprite` query.
pub fn reindex_ganger_sprites_on_character_roles_change(
    roles: Res<CharacterRoles>,
    sprites: Res<GangerSprites>,
    gangers: Query<(Entity, &Faction, &Facing)>,
    mut presenters: Query<&mut Sprite, With<GangerSprite>>,
) {
    for (entity, faction, facing) in &gangers {
        let Some(presenter) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut sprite) = presenters.get_mut(presenter) else {
            continue;
        };
        // RE-INDEX ONLY — preserve the existing tint (do NOT touch `sprite.color`).
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = atlas_index(&roles, *faction, *facing);
        }
    }
}
