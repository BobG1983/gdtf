//! The shared click/turn DECISIONS (GTW-238 / GTW-259): the read-only [`decide_left_click`] /
//! [`apply_left_click`] split and the [`decide_turn`] geometry — ONE precedence implementation
//! the mouse and the gamepad both use, plus the [`LeftClickReads`] / [`TurnReads`] bundles.

use bevy::prelude::*;
use gdtf_battle_sim::{
    Cell, Direction, Faction, OccupancyGrid, PlayerFaction, Position,
    acts::{FireRequested, MoveRequested, SetFacingRequested},
    tuning::CombatTuning,
};

use crate::{
    ActIntent, HoveredCell, PendingActIntent, SelectedFireMode,
    fire_surface::{ShooterFireData, try_fire_request},
    selection::resources::{SelectedShooter, WorldClickSuppressed, set_selection},
};

/// The read-only resources [`decide_left_click`] consults, grouped into ONE [`SystemParam`]
/// so a consuming system's parameter list stays under clippy's argument-count gate (the sim's
/// `BattleGridsParam` / the seam's `ActWriters` precedent).
///
/// Grouping the seven cohesive `Res<…>` reads into one param keeps
/// [`left_click_act`](crate::left_click_act) at five parameters. A transparent system-param
/// bundle of framework resources + landed newtypes — not itself a wrapped domain scalar.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LeftClickReads<'w> {
    /// The mouse button state — the Left press edge gates the whole decision.
    pub(super) mouse: Res<'w, ButtonInput<MouseButton>>,
    /// The cell the cursor hovers (the click target), resolved last update.
    hovered:          Res<'w, HoveredCell>,
    /// The coarse occupancy grid — the occupant + `is_blocked` reads.
    occupancy:        Res<'w, OccupancyGrid>,
    /// The selected fire mode — the FIRE branch's mode + the `can_fire` cone-mult.
    fire_mode:        Res<'w, SelectedFireMode>,
    /// The combat tuning the shared `can_fire` guard reads.
    tuning:           Res<'w, CombatTuning>,
    /// The player's own faction — the friend/foe gate for every branch.
    player:           Res<'w, PlayerFaction>,
    /// Whether a `gdtf_app` modal (the GTW-254 picker) is capturing the pointer — when `true`
    /// the whole decision is inert (the click belongs to the modal, not the world).
    suppressed:       Res<'w, WorldClickSuppressed>,
}

impl LeftClickReads<'_> {
    /// Whether a `gdtf_app` modal is currently capturing the pointer (GTW-254 clause 6b).
    ///
    /// The press surface that consumes a [`LeftClickReads`] bundle but is NOT
    /// [`left_click_act`](crate::left_click_act) — the gamepad's
    /// [`gamepad_click_act`](crate::gamepad::gamepad_click_act) — reads the suppression flag
    /// through this accessor (the bundle's fields are private), and is inert while it is
    /// `true`. Returns by value (a one-byte `Copy` newtype).
    #[must_use]
    pub fn suppressed(&self) -> WorldClickSuppressed {
        *self.suppressed
    }
}

/// The single resolved outcome of one left-click edge — the ONE source of truth the mouse
/// ([`left_click_act`](crate::left_click_act)) AND the gamepad
/// ([`gamepad_click_act`](crate::gamepad::gamepad_click_act)) both decide and apply (GTW-259).
///
/// A domain enum (no-bare-types: a resolved control decision is a named value, not a bare
/// discriminant) carrying exactly the payload each branch of the FIRE → SELECT → MOVE → CLEAR
/// precedence chain (GTW-238) needs. [`decide_left_click`] computes it from READ-ONLY inputs,
/// and [`apply_left_click`] commits it. Splitting decide-from-apply keeps the precedence in ONE
/// place: duplicating it per input device is the defect this refactor prevents.
#[derive(Debug, Clone, PartialEq)]
pub enum LeftClickOutcome {
    /// FIRE the carried request — clause 1 won (a fire mode, an ENEMY occupant, a player-faction
    /// selection, and the shared `can_fire` guard passed). Pushed as an [`ActIntent::Fire`]; the
    /// selection is unchanged.
    Fire(FireRequested),
    /// SELECT the carried entity — clause 2 won (the hovered cell holds one of YOUR gangers).
    /// Sets [`SelectedShooter::new`]; no act emitted.
    Select(Entity),
    /// MOVE the carried request — clause 3 won (a player-faction selection over an empty,
    /// in-bounds, unblocked cell). Pushed as an [`ActIntent::Move`]; the selection is unchanged.
    Move(MoveRequested),
    /// CLEAR the selection — clause 4 (none of the above, or nothing hovered). Sets
    /// [`SelectedShooter::cleared`].
    Clear,
}

