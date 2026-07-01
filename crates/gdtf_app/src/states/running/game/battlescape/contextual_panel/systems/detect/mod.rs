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
//! - **Execute** — the first [`LifeState::Downed`] ENEMY 8-adjacent
//!   ([`is_8_adjacent`](gdtf_battle_sim::downed_acts::is_8_adjacent)) to the actor (faction
//!   differs) — the coup-de-grâce.
//! - **Stabilize** — the first 8-adjacent downed ALLY (same faction) that is NOT already
//!   [`Stabilized`] (its bleed clock still runs) — the dressing act.
//! - **Melee** (GTW-507) — the first 8-adjacent, ALIVE, ENEMY ganger with a clear LOS
//!   ([`has_los`](gdtf_battle_sim::los::has_los)) — the close-combat strike. A STRONGER gate
//!   than Execute's downed-adjacency (alive + LOS, not downed; `docs/combat/resolution.md` §7).
//! - **Melee — cover-smash** (GTW-508) — with NO meleeable ganger in reach, the first 8-adjacent
//!   intact Cover / Wall cell, so the ONE Melee button offers either a ganger strike or a
//!   cover-smash (never both). Its `(cell, level)` rides the seam's melee-structure slot.
//! - **Shove** (GTW-525) — the first 8-adjacent, ALIVE, OPPOSING ganger — the deliberate
//!   knock-back. A WEAKER gate than Melee's: NO LOS required (a shove is contact) and NO weapon
//!   required (any ganger can shove).
//! - **Open Door** (GTW-315) — the first 8-adjacent openable terrain entity in the
//!   [`OpenState::Closed`](gdtf_battle_sim::OpenState) state — the deliberate open act. The button
//!   always OPENS (an already-open door is not offered; closing is not a contextual act), and F4 is
//!   PLAYER-ONLY (this runs only for a selected player-faction actor).
//!
//! The offer scans live in the [`scan`] submodule (GTW-508 C6 — code-health size cap); this
//! file (`mod.rs`) owns the [`detect_contextual_targets`] system, its query `type` aliases, and
//! the [`LosGrids`] bundle.
//!
//! The actual sim gates ([`execute_downed`](gdtf_battle_sim::execute_downed) /
//! [`stabilize_downed`](gdtf_battle_sim::stabilize_downed) /
//! [`dispatch_melee`](gdtf_battle_sim::dispatch_melee)) re-check faction + reach (+ LOS for
//! melee) authoritatively when the act fires; this layer only decides what to OFFER (and reuses
//! [`has_los`](gdtf_battle_sim::los::has_los) verbatim so the melee offer matches the sim's
//! geometry truth).

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    CoverLedger, OccupancyGrid, OpenState, SurfaceGrid,
    entity::TerrainCell,
    ganger::{Facing, Faction, LifeState, Position, Stabilized, Stance},
    tuning::CombatTuning,
};

