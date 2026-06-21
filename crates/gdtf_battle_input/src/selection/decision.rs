//! The shared click/turn DECISIONS (GTW-238 / GTW-259): the read-only [`decide_left_click`] /
//! [`apply_left_click`] split and the [`decide_turn`] geometry — ONE precedence implementation
//! the mouse and the gamepad both use, plus the [`LeftClickReads`] / [`TurnReads`] bundles.

use bevy::prelude::*;
use gdtf_battle_sim::{
    Cell, Direction, Faction, OccupancyGrid, PlayerFaction, Position, WieldedBy, Wields,
    acts::{FireRequested, MoveRequested, SetFacingRequested},
    tuning::CombatTuning,
};

use crate::{
    ActIntent, InspectTarget, PendingActIntent, SelectedFireMode,
    fire_surface::{ShooterFireData, WeaponMagazine, try_fire_request},
    selection::resources::{SelectedShooter, set_selection},
};

/// The read-only resources [`decide_left_click`] consults, grouped into ONE [`SystemParam`]
/// so a consuming system's parameter list stays under clippy's argument-count gate (the sim's
/// `BattleGridsParam` / the seam's `ActWriters` precedent).
///
/// Grouping the cohesive `Res<…>` reads into one param keeps
/// [`left_click_act`](crate::left_click_act) at five parameters. A transparent system-param
/// bundle of framework resources + landed newtypes — not itself a wrapped domain scalar.
///
/// The [`InspectTarget`] is DELIBERATELY NOT in this bundle (GTW-300): the click systems take it
/// as a [`ResMut<InspectTarget>`] (they WRITE its pin via [`apply_pin`]), and a `Res` of the same
/// resource in this bundle would be a `Res` + `ResMut` aliasing conflict (B0002). So
/// [`decide_left_click`] / [`decide_pin`] receive the inspect target as a separate `&` argument.
#[derive(bevy::ecs::system::SystemParam)]
pub struct LeftClickReads<'w> {
    /// The mouse button state — the Left press edge gates the whole decision.
    pub(super) mouse: Res<'w, ButtonInput<MouseButton>>,
    /// The coarse occupancy grid — the occupant + `is_blocked` reads.
    occupancy:        Res<'w, OccupancyGrid>,
    /// The selected fire mode — the FIRE branch's mode + the `can_fire` cone-mult.
    fire_mode:        Res<'w, SelectedFireMode>,
    /// The combat tuning the shared `can_fire` guard reads.
    tuning:           Res<'w, CombatTuning>,
    /// The player's own faction — the friend/foe gate for every branch.
    player:           Res<'w, PlayerFaction>,
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
    /// NO-OP — does NOTHING, leaving [`SelectedShooter`] untouched (no clear, no transient
    /// `None`). Two cases reach it: clause 3.5 (GTW-287), the clicked cell holds an ENEMY you
    /// cannot FIRE on (enemies are inspected via the GTW-274 hover panel, never selected or
    /// cleared as your shooter); and the no-hover case (GTW-288), a click with no map cell
    /// (cursor over the UI / a margin / off the map, the GTW-286 viewport gate) — which must
    /// not clear the selection, lest it flicker the status panel and break Mode/Stance.
    NoOp,
    /// CLEAR the selection — clause 4: a valid in-grid hovered cell
    /// ([`InspectTarget::hovered`] is [`Some`]) where none of FIRE / SELECT / MOVE / NO-OP
    /// applied (an empty / blocked cell with a non-player or stale selection). Sets
    /// [`SelectedShooter::cleared`]. The no-hover case is NOT here — that is the NO-OP above
    /// (GTW-288).
    Clear,
}

