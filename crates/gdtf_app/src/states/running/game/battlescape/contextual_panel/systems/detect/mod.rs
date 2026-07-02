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
//! - **Enter Emplacement** (GTW-543) — the first 8-adjacent VACANT weapon-emplacement terrain
//!   entity (carrying an [`EmplacementState`](gdtf_battle_sim::EmplacementState) that is
//!   `Vacant`) — the deliberate man act. F4 is PLAYER-ONLY (this runs only for a selected
//!   player-faction actor).
//! - **Exit Emplacement** (GTW-543) — the emplacement whose recorded
//!   [`EmplacementOccupant`](gdtf_battle_sim::EmplacementOccupant) IS the current selection — the
//!   deliberate dismount act. Offered ONLY to the occupant (there is NO force-eject; exit is a
//!   SEPARATE TU-costed act).
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
use gdtf_battle_input::{InspectTarget, SelectedShooter};
use gdtf_battle_sim::{
    CellLevel, CoverLedger, EmplacementOccupant, EmplacementState, OccupancyGrid, OpenState,
    SurfaceGrid, TrajectoryStyle,
    entity::TerrainCell,
    ganger::{Facing, Faction, LifeState, Position, Stabilized, Stance},
    tuning::CombatTuning,
    weapon::{MeleeWeapon, Wields},
};