/// Resolves ONE left-click edge through the FIRE → SELECT → MOVE → CLEAR precedence chain
/// (GTW-238) into a [`LeftClickOutcome`] — the SHARED decision (GTW-259) the mouse and the
/// gamepad both use.
///
/// READ-ONLY: it reads the hovered cell, the occupancy, the fire mode / tuning, the player
/// faction, the current selection, and the faction / firing components off the queries, and
/// returns the resolved outcome WITHOUT touching any state.
///
/// 1. **FIRE** — a fire mode is selected, the hovered cell holds an ENEMY occupant
///    (a [`Faction`] `!=` [`PlayerFaction`]), the current selection is a player-faction ganger,
///    and the shared [`can_fire`](gdtf_battle_sim::can_fire) guard passes ([`try_fire_request`])
///    → [`LeftClickOutcome::Fire`].
/// 2. **SELECT** — the hovered cell holds one of YOUR gangers
///    ([`Faction`] `==` [`PlayerFaction`]) → [`LeftClickOutcome::Select`].
/// 3. **MOVE** — there is a player-faction selection AND the hovered cell is empty
///    (`occupant == None`), in-bounds (a [`Some`] [`HoveredCell`] is always in-grid), and
///    unblocked (`!is_blocked`) → [`LeftClickOutcome::Move`].
/// 4. **CLEAR** — none of the above (or nothing hovered) → [`LeftClickOutcome::Clear`].
///
/// FALL-THROUGH (the user-confirmed precedence): a fire mode over an EMPTY / your-own /
/// non-enemy cell FAILS clause 1 (no enemy occupant) and falls through to clause 3 (MOVE) — the
/// fire mode does NOT lock out move.
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] read bundle + read-only
/// `Query<&Faction>` / `Query<ShooterFireData>` + the current [`SelectedShooter`], no
/// `&mut World`. It does NOT read the mouse button (the caller gates on its own press edge), so
/// the gamepad surface reuses it without a `ButtonInput<MouseButton>`.
#[must_use]
pub fn decide_left_click(
    reads: &LeftClickReads,
    factions: &Query<&Faction>,
    shooters: &Query<ShooterFireData>,
    selected: &SelectedShooter,
) -> LeftClickOutcome {
    // Nothing hovered -> no cell to act on -> CLEAR (clause 4, the no-hover case).
    let Some(target) = **reads.hovered else {
        return LeftClickOutcome::Clear;
    };
    let player = **reads.player;
    // The occupant at the hovered cell (None when empty), and its faction (None when the
    // occupant carries no `Faction` component — fail-closed, treated as not-yours).
    let occupant = reads.occupancy.occupant(&target);
    let occupant_faction = occupant.and_then(|e| factions.get(e).ok().copied());
    // Whether the CURRENT selection is a player-faction ganger (gates FIRE + MOVE).
    let selection_is_player = (**selected)
        .and_then(|e| factions.get(e).ok().copied())
        .is_some_and(|f| f == player);

    // 1. FIRE — a fire mode + an ENEMY occupant + a player-faction selection + can_fire.
    if let (Some(shooter), Some(enemy_faction)) = (**selected, occupant_faction)
        && selection_is_player
        && enemy_faction != player
        && let Some(request) =
            try_fire_request(shooter, target, &reads.fire_mode, &reads.tuning, shooters)
    {
        return LeftClickOutcome::Fire(request); // FIRE wins this edge.
    }

    // 2. SELECT — the hovered cell holds one of YOUR gangers.
    if let (Some(entity), Some(faction)) = (occupant, occupant_faction)
        && faction == player
    {
        return LeftClickOutcome::Select(entity); // SELECT wins — no act emitted.
    }

    // 3. MOVE — a player-faction selection + an empty, in-bounds, unblocked hovered cell.
    if let Some(actor) = **selected
        && selection_is_player
        && occupant.is_none()
        && !reads.occupancy.is_blocked(&target)
    {
        return LeftClickOutcome::Move(MoveRequested::new(actor, target)); // MOVE wins.
    }

    // 4. CLEAR — none of the above.
    LeftClickOutcome::Clear
}

