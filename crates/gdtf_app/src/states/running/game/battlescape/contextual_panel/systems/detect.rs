//! Detects the actionable downed neighbours of the [`SelectedShooter`] and drives the
//! contextual panel's reactive show/hide (GTW-294 live slice).
//!
//! [`detect_contextual_targets`] runs every `Update` (gated on the live-battle witness
//! `BattleInProgress` by [`ContextualPanelPlugin`](super::super::plugin::ContextualPanelPlugin))
//! and is the panel's BRAIN: it reads the current selection, scans the gangers for a downed
//! neighbour each contextual act could target, writes the result onto the
//! [`ContextualTargets`] seam, and toggles the panel root + each button's
//! [`Visibility`](bevy::render::view::Visibility) IN PLACE — never despawning the scaffold (the
//! buttons keep their identity so a later `OnExit` despawn is the only teardown).
//!
//! ## What is "actionable"
//!
//! With a [`SelectedShooter`] holding an alive actor, a candidate downed neighbour is any
//! [`LifeState::Downed`] ganger 8-adjacent ([`is_8_adjacent`]) to the actor:
//!
//! - **Execute** targets the first downed ENEMY (faction differs from the actor's) — the
//!   coup-de-grâce.
//! - **Stabilize** targets the first downed ALLY (faction equals the actor's) that is NOT
//!   already [`Stabilized`] (its bleed clock still runs) — the dressing act.
//!
//! The actual sim gates ([`execute_downed`](gdtf_battle_sim::execute_downed) /
//! [`stabilize_downed`](gdtf_battle_sim::stabilize_downed)) re-check faction + reach
//! authoritatively when the act fires; this layer only decides what to OFFER.

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    downed_acts::is_8_adjacent,
    ganger::{Faction, LifeState, Position, Stabilized},
};

use crate::states::running::game::battlescape::contextual_panel::components::{
    ContextualPanelRoot, ContextualTargets, ExecuteButton, OpenDoorButton, StabilizeButton,
};

/// The actor (selection) reads the detection scan needs — its grid cell + its gang.
///
/// A small named tuple alias so [`detect_contextual_targets`]'s `actors` query stays legible
/// under clippy `type_complexity`; both are the actor's existing `Copy` ganger newtypes (read
/// only, never mutated here).
type ActorReads = (&'static Position, &'static Faction);