use crate::states::running::game::battlescape::contextual_panel::components::{
    ContextualPanelRoot, ContextualTargets, EnterEmplacementButton, ExecuteButton,
    ExitEmplacementButton, MeleeButton, OpenDoorButton, ShoveButton, StabilizeButton,
    ThrowGrenadeButton,
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

/// The candidate-EMPLACEMENT reads the GTW-543 enter/exit scans need — each weapon-emplacement
/// terrain entity's handle, its [`EmplacementState`] (the enter scan offers only a VACANT
/// emplacement), its [`TerrainCell`] (the cell the actor must be 8-adjacent to for enter), and its
/// OPTIONAL [`EmplacementOccupant`] (present only while occupied — the exit scan offers the
/// emplacement whose occupant IS the selection). A named alias to keep
/// [`detect_contextual_targets`]'s `emplacements` query legible under clippy `type_complexity`.
///
/// An EMPLACEMENT is any terrain entity carrying an [`EmplacementState`] — the GTW-543 mechanism
/// attaches it only to emplacement pieces at spawn, so `With<EmplacementState>` selects exactly the
/// weapon emplacements and never a ganger (a ganger has no `EmplacementState`), keeping this query
/// disjoint from the ganger `candidates` / `actors` and the `doors` queries. (A door carries
/// `OpenState`, an emplacement `EmplacementState` — distinct components, so the two terrain queries
/// are also disjoint.)
type EmplacementReads = (
    Entity,
    &'static EmplacementState,
    &'static TerrainCell,
    Option<&'static EmplacementOccupant>,
);

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
    Without<EnterEmplacementButton>,
    Without<ExitEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Execute** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type ExecuteVisFilter = (
    With<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
    Without<EnterEmplacementButton>,
    Without<ExitEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Stabilize** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type StabilizeVisFilter = (
    With<StabilizeButton>,
    Without<ExecuteButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
    Without<EnterEmplacementButton>,
    Without<ExitEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Melee** button's [`Visibility`] disjointly from the other
/// contextual markers (GTW-507) — a named alias for clippy `type_complexity`.
type MeleeVisFilter = (
    With<MeleeButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
    Without<EnterEmplacementButton>,
    Without<ExitEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Shove** button's [`Visibility`] disjointly from the other
/// contextual markers (GTW-525) — a named alias for clippy `type_complexity`.
type ShoveVisFilter = (
    With<ShoveButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<OpenDoorButton>,
    Without<EnterEmplacementButton>,
    Without<ExitEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Open Door** button's [`Visibility`] disjointly from the other
/// contextual markers — a named alias for clippy `type_complexity`.
type OpenDoorVisFilter = (
    With<OpenDoorButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<EnterEmplacementButton>,
    Without<ExitEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Enter Emplacement** button's [`Visibility`] disjointly from the
/// other contextual markers (GTW-543) — a named alias for clippy `type_complexity`.
type EnterEmplacementVisFilter = (
    With<EnterEmplacementButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
    Without<ExitEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Exit Emplacement** button's [`Visibility`] disjointly from the
/// other contextual markers (GTW-543) — a named alias for clippy `type_complexity`.
type ExitEmplacementVisFilter = (
    With<ExitEmplacementButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
    Without<EnterEmplacementButton>,
    Without<ThrowGrenadeButton>,
);

/// Query filter selecting the **Throw** button's [`Visibility`] disjointly from the other
/// contextual markers (GTW-546) — a named alias for clippy `type_complexity`.
type ThrowGrenadeVisFilter = (
    With<ThrowGrenadeButton>,
    Without<ExecuteButton>,
    Without<StabilizeButton>,
    Without<MeleeButton>,
    Without<ShoveButton>,
    Without<OpenDoorButton>,
    Without<EnterEmplacementButton>,
    Without<ExitEmplacementButton>,
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

/// The reads the GTW-546 THROW offer needs, bundled into one
/// [`SystemParam`](bevy::ecs::system::SystemParam) so [`detect_contextual_targets`] stays under
/// clippy's argument-count gate (the [`LosGrids`] precedent).
///
/// A blind throw is offered when the selection wields a
/// [`TrajectoryStyle::Arc`](gdtf_battle_sim::TrajectoryStyle) weapon (resolved `selection ->
/// Wields -> the RANGED weapon entity -> its TrajectoryStyle`, EXCLUDING the melee weapon /
/// fists via `With<MeleeWeapon>`) AND a target cell is hovered
/// ([`InspectTarget::hovered`](gdtf_battle_input::InspectTarget::hovered)). All reads are read-only
/// (the detection layer never mutates the sim / input). [`InspectTarget`] is `Option` so the
/// system stays valid before the picker inserts it (`bevy-traps.md` #1); the throw offer then
/// simply has no target cell.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct ThrowReads<'w, 's> {
    /// The live hovered cell (the picker's per-update write) — the BLIND throw's target CELL.
    /// `Option` so the offer stays valid before the picker inserts the resource.
    inspect: Option<Res<'w, InspectTarget>>,
    /// The `selection -> Wields -> weapon entities` relationship read — the throw resolves the
    /// RANGED weapon through this (the sim's `ranged_weapon` idiom).
    wields:  Query<'w, 's, &'static Wields>,
    /// The `With<MeleeWeapon>` filter query backing the `is_melee` closure `ranged_weapon` takes —
    /// so the throw reads the RANGED weapon's trajectory, never the melee / fists entity.
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
    /// Each weapon entity's [`TrajectoryStyle`] — the throw is offered only when the resolved
    /// ranged weapon's style is [`Arc`](gdtf_battle_sim::TrajectoryStyle::Arc).
    styles:  Query<'w, 's, &'static TrajectoryStyle>,
}

impl ThrowReads<'_, '_> {
    /// The target CELL a THROW would lob at (GTW-546), or [`None`] when `actor` wields no
    /// [`Arc`](TrajectoryStyle::Arc) weapon or no cell is hovered.
    ///
    /// Resolves `actor -> Wields -> the RANGED weapon entity` (EXCLUDING the melee / fists entity
    /// via the `With<MeleeWeapon>` filter — the sim's `ranged_weapon` idiom), reads that weapon's
    /// [`TrajectoryStyle`], and offers the hovered cell only when the style is `Arc`. The throw is
    /// BLIND — no adjacency / LOS gate — so the ONLY offer condition is "wields an `Arc` weapon and
    /// the cursor is over a cell". The sim's
    /// [`dispatch_throw_grenade`](gdtf_battle_sim::acts::dispatch_throw_grenade) re-gate (loaded
    /// round + affords `ThrowTu`) is the authoritative check; this only decides what to OFFER.
    fn throw_target(&self, actor: Entity) -> Option<CellLevel> {
        let hovered = self.inspect.as_ref()?.hovered()?;
        let wields = self.wields.get(actor).ok()?;
        let weapon = wields.ranged_weapon(|entity| self.melee.get(entity).is_ok())?;
        let style = self.styles.get(weapon).ok()?;
        style.is_arc().then_some(hovered)
    }
}

/// The nine disjoint per-marker `&mut Visibility` queries the detection system toggles IN PLACE
/// — the panel root + eight contextual buttons — bundled into ONE
/// [`SystemParam`](bevy::ecs::system::SystemParam) so [`detect_contextual_targets`] stays under
/// Bevy's 16-param system limit (the GTW-546 Throw button pushed the flat form past it).
///
/// Each field is a `Query<&mut Visibility, …>` disjoint by its `With<M>` / `Without<other-markers>`
/// filter, so grouping them is a pure legibility / arity bundle, not a borrow change — the toggling
/// still writes each node's [`Visibility`] in place (never despawn — `ui-mutate-not-respawn`). A
/// transparent system-param bundle, not itself a wrapped domain scalar.
#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::states::running::game::battlescape) struct PanelVisibility<'w, 's> {
    /// The panel ROOT box's visibility — shown iff ANY contextual act is offered.
    root:              Query<'w, 's, &'static mut Visibility, RootVisFilter>,
    /// The **Execute** button's visibility (GTW-294).
    execute:           Query<'w, 's, &'static mut Visibility, ExecuteVisFilter>,
    /// The **Stabilize** button's visibility (GTW-294).
    stabilize:         Query<'w, 's, &'static mut Visibility, StabilizeVisFilter>,
    /// The **Melee** button's visibility (GTW-507).
    melee:             Query<'w, 's, &'static mut Visibility, MeleeVisFilter>,
    /// The **Shove** button's visibility (GTW-525).
    shove:             Query<'w, 's, &'static mut Visibility, ShoveVisFilter>,
    /// The **Open Door** button's visibility (GTW-315).
    open_door:         Query<'w, 's, &'static mut Visibility, OpenDoorVisFilter>,
    /// The **Enter Emplacement** button's visibility (GTW-543).
    enter_emplacement: Query<'w, 's, &'static mut Visibility, EnterEmplacementVisFilter>,
    /// The **Exit Emplacement** button's visibility (GTW-543).
    exit_emplacement:  Query<'w, 's, &'static mut Visibility, ExitEmplacementVisFilter>,
    /// The **Throw** button's visibility (GTW-546).
    throw_grenade:     Query<'w, 's, &'static mut Visibility, ThrowGrenadeVisFilter>,
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
/// a read-only `emplacements` [`Query`] (the GTW-543 enter/exit scan — disjoint via
/// `With<EmplacementState>`), the [`LosGrids`] bundle the melee LOS gate reads, the GTW-546
/// [`ThrowReads`] bundle the throw offer reads, and the [`PanelVisibility`] bundle of nine disjoint
/// per-marker `Query<&mut Visibility, …>`s (the root + eight buttons) the toggling writes.
#[expect(
    clippy::too_many_arguments,
    reason = "the read-only scan inputs (selection + targets seam + actors/candidates/doors/\
              emplacements queries + the GTW-507 LosGrids + the GTW-546 ThrowReads) alongside the \
              PanelVisibility toggle bundle sit just over clippy's argument gate; each is a \
              cohesive concern already bundled where it groups, and the disjoint scan queries \
              cannot fold further"
)]
pub(in crate::states::running::game::battlescape) fn detect_contextual_targets(
    selected: Res<SelectedShooter>,
    mut targets: ResMut<ContextualTargets>,
    actors: Query<ActorReads>,
    candidates: Query<CandidateReads>,
    doors: Query<DoorReads>,
    emplacements: Query<EmplacementReads>,
    grids: LosGrids,
    throw: ThrowReads,
    mut vis: PanelVisibility,
) {
    // Resolve the actor: a selection holding an entity that carries the read components. Any
    // miss (no selection, or a selection lacking the reads) clears the offers + hides the panel
    // — fail-closed, no panic. The selected ENTITY (not just its reads) is kept for the GTW-543
    // exit scan, which offers the emplacement whose recorded occupant IS this selection.
    let selected_entity = **selected;
    let actor = selected_entity.and_then(|entity| actors.get(entity).ok());

    let (
        execute,
        stabilize,
        melee,
        melee_structure,
        shove,
        open_door,
        enter_emp,
        exit_emp,
        throw_grenade,
    ) = match (selected_entity, actor) {
        (Some(actor_entity), Some((actor_pos, actor_faction, actor_stance, actor_facing))) => {
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
            // GTW-543 — the ENTER-EMPLACEMENT target: the first 8-adjacent VACANT weapon
            // emplacement. F4 is PLAYER-ONLY — this offer runs only for a selected PLAYER-faction
            // actor, which the selection resolve already scopes to. And the EXIT-EMPLACEMENT
            // target: the emplacement whose recorded occupant IS this selection (offered ONLY to
            // the occupant — there is NO force-eject).
            let enter_emp = scan_enter_emplacement(*actor_pos, &emplacements);
            let exit_emp = scan_exit_emplacement(actor_entity, &emplacements);
            // GTW-546 — the THROW target: the hovered cell, offered ONLY when the selection
            // wields an `Arc` weapon. A BLIND lob — no adjacency / LOS gate. F4 PLAYER-ONLY
            // (the selection resolve already scopes to a player-faction actor).
            let throw_grenade = throw.throw_target(actor_entity);
            (
                execute,
                stabilize,
                melee,
                melee_structure,
                shove,
                open_door,
                enter_emp,
                exit_emp,
                throw_grenade,
            )
        }
        _ => (None, None, None, None, None, None, None, None, None),
    };

    // Write the offers onto the seam (the press router reads these). Built via a direct struct
    // literal — the fields are `pub(in …battlescape)`, visible here, and an eight-field constructor
    // would trip clippy's argument-count gate.
    *targets = ContextualTargets {
        execute,
        stabilize,
        melee,
        melee_structure,
        shove,
        open_door,
        enter_emplacement: enter_emp,
        exit_emplacement: exit_emp,
        throw_grenade,
    };

    // Toggle visibility IN PLACE — never despawn (ui-mutate-not-respawn). The Melee button shows
    // when EITHER a meleeable ganger OR an adjacent structure to smash is in reach (GTW-508).
    let melee_offered = melee.is_some() || melee_structure.is_some();
    set_visibility(&mut vis.execute, execute.is_some());
    set_visibility(&mut vis.stabilize, stabilize.is_some());
    set_visibility(&mut vis.melee, melee_offered);
    // GTW-525 — the Shove button shows when an 8-adjacent alive opposing ganger is in reach.
    set_visibility(&mut vis.shove, shove.is_some());
    // GTW-315 — the Open Door button shows when an 8-adjacent CLOSED door is in reach.
    set_visibility(&mut vis.open_door, open_door.is_some());
    // GTW-543 — the Enter button shows when an 8-adjacent VACANT emplacement is in reach; the Exit
    // button shows ONLY when the selection is manning an emplacement (offered to the occupant only).
    set_visibility(&mut vis.enter_emplacement, enter_emp.is_some());
    set_visibility(&mut vis.exit_emplacement, exit_emp.is_some());
    // GTW-546 — the Throw button shows when the selection wields an `Arc` weapon and a target cell
    // is hovered (a BLIND lob — no adjacency / LOS gate).
    set_visibility(&mut vis.throw_grenade, throw_grenade.is_some());
    // The panel shows iff at least one contextual act is offered.
    set_visibility(
        &mut vis.root,
        execute.is_some()
            || stabilize.is_some()
            || melee_offered
            || shove.is_some()
            || open_door.is_some()
            || enter_emp.is_some()
            || exit_emp.is_some()
            || throw_grenade.is_some(),
    );
}

/// The pure offer scans the brain runs — the Execute/Stabilize downed scan + the GTW-507 melee
/// + the GTW-508 cover-smash scans, split into a submodule to keep each file under the size cap.
mod scan;

use scan::{
    scan_enter_emplacement, scan_exit_emplacement, scan_melee_structure, scan_melee_target,
    scan_open_door, scan_shove_target, scan_targets,
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
