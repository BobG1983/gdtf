//! The ganger-sprite visibility RESOLVER (GTW-627): the ONE system holding
//! `Query<&mut Visibility, With<GangerSprite>>` in the crate, fed by a single pure
//! band × fog classifier.
//!
//! Before GTW-627 four sites wrote a ganger sprite's [`Visibility`] (the spawn seed, the
//! move flip, the on-change level filter, and the fog writer's actor arm), held consistent
//! by a shared predicate and ordering conventions. Now the decision lives in ONE pure
//! classifier ([`classify_ganger_visibility`]) and the component has ONE writer
//! ([`resolve_ganger_visibility`]); the spawn seeds its initial value through the same
//! classifier (via [`GangerVisibilityFacts::classify`]), so a second writer is
//! unrepresentable rather than conventioned.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{CellLevel, Faction, LifeState, Position},
    visibility::{FactionRelation, SquadVisibility, is_ganger_visible},
};

use super::sprite_map::{GangerSprite, GangerSprites};
use crate::{
    ActiveLevel, IsolateView, StoreyViewMode, ViewMode, actors::quiet::set_visibility_quiet,
};

/// The fog-side facts the classifier composes when the fog resources are RESIDENT.
///
/// Bundles the sim's [`SquadVisibility`] sets with the battle's optional
/// [`PlayerFaction`]: the squad sets decide WHAT is seen, the player faction decides which
/// gangers are trivially visible ([`FactionRelation::OwnSquad`]). `player` stays [`None`]
/// in a harness whose setup seeded no player faction — then every ganger is fog-gated
/// ([`FactionRelation::Other`], fail-closed), exactly the pre-GTW-627 actor-arm behaviour.
pub(super) struct GangerFogFacts<'a> {
    /// The squad's VISIBLE / EXPLORED sets — the sim-owned fog truth.
    squad:  &'a SquadVisibility,
    /// The player-controlled faction, when a battle seeded one.
    player: Option<PlayerFaction>,
}

impl<'a> GangerFogFacts<'a> {
    /// Bundle the resident fog facts for one classification pass.
    pub(super) const fn new(squad: &'a SquadVisibility, player: Option<PlayerFaction>) -> Self {
        Self { squad, player }
    }
}

/// The squad-fog [`FactionRelation`] a ganger sprite is gated by.
///
/// A live ([`Alive`](LifeState::Alive)) PLAYER-faction ganger is
/// [`FactionRelation::OwnSquad`] (always shown); everything else — an ENEMY ganger, or a
/// CORPSE of either faction (a downed/dead body is no longer a member of the seeing
/// squad) — is [`FactionRelation::Other`], shown only on a squad-VISIBLE cell. `player`
/// is [`None`] when no [`PlayerFaction`] is resident, in which case every ganger is
/// treated as [`FactionRelation::Other`] (fog-gated) — fail-closed.
pub(super) fn actor_relation(
    player: Option<PlayerFaction>,
    faction: Faction,
    life: LifeState,
) -> FactionRelation {
    let is_player = player.is_some_and(|p| *p == faction);
    if is_player && *life.is_active() {
        FactionRelation::OwnSquad
    } else {
        FactionRelation::Other
    }
}

