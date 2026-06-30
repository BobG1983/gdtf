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
//! With a [`SelectedShooter`] holding an alive actor, the offered targets are:
//!
//! - **Execute** — the first [`LifeState::Downed`] ENEMY 8-adjacent ([`is_8_adjacent`]) to the
//!   actor (faction differs) — the coup-de-grâce.
//! - **Stabilize** — the first 8-adjacent downed ALLY (same faction) that is NOT already
//!   [`Stabilized`] (its bleed clock still runs) — the dressing act.
//! - **Melee** (GTW-507) — the first 8-adjacent, ALIVE, ENEMY ganger with a clear LOS
//!   ([`has_los`]) — the close-combat strike. A STRONGER gate than Execute's downed-adjacency
//!   (alive + LOS, not downed; `docs/combat/resolution.md` §7).
//!
//! The actual sim gates ([`execute_downed`](gdtf_battle_sim::execute_downed) /
//! [`stabilize_downed`](gdtf_battle_sim::stabilize_downed) /
//! [`dispatch_melee`](gdtf_battle_sim::dispatch_melee)) re-check faction + reach (+ LOS for
//! melee) authoritatively when the act fires; this layer only decides what to OFFER (and reuses
//! [`has_los`] verbatim so the melee offer matches the sim's geometry truth).

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    CoverLedger, OccupancyGrid, SurfaceGrid,
    downed_acts::is_8_adjacent,
    ganger::{Facing, Faction, LifeState, Position, Stabilized, Stance, StanceKind},
    los::{Observer, PeekOffset, Target, has_los},
    tuning::CombatTuning,
};

use crate::states::running::game::battlescape::contextual_panel::components::{
    ContextualPanelRoot, ContextualTargets, ExecuteButton, MeleeButton, OpenDoorButton,
    StabilizeButton,
};

/// The actor (selection) reads the detection scan needs — its grid cell + its gang (required,
/// for the Execute / Stabilize adjacency scan) plus its OPTIONAL stance + facing (the GTW-507
/// melee LOS observer eye).
///
/// A small named tuple alias so [`detect_contextual_targets`]'s `actors` query stays legible
/// under clippy `type_complexity`; every field is the actor's existing `Copy` ganger newtype
/// (read only, never mutated here). [`Stance`] / [`Facing`] are [`Option`] so the actor still
/// resolves for the Execute / Stabilize acts (which need neither) when they are absent; the melee
/// scan simply offers nothing without them (a real fielded ganger always carries both).
type ActorReads = (
    &'static Position,
    &'static Faction,
    Option<&'static Stance>,
    Option<&'static Facing>,
);

/// The candidate-neighbour reads the scan needs — each ganger's cell, life, gang, the optional
/// stabilized flag (Stabilize), and (for the GTW-507 melee LOS gate) its OPTIONAL stance (the LOS
/// aim silhouette). A named alias to keep [`detect_contextual_targets`]'s `candidates` query under
/// clippy `type_complexity`. [`Stance`] is [`Option`] so the Execute / Stabilize scan still sees
/// a minimal downed neighbour that carries no stance; the melee LOS scan falls back to a standing
/// silhouette when it is absent (a real fielded ganger always carries one).
type CandidateReads = (
    Entity,
    &'static Position,
    &'static LifeState,
    &'static Faction,
    Option<&'static Stabilized>,
    Option<&'static Stance>,
);