/// Commits a [`decide_left_click`] [`LeftClickOutcome`] to the input-layer state — the SHARED
/// apply step (GTW-259) the mouse and gamepad both run.
///
/// FIRE / MOVE push the carried act onto [`PendingActIntent`] (the ONE
/// [`dispatch_act_intents`](crate::dispatch_act_intents) drain emits it); SELECT sets
/// [`SelectedShooter::new`]; CLEAR sets [`SelectedShooter::cleared`]. The selection writes go
/// through `set_selection` (change-detection hygiene). FIRE / MOVE do NOT touch the selection.
///
/// Param-only (`bevy-traps.md` #7): the [`ResMut<SelectedShooter>`] /
/// [`ResMut<PendingActIntent>`] writes, no `&mut World`.
pub fn apply_left_click(
    outcome: LeftClickOutcome,
    selected: &mut ResMut<SelectedShooter>,
    pending: &mut ResMut<PendingActIntent>,
) {
    match outcome {
        LeftClickOutcome::Fire(request) => pending.push(ActIntent::Fire(request)),
        LeftClickOutcome::Select(entity) => {
            set_selection(selected, SelectedShooter::new(entity));
        }
        LeftClickOutcome::Move(request) => pending.push(ActIntent::Move(request)),
        LeftClickOutcome::Clear => set_selection(selected, SelectedShooter::cleared()),
    }
}

/// The read-only resources the turn-to-face surfaces consult, grouped into ONE [`SystemParam`]
/// so each surface's parameter list stays under clippy's argument-count gate.
///
/// Grouping the cohesive `Res<…>` reads (the hovered cell, the player faction, the current
/// selection, and the GTW-254 [`WorldClickSuppressed`] flag) into one param keeps
/// [`right_click_turn_to_face`](crate::right_click_turn_to_face) (mouse) and
/// [`gamepad_turn`](crate::gamepad::gamepad_turn) (East) at five parameters each. A transparent
/// system-param bundle REUSED by BOTH turn surfaces so they read the SAME inputs identically.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TurnReads<'w> {
    /// The cell the cursor / software cursor hovers — the turn target.
    pub hovered:    Res<'w, HoveredCell>,
    /// The player's own faction — the turn surfaces act only on a player-faction selection.
    pub player:     Res<'w, PlayerFaction>,
    /// The current selection — the actor that turns.
    pub selected:   Res<'w, SelectedShooter>,
    /// Whether a `gdtf_app` modal (the GTW-254 picker) is capturing the pointer — when `true`
    /// the turn is inert (the press belongs to the modal, not the world, clause 6b).
    pub suppressed: Res<'w, WorldClickSuppressed>,
}

/// Resolves the turn-to-face decision (GTW-238) into an optional [`SetFacingRequested`] — the
/// SHARED turn decision (GTW-259) the mouse ([`right_click_turn_to_face`](crate::right_click_turn_to_face))
/// and the gamepad ([`gamepad_turn`](crate::gamepad::gamepad_turn), the East button) both use.
///
/// READ-ONLY: given the player-faction [`SelectedShooter`] (already faction-gated by the
/// caller), the [`HoveredCell`], and the actor [`Position`] query, it reads the actor's cell,
/// computes the target [`Direction`] toward the hovered cell via
/// [`Direction::from_cells`](gdtf_battle_sim::Direction::from_cells), and returns
/// [`Some`]`(`[`SetFacingRequested::new`]`)` — or [`None`] (no turn) when: there is no
/// selection; nothing is hovered; the actor has no [`Position`] (fail-closed via the query
/// lookup); or the hovered cell is the actor's OWN cell (`from_cells` returns [`None`]).
///
/// The per-45deg-step turn TU cost is the SIM's facing dispatch (GTW-235), NOT here. The
/// `selected` faction GATE (player-only) stays at each call site so this helper is purely the
/// geometric decision both surfaces share.
///
/// Param-only (`bevy-traps.md` #7): reads via the passed-in [`SelectedShooter`] +
/// [`HoveredCell`] + read-only `Query<&Position>`, no `&mut World`.
#[must_use]
pub fn decide_turn(
    selected: &SelectedShooter,
    hovered: &HoveredCell,
    positions: &Query<&Position>,
) -> Option<SetFacingRequested> {
    let actor = (**selected)?;
    // Nothing hovered -> no target cell.
    let target = (**hovered)?;
    // The actor's grid cell (fail-closed if it has no Position component).
    let position = positions.get(actor).ok()?;
    let actor_cell = Cell::new(position.x, position.y);
    let hovered_cell = Cell::new(target.x, target.y);
    // Hovering the actor's OWN cell yields no direction -> no-op (None).
    let facing = Direction::from_cells(actor_cell, hovered_cell)?;
    Some(SetFacingRequested::new(actor, facing))
}