/// The PARALLEL inspect-panel pin effect of one left-click edge — ORTHOGONAL to the
/// [`LeftClickOutcome`] selection/act effect (GTW-300).
///
/// A SECOND domain enum, NOT folded into [`LeftClickOutcome`], because the pin is a VIEW
/// concern that must COEXIST with the selection/act effect: clicking your own ganger both
/// SELECTS it AND keeps the pin; clicking an empty tile both CLEARS the selection AND unpins.
/// One enum can't carry both effects without a compound case per combination — keeping them as
/// two orthogonal outcomes (decided together, applied together) is the "composes correctly,
/// no double-dispatch" the contract names: each effect is committed exactly once, on its own
/// resource. [`decide_pin`] computes it alongside [`decide_left_click`]; [`apply_pin`] commits
/// it to the [`InspectTarget`] right after [`apply_left_click`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinOutcome {
    /// PIN the inspect panel to the carried cell — the click landed on COVER (a wall / cover
    /// cell) or an ENEMY fighter. The panel freezes on that cell's occupant / terrain until an
    /// [`Unpin`](PinOutcome::Unpin).
    Pin(gdtf_battle_sim::CellLevel),
    /// UNPIN the inspect panel — the click landed on an EMPTY in-grid tile; hover resumes.
    Unpin,
    /// KEEP the current pin untouched — the click was on your OWN ganger (a SELECT), a FIRE on
    /// an enemy, or a no-hover click (over the UI / a margin / off the map). The pin is a
    /// separate view state; these clicks leave it exactly as it was.
    Keep,
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
///    (`occupant == None`), in-bounds (a [`Some`] hovered cell is always in-grid), and
///    unblocked (`!is_blocked`) → [`LeftClickOutcome::Move`].
/// 4. **CLEAR** — a valid in-grid hovered cell (a [`Some`] [`InspectTarget::hovered`]) where none
///    of the above applied → [`LeftClickOutcome::Clear`]. The genuine "nothing to act on" cases
///    (an empty / blocked cell with a non-player or stale selection) still clear.
///
/// A NO-HOVER click is NOT clause 4: when the hovered cell is [`None`] — a click with no map
/// cell (cursor over the bottom UI / a margin / off the map, which the GTW-286 viewport gate
/// resolves to no cell) — [`decide_left_click`] returns [`LeftClickOutcome::NoOp`] up front
/// (GTW-288), NOT [`LeftClickOutcome::Clear`]. Clearing on every over-UI click flickered the
/// status panel and broke Mode/Stance (which need the selection to resolve the weapon).
///
/// Between MOVE (clause 3) and CLEAR (clause 4) sits the GTW-287 **NO-OP** rung: when the
/// hovered cell holds an ENEMY occupant ([`Faction`] `!=` [`PlayerFaction`]) you could not FIRE
/// on, [`decide_left_click`] returns [`LeftClickOutcome::NoOp`] instead of falling through to
/// CLEAR. Clicking an enemy you can't fire on leaves your current player selection UNTOUCHED
/// (enemies are inspected via the GTW-274 hover panel, not selected/cleared as your shooter) —
/// no clear, no transient `None`, no "No ganger selected" flash, no auto-select revert.
///
/// FALL-THROUGH (the user-confirmed precedence): a fire mode over an EMPTY / your-own /
/// non-enemy cell FAILS clause 1 (no enemy occupant) and falls through to clause 3 (MOVE) — the
/// fire mode does NOT lock out move. An ENEMY-occupied cell you cannot fire on does NOT fall
/// through to CLEAR (it would wipe the selection); it is the GTW-287 NO-OP rung above.
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] read bundle + the [`InspectTarget`]
/// (passed as a separate `&` so the caller can hold it as a `ResMut` for the pin write — see
/// [`LeftClickReads`]) + read-only `Query<&Faction>` / `Query<ShooterFireData>` / `Query<&Wields>`
/// / the weapon-[`Magazine`](gdtf_battle_sim::Magazine) query + the current [`SelectedShooter`],
/// no `&mut World`. The `Wields` + weapon-magazine queries resolve the fire guard's magazine off
/// the related weapon entity (`ganger → Wields → the weapon entity`, GTW-323 slice 3). It does
/// NOT read the mouse button (the caller gates on its own press edge), so the gamepad surface
/// reuses it without a `ButtonInput<MouseButton>`.
#[must_use]
pub fn decide_left_click(
    reads: &LeftClickReads,
    inspect: &InspectTarget,
    factions: &Query<&Faction>,
    shooters: &Query<ShooterFireData>,
    wields: &Query<&Wields>,
    weapons: &Query<WeaponMagazine, With<WieldedBy>>,
    selected: &SelectedShooter,
) -> LeftClickOutcome {
    // Nothing hovered -> no cell to act on -> NO-OP (GTW-288): the GTW-286 viewport gate
    // resolves a click over the bottom UI / a margin / off the map to no hovered cell, and
    // such a click must leave the selection UNTOUCHED (NOT clear it — that flickered the
    // status panel and broke Mode/Stance, which need the selection to resolve the weapon).
    // The click acts on the LIVE hovered cell (where you clicked), independent of any panel pin.
    let Some(target) = inspect.hovered() else {
        return LeftClickOutcome::NoOp;
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
        && let Some(request) = try_fire_request(
            shooter,
            target,
            &reads.fire_mode,
            &reads.tuning,
            shooters,
            wields,
            weapons,
        )
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

    // 3.5. NO-OP (GTW-287) — the cell holds an ENEMY you couldn't FIRE on. Clicking an enemy you
    // can't fire on must NOT clear the player's selection (the "No ganger selected" flash bug):
    // it is a no-op, the enemy is inspected via the GTW-274 hover panel. Reached only after FIRE
    // failed above, so this is the can't-fire-on-this-enemy case.
    if occupant_faction.is_some_and(|faction| faction != player) {
        return LeftClickOutcome::NoOp; // enemy click -> leave the selection untouched.
    }

    // 4. CLEAR — none of the above.
    LeftClickOutcome::Clear
}

/// Resolves ONE left-click edge into its PARALLEL inspect-panel [`PinOutcome`] (GTW-300) — the
/// SHARED pin decision the mouse and the gamepad both use, ORTHOGONAL to [`decide_left_click`].
///
/// READ-ONLY: it reads the LIVE hovered cell (where you clicked), the occupant + its faction at
/// that cell, and the terrain, and returns the resolved pin effect WITHOUT touching any state.
/// Computed ALONGSIDE [`decide_left_click`] so a single click yields both effects; they are
/// committed by [`apply_pin`] / [`apply_left_click`] independently (the contract's "composes
/// correctly"). The pin precedence (each tested against the LIVE hovered cell):
///
/// 1. **PIN to cover** — the cell's [`TerrainKind`](gdtf_battle_sim::TerrainKind) is a wall /
///    cover ([`is_blocked`](gdtf_battle_sim::OccupancyGrid::is_blocked)) → [`PinOutcome::Pin`].
/// 2. **PIN to an enemy** — the cell holds an occupant whose [`Faction`] differs from the
///    [`PlayerFaction`] → [`PinOutcome::Pin`]. (So clicking an enemy pins it whether or not the
///    click ALSO fires — the FIRE branch in [`decide_left_click`] is independent.)
/// 3. **KEEP** — the cell holds one of YOUR OWN gangers (a SELECT) → [`PinOutcome::Keep`]
///    (selecting your ganger never disturbs the pin).
/// 4. **UNPIN** — an EMPTY in-grid tile (no enemy, no cover, no own ganger) → [`PinOutcome::Unpin`]
///    (hover resumes normally).
///
/// A NO-HOVER click (the live hovered cell is [`None`] — over the UI / a margin / off the map,
/// the GTW-286 viewport gate) → [`PinOutcome::Keep`]: a click that hit no map cell must never
/// disturb the pin (mirrors [`decide_left_click`]'s NO-OP for the same case, GTW-288).
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] bundle + the [`InspectTarget`] (a
/// separate `&` so the caller can hold the same resource as a `ResMut` for the pin write) + the
/// read-only `Query<&Faction>`, no `&mut World`. Does NOT read the mouse button (the caller gates
/// on its own press edge), so the gamepad surface reuses it identically.
#[must_use]
pub fn decide_pin(
    reads: &LeftClickReads,
    inspect: &InspectTarget,
    factions: &Query<&Faction>,
) -> PinOutcome {
    // The click acts on the LIVE hovered cell (where you clicked), independent of any panel pin.
    let Some(cell) = inspect.hovered() else {
        // No map cell (over the UI / a margin / off the map) — leave the pin untouched (GTW-288).
        return PinOutcome::Keep;
    };
    let player = **reads.player;
    let occupant = reads.occupancy.occupant(&cell);
    let occupant_faction = occupant.and_then(|e| factions.get(e).ok().copied());

    // 1. PIN to COVER — a wall / cover cell. Pinning is a VIEW concern, so it pins the
    //    structural object regardless of selection / fire mode.
    if reads.occupancy.is_blocked(&cell) {
        return PinOutcome::Pin(cell);
    }
    // 2. PIN to an ENEMY — an occupant whose faction differs from the player's. Independent of
    //    whether the click ALSO fires (the FIRE branch is decided separately).
    if occupant_faction.is_some_and(|faction| faction != player) {
        return PinOutcome::Pin(cell);
    }
    // 3. KEEP — your OWN ganger (a SELECT): selecting never disturbs the pin.
    if occupant_faction.is_some_and(|faction| faction == player) {
        return PinOutcome::Keep;
    }
    // 4. UNPIN — an empty in-grid tile: hover resumes normally.
    PinOutcome::Unpin
}

/// Commits a [`decide_left_click`] [`LeftClickOutcome`] to the input-layer state — the SHARED
/// apply step (GTW-259) the mouse and gamepad both run.
///
/// FIRE / MOVE push the carried act onto [`PendingActIntent`] (the ONE
/// [`dispatch_act_intents`](crate::dispatch_act_intents) drain emits it); SELECT sets
/// [`SelectedShooter::new`]; CLEAR sets [`SelectedShooter::cleared`]. The selection writes go
/// through `set_selection` (change-detection hygiene). FIRE / MOVE / NO-OP do NOT touch the
/// selection — and [`LeftClickOutcome::NoOp`] (GTW-287, the enemy-click case) does NOTHING at
/// all: no push, no `set_selection`, so the player's selection is left exactly as it was.
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
        // GTW-287 — an enemy-click no-op: do nothing, leave the selection untouched.
        LeftClickOutcome::NoOp => {}
        LeftClickOutcome::Clear => set_selection(selected, SelectedShooter::cleared()),
    }
}