use crate::states::running::game::battlescape::contextual_panel::components::{
    ContextualPanelRoot, ContextualTargets, ExecuteButton, MeleeButton, OpenDoorButton,
    ShoveButton, StabilizeButton,
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

/// The candidate-DOOR reads the GTW-315 open-door scan needs — each openable terrain entity's
/// handle, its [`OpenState`] (the scan offers only a CLOSED door), and its [`TerrainCell`] (the
/// cell the actor must be 8-adjacent to). A named alias to keep
/// [`detect_contextual_targets`]'s `doors` query legible under clippy `type_complexity`.
///
/// A DOOR is any terrain entity carrying an [`OpenState`] — the GTW-503 openable mechanism attaches
/// it only to openable pieces (doors / hatches) at spawn, so `With<OpenState>` selects exactly the
/// openable terrain and never a ganger (a ganger has no `OpenState`), keeping this query disjoint
/// from the ganger `candidates` / `actors` queries.
type DoorReads = (Entity, &'static OpenState, &'static TerrainCell);

/// Query filter selecting the contextual panel ROOT's [`Visibility`] disjointly from the five
/// button markers (so the six `&mut Visibility` queries never alias) — a named alias to keep
/// [`detect_contextual_targets`]'s signature under clippy `type_complexity`.
type RootVisFilter = (
    With<ContextualPanelRoot>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Execute** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type ExecuteVisFilter = (
    With<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Stabilize** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type StabilizeVisFilter = (
    With<StabilizeButton>,
    Without<ExecuteButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Melee** button's [`Visibility`] disjointly from the other
/// contextual markers (GTW-507) — a named alias for clippy `type_complexity`.
type MeleeVisFilter = (
    With<MeleeButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Shove** button's [`Visibility`] disjointly from the other
/// contextual markers (GTW-525) — a named alias for clippy `type_complexity`.
type ShoveVisFilter = (
    With<ShoveButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<OpenDoorButton>,
);

/// Query filter selecting the **Open Door** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type OpenDoorVisFilter = (
    With<OpenDoorButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
);

/// The change-driven world grids + tuning the GTW-507 melee LOS gate reads, bundled into one
/// [`SystemParam`](bevy::ecs::system::SystemParam) so [`detect_contextual_targets`] stays under
/// clippy's argument-count gate (the sim's `BattleGridsParam` precedent).
///
/// The three grids [`has_los`](gdtf_battle_sim::los::has_los) marches through (read-only — the
/// detection layer never mutates the sim) plus the [`CombatTuning`] the LOS geometry reads.
/// `Option` reads so the system
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
/// 1. Writes the targets onto the [`ContextualTargets`] seam (so
///    [`contextual_button_intents`](super::intents::contextual_button_intents) can route a press
///    to the carried target).
/// 2. Sets each button's [`Visibility`](bevy::render::view::Visibility) in place — `Visible`
///    iff its target is [`Some`], else `Hidden`; the **Shove** button (GTW-525) reveals on an
///    8-adjacent alive opposing ganger (no LOS / weapon needed); the **Open Door** button
///    (GTW-315) reveals on an 8-adjacent CLOSED door; and the panel ROOT is `Visible` iff ANY act
///    has a target, else `Hidden`.
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
/// only decides what to OFFER (and reuses [`has_los`](gdtf_battle_sim::los::has_los) verbatim so
/// the offered shot matches the sim's geometry truth).
///
/// Param-only (`bevy-traps.md` #7): the [`SelectedShooter`] + [`ContextualTargets`] resources,
/// a read-only `actors` [`Query`], a read-only `candidates` [`Query`], a read-only `doors`
/// [`Query`] (the GTW-315 open-door scan — disjoint from the ganger queries via `With<OpenState>`),
/// the [`LosGrids`] bundle the melee LOS gate reads, and six disjoint per-marker
/// `Query<&mut Visibility, …>`s for the root + five buttons.
#[expect(
    clippy::too_many_arguments,
    reason = "six disjoint per-marker Visibility queries (root + five buttons) are the \
              mutate-in-place idiom (ui-mutate-not-respawn); folding them into a SystemParam \
              bundle would not reduce the disjoint-query count and only adds indirection — and \
              the GTW-507 melee LOS gate adds the read-only actors/candidates/LosGrids set"
)]
pub(in crate::states::running::game::battlescape) fn detect_contextual_targets(
    selected: Res<SelectedShooter>,
    mut targets: ResMut<ContextualTargets>,
    actors: Query<ActorReads>,
    candidates: Query<CandidateReads>,
    doors: Query<DoorReads>,
    grids: LosGrids,
    mut panel_root: Query<&mut Visibility, RootVisFilter>,
    mut execute_btn: Query<&mut Visibility, ExecuteVisFilter>,
    mut stabilize_btn: Query<&mut Visibility, StabilizeVisFilter>,
    mut melee_btn: Query<&mut Visibility, MeleeVisFilter>,
    mut shove_btn: Query<&mut Visibility, ShoveVisFilter>,
    mut open_door_btn: Query<&mut Visibility, OpenDoorVisFilter>,
) {
    // Resolve the actor: a selection holding an entity that carries the read components. Any
    // miss (no selection, or a selection lacking the reads) clears the offers + hides the panel
    // — fail-closed, no panic.
    let actor = (**selected).and_then(|entity| actors.get(entity).ok());

    let (execute, stabilize, melee, melee_structure, shove, open_door) = match actor {
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
            // GTW-508 — the melee-STRUCTURE target: an 8-adjacent intact Cover / Wall cell.
            // Offered ONLY when NO meleeable ganger is in reach (a ganger strike takes priority),
            // so the one Melee button routes to a ganger strike or a cover-smash, never both.
            let melee_structure = if melee.is_some() {
                None
            } else {
                scan_melee_structure(*actor_pos, &grids)
            };
            // GTW-525 — the SHOVE target: an 8-adjacent, ALIVE, OPPOSING ganger. A WEAKER gate
            // than Melee's — NO LOS required (a shove is contact) and NO weapon required (any
            // ganger can shove), so it needs neither the actor's stance/facing nor the LOS grids.
            let shove = scan_shove_target(*actor_pos, *actor_faction, &candidates);
            // GTW-315 — the OPEN-DOOR target: the first 8-adjacent openable terrain entity in the
            // CLOSED state. The button always OPENS (an already-open door is not offered; closing
            // is not a contextual act), and F4 is PLAYER-ONLY — this offer runs only for a selected
            // PLAYER-faction actor, which the selection resolve already scopes to.
            let open_door = scan_open_door(*actor_pos, &doors);
            (execute, stabilize, melee, melee_structure, shove, open_door)
        }
        None => (None, None, None, None, None, None),
    };

    // Write the offers onto the seam (the press router reads these).
    *targets =
        ContextualTargets::with(execute, stabilize, melee, melee_structure, shove, open_door);

    // Toggle visibility IN PLACE — never despawn (ui-mutate-not-respawn). The Melee button shows
    // when EITHER a meleeable ganger OR an adjacent structure to smash is in reach (GTW-508).
    let melee_offered = melee.is_some() || melee_structure.is_some();
    set_visibility(&mut execute_btn, execute.is_some());
    set_visibility(&mut stabilize_btn, stabilize.is_some());
    set_visibility(&mut melee_btn, melee_offered);
    // GTW-525 — the Shove button shows when an 8-adjacent alive opposing ganger is in reach.
    set_visibility(&mut shove_btn, shove.is_some());
    // GTW-315 — the Open Door button shows when an 8-adjacent CLOSED door is in reach.
    set_visibility(&mut open_door_btn, open_door.is_some());
    // The panel shows iff at least one contextual act is offered.
    set_visibility(
        &mut panel_root,
        execute.is_some()
            || stabilize.is_some()
            || melee_offered
            || shove.is_some()
            || open_door.is_some(),
    );
}

/// The pure offer scans the brain runs — the Execute/Stabilize downed scan + the GTW-507 melee
/// + the GTW-508 cover-smash scans, split into a submodule to keep each file under the size cap.
mod scan;

use scan::{
    scan_melee_structure, scan_melee_target, scan_open_door, scan_shove_target, scan_targets,
};

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