/// Query filter selecting the contextual panel ROOT's [`Visibility`] disjointly from the three
/// button markers (so the four `&mut Visibility` queries never alias) — a named alias to keep
/// [`detect_contextual_targets`]'s signature under clippy `type_complexity`.
type RootVisFilter = (
    With<ContextualPanelRoot>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Execute** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type ExecuteVisFilter = (
    With<ExecuteButton>,
    Without<StabilizeButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Stabilize** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type StabilizeVisFilter = (
    With<StabilizeButton>,
    Without<ExecuteButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Open Door** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type OpenDoorVisFilter = (
    With<OpenDoorButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
);

/// Detects the [`SelectedShooter`]'s actionable downed neighbours and drives the contextual
/// panel's show/hide reactively (GTW-294).
///
/// Resolves the selection's [`Position`] + [`Faction`], scans every ganger for a
/// [`LifeState::Downed`] neighbour 8-adjacent ([`is_8_adjacent`]) to it, and picks the first
/// downed ENEMY (different faction) as the **Execute** target and the first downed,
/// not-yet-[`Stabilized`] ALLY (same faction) as the **Stabilize** target. It then:
///
/// 1. Writes the two targets onto the [`ContextualTargets`] seam (so
///    [`contextual_button_intents`](super::intents::contextual_button_intents) can route a press
///    to the carried target).
/// 2. Sets each button's [`Visibility`](bevy::render::view::Visibility) in place — `Visible`
///    iff its target is [`Some`], else `Hidden`; the **Open Door** button stays `Hidden` (a
///    deferred act); and the panel ROOT is `Visible` iff EITHER act has a target, else `Hidden`.
///
/// With NO [`SelectedShooter`] (or a selection whose entity lacks the read components) both
/// targets are cleared to [`None`] and the panel + all buttons are hidden — fail-closed, no
/// panic (`bevy-traps.md` rule: handle the `Option`, never `unwrap`). Visibility is mutated IN
/// PLACE on the existing scaffold entities — NEVER despawn/respawn (the
/// `ui-mutate-not-respawn` ruling), so the buttons keep their identity across frames.
///
/// Param-only (`bevy-traps.md` #7): the [`SelectedShooter`] + [`ContextualTargets`] resources,
/// a read-only `actors` [`Query`], a read-only `candidates` [`Query`] over every ganger's
/// `(Entity, Position, LifeState, Faction, Option<Stabilized>)`, and four disjoint per-marker
/// `Query<&mut Visibility, …>`s for the root + three buttons.
#[expect(
    clippy::too_many_arguments,
    reason = "four disjoint per-marker Visibility queries (root + three buttons) are the \
              mutate-in-place idiom (ui-mutate-not-respawn); folding them into a SystemParam \
              bundle would not reduce the disjoint-query count and only adds indirection"
)]
pub(in crate::states::running::game::battlescape) fn detect_contextual_targets(
    selected: Res<SelectedShooter>,
    mut targets: ResMut<ContextualTargets>,
    actors: Query<ActorReads>,
    candidates: Query<(Entity, &Position, &LifeState, &Faction, Option<&Stabilized>)>,
    mut panel_root: Query<&mut Visibility, RootVisFilter>,
    mut execute_btn: Query<&mut Visibility, ExecuteVisFilter>,
    mut stabilize_btn: Query<&mut Visibility, StabilizeVisFilter>,
    mut open_door_btn: Query<&mut Visibility, OpenDoorVisFilter>,
) {
    // Resolve the actor: a selection holding an entity that carries the read components. Any
    // miss (no selection, or a selection lacking Position/Faction) clears the offers + hides
    // the panel — fail-closed, no panic.
    let actor = (**selected).and_then(|entity| actors.get(entity).ok());

    let (execute, stabilize) = match actor {
        Some((actor_pos, actor_faction)) => scan_targets(*actor_pos, *actor_faction, &candidates),
        None => (None, None),
    };

    // Write the offers onto the seam (the press router reads these).
    *targets = ContextualTargets::with(execute, stabilize);

    // Toggle visibility IN PLACE — never despawn (ui-mutate-not-respawn).
    set_visibility(&mut execute_btn, execute.is_some());
    set_visibility(&mut stabilize_btn, stabilize.is_some());
    // Open Door is a deferred act — it stays hidden regardless of detection.
    set_visibility(&mut open_door_btn, false);
    // The panel shows iff at least one contextual act is offered.
    set_visibility(&mut panel_root, execute.is_some() || stabilize.is_some());
}

/// Scans `candidates` for the actor's actionable downed neighbours.
///
/// Returns `(execute, stabilize)`: the first 8-adjacent downed ENEMY (different faction) and
/// the first 8-adjacent downed ALLY (same faction) that is not already [`Stabilized`]. Pure
/// over the queried components (no world mutation) so the offer logic is testable in isolation
/// from the visibility toggling.
fn scan_targets(
    actor_pos: Position,
    actor_faction: Faction,
    candidates: &Query<(Entity, &Position, &LifeState, &Faction, Option<&Stabilized>)>,
) -> (Option<Entity>, Option<Entity>) {
    let mut execute = None;
    let mut stabilize = None;

    for (entity, pos, life, faction, stabilized) in candidates {
        // Only DOWNED neighbours within the 8-adjacent reach are candidates.
        if *life != LifeState::Downed || !is_8_adjacent(actor_pos, *pos) {
            continue;
        }
        if *faction == actor_faction {
            // A downed ALLY — stabilize, unless its bleed clock is already halted.
            let already_stabilized = stabilized.is_some_and(|flag| **flag);
            if stabilize.is_none() && !already_stabilized {
                stabilize = Some(entity);
            }
        } else if execute.is_none() {
            // A downed ENEMY — execute.
            execute = Some(entity);
        }
    }

    (execute, stabilize)
}

/// Sets the single matched [`Visibility`] to `Visible` (when `show`) or `Hidden`, in place.
///
/// A small helper so each of the four disjoint per-marker queries toggles its node identically
/// without spelling the `Visible`/`Hidden` branch four times. Mutates the existing component —
/// it never spawns or despawns (the `ui-mutate-not-respawn` ruling). A query that matches no
/// node (the panel not yet spawned) is a silent no-op.
fn set_visibility<F: bevy::ecs::query::QueryFilter>(
    query: &mut Query<&mut Visibility, F>,
    show: bool,
) {
    let want = if show {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in query {
        // Change-detection hygiene: only write on a real change so an unchanged node does not
        // spuriously trip `Changed<Visibility>`.
        if *visibility != want {
            *visibility = want;
        }
    }
}