/// Commits a [`decide_pin`] [`PinOutcome`] to the [`InspectTarget`]'s pin (GTW-300) — the SHARED
/// pin apply step the mouse and gamepad both run, ORTHOGONAL to [`apply_left_click`].
///
/// [`Pin`](PinOutcome::Pin) pins the cell ([`set_pinned`](InspectTarget::set_pinned));
/// [`Unpin`](PinOutcome::Unpin) clears it ([`clear_pin`](InspectTarget::clear_pin));
/// [`Keep`](PinOutcome::Keep) does NOTHING, leaving the pin exactly as it was. It writes ONLY the
/// pin — never the live hovered cell — so the cursor keeps tracking under a pin. Because pin and
/// selection live on DIFFERENT resources ([`InspectTarget`] vs
/// [`SelectedShooter`](crate::SelectedShooter) / [`PendingActIntent`]), this commit and
/// [`apply_left_click`] never conflict: each click's two effects are applied once each, no
/// double-dispatch.
///
/// Param-only (`bevy-traps.md` #7): the [`ResMut<InspectTarget>`] write, no `&mut World`.
pub fn apply_pin(outcome: PinOutcome, target: &mut ResMut<InspectTarget>) {
    match outcome {
        PinOutcome::Pin(cell) => {
            // Pin only on a real change so a re-pin of the same cell does not trip
            // `Changed<InspectTarget>` (the picker writes the hovered cell with the same hygiene).
            if target.pinned() != Some(cell) {
                target.set_pinned(cell);
            }
        }
        PinOutcome::Unpin => {
            if target.pinned().is_some() {
                target.clear_pin();
            }
        }
        // KEEP — leave the pin exactly as it was (own-ganger select, fire, or no-hover click).
        PinOutcome::Keep => {}
    }
}