/// Query filter selecting the contextual panel ROOT's [`Visibility`] disjointly from the four
/// button markers (so the five `&mut Visibility` queries never alias) — a named alias to keep
/// [`detect_contextual_targets`]'s signature under clippy `type_complexity`.
type RootVisFilter = (
    With<ContextualPanelRoot>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Execute** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type ExecuteVisFilter = (
    With<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Stabilize** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type StabilizeVisFilter = (
    With<StabilizeButton>,
    Without<ExecuteButton>,
    Without<MeleeButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Melee** button's [`Visibility`] disjointly from the other
/// contextual markers (GTW-507) — a named alias for clippy `type_complexity`.
type MeleeVisFilter = (
    With<MeleeButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Open Door** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type OpenDoorVisFilter = (
    With<OpenDoorButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
);

/// The change-driven world grids + tuning the GTW-507 melee LOS gate reads, bundled into one
/// [`SystemParam`](bevy::ecs::system::SystemParam) so [`detect_contextual_targets`] stays under
/// clippy's argument-count gate (the sim's `BattleGridsParam` precedent).
///
/// The three grids [`has_los`] marches through (read-only — the detection layer never mutates
/// the sim) plus the [`CombatTuning`] the LOS geometry reads. `Option` reads so the system
/// stays valid before a battle inserts them (the melee scan then offers no target —
/// `bevy-traps.md` #1); in a live battle they are always present.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct LosGrids<'w> {
    /// The coarse 3D occupancy grid — the LOS march's collision / occupant-band surface (also
    /// the observer's stair-eye-offset lookup).
    occupancy: Option<Res<'w, OccupancyGrid>>,
    /// The persistent floor/roof-slab + ground surface grid the LOS march flies through.
    surface:   Option<Res<'w, SurfaceGrid>>,
    /// The model cover ledger — peeked (read only) for the LOS march's cover bands.
    cover:     Option<Res<'w, CoverLedger>>,
    /// The combat tuning the LOS view geometry reads.
    tuning:    Option<Res<'w, CombatTuning>>,
}

/// Detects the [`SelectedShooter`]'s actionable downed neighbours and drives the contextual
/// panel's show/hide reactively (GTW-294).
///
/// Resolves the selection's [`Position`] + [`Faction`] (+ its [`Stance`] / [`Facing`] for the
/// melee LOS eye), scans every ganger for an actionable neighbour, and picks: the first
/// 8-adjacent downed ENEMY as the **Execute** target; the first 8-adjacent downed,
/// not-yet-[`Stabilized`] ALLY as the **Stabilize** target; and (GTW-507) the first 8-adjacent,
/// ALIVE, ENEMY ganger with a clear LOS as the **Melee** target — a STRONGER gate than Execute's
/// downed-adjacency (alive + LOS, not downed). It then:
///
/// 1. Writes the three targets onto the [`ContextualTargets`] seam (so
///    [`contextual_button_intents`](super::intents::contextual_button_intents) can route a press
///    to the carried target).
/// 2. Sets each button's [`Visibility`](bevy::render::view::Visibility) in place — `Visible`
///    iff its target is [`Some`], else `Hidden`; the **Open Door** button stays `Hidden` (a
///    deferred act); and the panel ROOT is `Visible` iff ANY act has a target, else `Hidden`.
///
/// With NO [`SelectedShooter`] (or a selection whose entity lacks the read components) all
/// targets are cleared to [`None`] and the panel + all buttons are hidden — fail-closed, no
/// panic (`bevy-traps.md` rule: handle the `Option`, never `unwrap`). The melee LOS scan needs
/// the live battle grids; with any of them absent (a pre-battle frame) it offers NO melee
/// target (the `Option<Res<…>>` reads in [`LosGrids`] keep the system valid — `bevy-traps.md`
/// #1). Visibility is mutated IN PLACE on the existing scaffold entities — NEVER despawn/respawn
/// (the `ui-mutate-not-respawn` ruling), so the buttons keep their identity across frames.
///
/// The actual sim gate ([`dispatch_melee`](gdtf_battle_sim::dispatch_melee)) re-checks
/// adjacency + LOS + alive + opposing faction authoritatively when the act fires; this layer
/// only decides what to OFFER (and reuses [`has_los`] verbatim so the offered shot matches the
/// sim's geometry truth).
///
/// Param-only (`bevy-traps.md` #7): the [`SelectedShooter`] + [`ContextualTargets`] resources,
/// a read-only `actors` [`Query`], a read-only `candidates` [`Query`], the [`LosGrids`] bundle
/// the melee LOS gate reads, and five disjoint per-marker `Query<&mut Visibility, …>`s for the
/// root + four buttons.
#[expect(
    clippy::too_many_arguments,
    reason = "five disjoint per-marker Visibility queries (root + four buttons) are the \
              mutate-in-place idiom (ui-mutate-not-respawn); folding them into a SystemParam \
              bundle would not reduce the disjoint-query count and only adds indirection — and \
              the GTW-507 melee LOS gate adds the read-only actors/candidates/LosGrids set"
)]
pub(in crate::states::running::game::battlescape) fn detect_contextual_targets(
    selected: Res<SelectedShooter>,
    mut targets: ResMut<ContextualTargets>,
    actors: Query<ActorReads>,
    candidates: Query<CandidateReads>,
    grids: LosGrids,
    mut panel_root: Query<&mut Visibility, RootVisFilter>,
    mut execute_btn: Query<&mut Visibility, ExecuteVisFilter>,
    mut stabilize_btn: Query<&mut Visibility, StabilizeVisFilter>,
    mut melee_btn: Query<&mut Visibility, MeleeVisFilter>,
    mut open_door_btn: Query<&mut Visibility, OpenDoorVisFilter>,
) {
    // Resolve the actor: a selection holding an entity that carries the read components. Any
    // miss (no selection, or a selection lacking the reads) clears the offers + hides the panel
    // — fail-closed, no panic.
    let actor = (**selected).and_then(|entity| actors.get(entity).ok());

    let (execute, stabilize, melee) = match actor {
        Some((actor_pos, actor_faction, actor_stance, actor_facing)) => {
            let (execute, stabilize) = scan_targets(*actor_pos, *actor_faction, &candidates);
            // GTW-507 — the melee target: an 8-adjacent, ALIVE, ENEMY ganger with a clear LOS.
            // The LOS observer eye needs the actor's stance + facing; without BOTH (a minimal
            // actor that carries neither) no melee is offered — the Execute / Stabilize acts,
            // which need neither, are unaffected.
            let melee = match (actor_stance, actor_facing) {
                (Some(actor_stance), Some(actor_facing)) => scan_melee_target(
                    *actor_pos,
                    *actor_faction,
                    *actor_stance,
                    *actor_facing,
                    &candidates,
                    &grids,
                ),
                _ => None,
            };
            (execute, stabilize, melee)
        }
        None => (None, None, None),
    };

    // Write the offers onto the seam (the press router reads these).
    *targets = ContextualTargets::with(execute, stabilize, melee);

    // Toggle visibility IN PLACE — never despawn (ui-mutate-not-respawn).
    set_visibility(&mut execute_btn, execute.is_some());
    set_visibility(&mut stabilize_btn, stabilize.is_some());
    set_visibility(&mut melee_btn, melee.is_some());
    // Open Door is a deferred act — it stays hidden regardless of detection.
    set_visibility(&mut open_door_btn, false);
    // The panel shows iff at least one contextual act is offered.
    set_visibility(
        &mut panel_root,
        execute.is_some() || stabilize.is_some() || melee.is_some(),
    );
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
    candidates: &Query<CandidateReads>,
) -> (Option<Entity>, Option<Entity>) {
    let mut execute = None;
    let mut stabilize = None;

    for (entity, pos, life, faction, stabilized, _stance) in candidates {
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

/// Scans `candidates` for the actor's actionable MELEE target — the first 8-adjacent, ALIVE,
/// ENEMY ganger with a clear line of sight from the actor (GTW-507; `docs/combat/resolution.md`
/// §7).
///
/// A STRONGER offer gate than Execute's downed-adjacency: the target must be ALIVE (incl.
/// Downed — [`LifeState::is_active`]), of the OPPOSING faction, 8-adjacent ([`is_8_adjacent`]),
/// AND in clear LOS ([`has_los`], REUSED verbatim so the offered shot matches the sim's geometry
/// truth). Returns the first such candidate, or [`None`] when none qualifies — including when
/// the battle grids are absent (a pre-battle frame), so the melee button stays hidden until a
/// live battle. A corpse never blocks the LOS march (the `is_dead` pass-through reused).
///
/// Pure over the queried components + the read grids (no world mutation), so it is testable in
/// isolation. The sim's [`dispatch_melee`](gdtf_battle_sim::dispatch_melee) gate is the
/// authoritative re-check when the act actually fires; this only decides what to OFFER.
fn scan_melee_target(
    actor_pos: Position,
    actor_faction: Faction,
    actor_stance: Stance,
    actor_facing: Facing,
    candidates: &Query<CandidateReads>,
    grids: &LosGrids,
) -> Option<Entity> {
    // The melee LOS gate needs the live battle grids; with any absent (pre-battle) offer nothing
    // — fail-closed (bevy-traps.md #1: handle the Option, never unwrap).
    let (Some(occupancy), Some(surface), Some(cover), Some(tuning)) = (
        grids.occupancy.as_ref(),
        grids.surface.as_ref(),
        grids.cover.as_ref(),
        grids.tuning.as_ref(),
    ) else {
        return None;
    };

    // A Dead ganger is a corpse the LOS march flies THROUGH (the same `is_dead` pass-through
    // `has_los` takes). Read each candidate's CURRENT LifeState; an absent entity is no corpse.
    let is_dead = |entity: Entity| {
        candidates
            .get(entity)
            .is_ok_and(|(_, _, life, ..)| *life == LifeState::Dead)
    };

    // The fallback silhouette for a candidate carrying no stance (a real fielded ganger always
    // has one; this keeps the LOS aim defined for a minimal test/edge entity).
    let standing = Stance::new(StanceKind::Standing);
    for (entity, pos, life, faction, _stabilized, stance) in candidates {
        // An ALIVE (incl. Downed) OPPOSING ganger within the 8-adjacent reach.
        if !life.is_active() || *faction == actor_faction || !is_8_adjacent(actor_pos, *pos) {
            continue;
        }
        // The LOS gate — a clear sight line actor → target over the SAME voxel geometry the sim
        // fires through (built exactly as the sim's `dispatch_melee` builds the observer/target).
        let observer = Observer {
            position:         &actor_pos,
            stance:           &actor_stance,
            facing:           &actor_facing,
            stair_eye_offset: occupancy.stair_eye_offset_at(&actor_pos),
            peek_offset:      PeekOffset::default(),
        };
        let target = Target {
            position: pos,
            stance:   stance.unwrap_or(&standing),
        };
        if *has_los(
            &observer, &target, occupancy, surface, cover, tuning, is_dead,
        ) {
            return Some(entity);
        }
    }
    None
}

/// Sets the single matched [`Visibility`] to `Visible` (when `show`) or `Hidden`, in place.
///
/// A small helper so each of the five disjoint per-marker queries toggles its node identically
/// without spelling the `Visible`/`Hidden` branch five times. Mutates the existing component —
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