/// The ONE pure ganger-visibility classifier (GTW-627 C1): compose the drawn-band storey
/// fact with the fog fact into the sprite's [`Visibility`].
///
/// * **Band fact** (always) — the shared
///   [`ActiveLevel::draws_storey`](crate::ActiveLevel::draws_storey) predicate under the
///   composed [`StoreyViewMode`] (GTW-520 drawn-band membership / GTW-521 full-view
///   ceiling / the GTW-594 Isolate band, all derived from the one storey-treatment
///   classifier): a ganger on any drawn storey passes; one on a hidden storey is culled.
/// * **Fog fact** (only when `fog` carries the RESIDENT fog resources) — the untouched
///   GTW-342 hard-cut: a live player ganger is trivially visible
///   ([`FactionRelation::OwnSquad`]); an enemy or a corpse is shown iff its cell is
///   squad-VISIBLE ([`is_ganger_visible`]) — no fade, no last-known ghost. With `fog`
///   [`None`] (the fog sets not resident — a focused harness) the classifier is BAND-ONLY,
///   preserving the fog-inert harness behaviour exactly: the absent-fog branch IS the
///   band-only mode, not a mode flag.
///
/// Shown ⇔ band AND fog: [`Visibility::Inherited`] when both facts pass,
/// [`Visibility::Hidden`] otherwise.
pub(super) fn classify_ganger_visibility(
    pos: &Position,
    faction: Faction,
    life: LifeState,
    active: ActiveLevel,
    mode: StoreyViewMode,
    fog: Option<&GangerFogFacts<'_>>,
) -> Visibility {
    // The storey axis: the canonical CellLevel::level accessor through Position's deref
    // (GTW-565) against the ONE shared band predicate.
    let in_drawn_band = active.draws_storey(pos.level(), mode);
    // The fog axis: only when the fog resources are resident; band-only when absent.
    let shown_by_fog = fog.is_none_or(|facts| {
        let key: CellLevel = **pos;
        *is_ganger_visible(
            facts.squad,
            &key,
            actor_relation(facts.player, faction, life),
        )
    });
    if in_drawn_band && shown_by_fog {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

/// The classifier's resource inputs, bundled once for the resolver AND the spawn seed
/// (GTW-627 C3) — so both sites reach the one pure classifier
/// (`classify_ganger_visibility`) through the same facts and neither re-derives a
/// compose decision.
#[derive(SystemParam)]
pub struct GangerVisibilityFacts<'w> {
    /// The presenter-owned active view storey — the band fact's ceiling input.
    active:  Res<'w, ActiveLevel>,
    /// The presenter-owned view mode — chooses the band ceiling rule (GTW-521).
    view:    Res<'w, ViewMode>,
    /// The presenter-owned Isolate toggle (GTW-594) — wins over the two-state mode when
    /// on; composed with `view` into the classifier's [`StoreyViewMode`] input.
    isolate: Res<'w, IsolateView>,
    /// The sim's squad fog sets — the fog RESIDENCY witness (`bevy-traps.md` #1): absent
    /// (a focused harness with no battle fog) puts the classifier in band-only mode.
    squad:   Option<Res<'w, SquadVisibility>>,
    /// The battle's player faction; optional INSIDE the fog fact (fail-closed to
    /// [`FactionRelation::Other`] when absent while the squad sets are resident).
    player:  Option<Res<'w, PlayerFaction>>,
}

impl GangerVisibilityFacts<'_> {
    /// Classify one ganger through the pure classifier under the currently-resident facts.
    pub(super) fn classify(&self, pos: &Position, faction: Faction, life: LifeState) -> Visibility {
        let fog = self
            .squad
            .as_deref()
            .map(|squad| GangerFogFacts::new(squad, self.player.as_deref().copied()));
        classify_ganger_visibility(
            pos,
            faction,
            life,
            *self.active,
            StoreyViewMode::new(*self.view, *self.isolate),
            fog.as_ref(),
        )
    }
}

/// `Update` ([`PresenterSystems::Compose`](crate::PresenterSystems), GTW-627): the ONE
/// writer of every ganger sprite's [`Visibility`] — resolve each mapped sprite through the
/// pure classifier every frame.
///
/// Runs in the `Compose` stage, chained strictly after the `Scene` stage that spawns /
/// moves the sprites (GTW-623 stage membership — no pairwise `.after` edges), so it always
/// resolves against the frame's settled sim state. For each live sim ganger it looks the
/// presenter sprite up through [`GangerSprites`] and writes the classifier's verdict via
/// the shared tick-quiet seam (`set_if_neq` — an unchanged sprite's change ticks stay
/// untouched, GTW-627 C3). A not-yet-materialized sprite (the deferred `spawn_scene`,
/// GTW-322) is skipped and picked up the frame its components exist; its spawn-seeded
/// value came through the same classifier, so there is no first-frame flicker.
///
/// Param-only (`bevy-traps.md` #7): [`Res<GangerSprites>`], the bundled
/// [`GangerVisibilityFacts`], the sim-ganger query, and the ONE
/// `Query<&mut Visibility, With<GangerSprite>>` in the crate (GTW-627 A1).
pub fn resolve_ganger_visibility(
    sprites: Res<GangerSprites>,
    facts: GangerVisibilityFacts,
    gangers: Query<(Entity, &Position, &Faction, &LifeState)>,
    mut actors: Query<&mut Visibility, With<GangerSprite>>,
) {
    for (entity, pos, faction, life) in &gangers {
        let Some(sprite) = sprites.sprite_for(entity) else {
            continue;
        };
        let Ok(mut visibility) = actors.get_mut(sprite) else {
            continue;
        };
        set_visibility_quiet(&mut visibility, facts.classify(pos, *faction, *life));
    }
}