/// The read-only resources the turn-to-face surfaces consult, grouped into ONE [`SystemParam`]
/// so each surface's parameter list stays under clippy's argument-count gate.
///
/// Grouping the cohesive `Res<…>` reads (the hovered cell, the player faction, and the current
/// selection) into one param keeps
/// [`right_click_turn_to_face`](crate::right_click_turn_to_face) (mouse) and
/// [`gamepad_turn`](crate::gamepad::gamepad_turn) (East) at four parameters each. A transparent
/// system-param bundle REUSED by BOTH turn surfaces so they read the SAME inputs identically.
#[derive(bevy::ecs::system::SystemParam)]
pub struct TurnReads<'w> {
    /// The inspect target — its LIVE hovered cell is the turn target (turning faces the cursor,
    /// independent of any panel pin).
    pub hovered:  Res<'w, InspectTarget>,
    /// The player's own faction — the turn surfaces act only on a player-faction selection.
    pub player:   Res<'w, PlayerFaction>,
    /// The current selection — the actor that turns.
    pub selected: Res<'w, SelectedShooter>,
}

/// Resolves the turn-to-face decision (GTW-238) into an optional [`SetFacingRequested`] — the
/// SHARED turn decision (GTW-259) the mouse ([`right_click_turn_to_face`](crate::right_click_turn_to_face))
/// and the gamepad ([`gamepad_turn`](crate::gamepad::gamepad_turn), the East button) both use.
///
/// READ-ONLY: given the player-faction [`SelectedShooter`] (already faction-gated by the
/// caller), the [`InspectTarget`] (its LIVE hovered cell), and the actor [`Position`] query, it reads the actor's cell,
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
/// [`InspectTarget`] (its LIVE hovered cell) + read-only `Query<&Position>`, no `&mut World`.
#[must_use]
pub fn decide_turn(
    selected: &SelectedShooter,
    hovered: &InspectTarget,
    positions: &Query<&Position>,
) -> Option<SetFacingRequested> {
    let actor = (**selected)?;
    // Nothing hovered -> no target cell. Faces the LIVE cursor cell, independent of any pin.
    let target = hovered.hovered()?;
    // The actor's grid cell (fail-closed if it has no Position component).
    let position = positions.get(actor).ok()?;
    let actor_cell = Cell::new(position.x, position.y);
    let hovered_cell = Cell::new(target.x, target.y);
    // Hovering the actor's OWN cell yields no direction -> no-op (None).
    let facing = Direction::from_cells(actor_cell, hovered_cell)?;
    Some(SetFacingRequested::new(actor, facing))
}
