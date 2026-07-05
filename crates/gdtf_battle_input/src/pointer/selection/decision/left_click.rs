//! The FIRE → SELECT → MOVE → CLEAR precedence: the [`LeftClickOutcome`] vocabulary + the one
//! shared decide (GTW-238 / GTW-259).

use bevy::prelude::*;
use gdtf_battle_presenter::cell_squad_visible;
use gdtf_battle_sim::{
    CellLevel, Faction, FactionRelation, MeleeQuery, WieldedBy, Wields,
    acts::{FireRequested, MoveRequested},
};

use super::reads::LeftClickReads;
use crate::{
    InspectTarget,
    fire_surface::{ShooterFireData, WeaponMagazine, try_fire_request},
    selection::{path_preview::PathPreviewTarget, resources::SelectedShooter},
};

/// The single resolved outcome of one left-click edge — the ONE source of truth the mouse
/// ([`left_click_act`](crate::left_click_act)) AND the gamepad
/// ([`gamepad_click_act`](crate::gamepad::gamepad_click_act)) both decide and apply (GTW-259).
///
/// A domain enum (no-bare-types: a resolved control decision is a named value, not a bare
/// discriminant) carrying exactly the payload each branch of the FIRE → SELECT → MOVE → CLEAR
/// precedence chain (GTW-238) needs. [`decide_left_click`] computes it from READ-ONLY inputs,
/// and [`apply_left_click`](super::apply_left_click) commits it. Splitting decide-from-apply keeps the precedence in ONE
/// place: duplicating it per input device is the defect this refactor prevents.
#[derive(Debug, Clone, PartialEq)]
pub enum LeftClickOutcome {
    /// FIRE the carried request — clause 1 won (a fire mode, an ENEMY occupant, a player-faction
    /// selection, and the shared `can_fire` guard passed). Pushed as an [`ActIntent::Fire`](crate::ActIntent::Fire); the
    /// selection is unchanged.
    Fire(FireRequested),
    /// SELECT the carried entity — clause 2 won (the hovered cell holds one of YOUR gangers).
    /// Sets [`SelectedShooter::new`] AND CLEARS the pending move target (GTW-356 C4: a new
    /// selection drops any stale route preview); no act emitted.
    Select(Entity),
    /// SET the carried cell as the move target — the GTW-356 two-click CLICK-1 / RE-TARGET
    /// rung (clause 3, OQ-5). A player-faction selection clicked a VALID move target (empty,
    /// in-bounds, unblocked, NOT a vertical-link tile) that does NOT equal the current
    /// [`PathPreviewTarget`] (it was [`None`] or a different cell). Sets
    /// [`PathPreviewTarget::new`] so the preview + range overlay + cost show; NO act emitted,
    /// the selection is unchanged. A SECOND click on the SAME cell then COMMITS ([`Move`]).
    SetMoveTarget(CellLevel),
    /// COMMIT the carried move — the GTW-356 two-click CLICK-2 rung (clause 3, OQ-5). A
    /// player-faction selection clicked a VALID move target that EQUALS the current
    /// [`PathPreviewTarget`]. Pushed as an [`ActIntent::Move`](crate::ActIntent::Move) AND CLEARS
    /// [`PathPreviewTarget`] (the route is committed, the preview is consumed); the selection
    /// is unchanged.
    Move(MoveRequested),
    /// NO-OP — does NOTHING, leaving [`SelectedShooter`] AND [`PathPreviewTarget`] untouched (no
    /// clear, no transient `None`). Three cases reach it: clause 3.5 (GTW-287), the clicked cell
    /// holds an ENEMY you cannot FIRE on (enemies are inspected via the GTW-274 hover panel, never
    /// selected or cleared as your shooter); the no-hover case (GTW-288), a click with no map cell
    /// (cursor over the UI / a margin / off the map, the GTW-286 viewport gate) — which must not
    /// clear the selection, lest it flicker the status panel and break Mode/Stance; and the
    /// GTW-356 OQ-4 LINK-TILE case, a click on a vertical-link tile, which is NOT a targeting
    /// action (no target set, no move dispatched, no active-level change — the player switches
    /// storey + clicks a destination tile, and the route auto-stitches through the link).
    NoOp,
    /// CLEAR the selection — clause 4: a valid in-grid hovered cell
    /// ([`InspectTarget::hovered`] is [`Some`]) where none of FIRE / SELECT / MOVE / NO-OP
    /// applied (an empty / blocked cell with a non-player or stale selection). Sets
    /// [`SelectedShooter::cleared`]. The no-hover case is NOT here — that is the NO-OP above
    /// (GTW-288).
    Clear,
}

/// Resolves ONE left-click edge through the FIRE → SELECT → MOVE → CLEAR precedence chain
/// (GTW-238) into a [`LeftClickOutcome`] — the SHARED decision (GTW-259) the mouse and the
/// gamepad both use.
///
/// READ-ONLY: it reads the hovered cell, the occupancy, the fire mode / tuning, the player
/// faction, the squad fog, the current selection, and the faction / firing components off the
/// queries, and returns the resolved outcome WITHOUT touching any state.
///
/// 0. **TARGETING FOG GATE (GTW-11)** — BEFORE FIRE: if the hovered cell holds an ENEMY and
///    the player selection could fire on it, but that cell is NOT squad-VISIBLE (UNSEEN *or*
///    merely EXPLORED, both refused identically), the fire commit is refused →
///    [`LeftClickOutcome::NoOp`] (armed, zero mutation). The same shared
///    [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) read the reticle + the
///    hint consume (FAIL-CLOSED on an absent fog), so the three never disagree. A MOVE into an
///    EXPLORED EMPTY cell is NOT a fire target and is unaffected (walking into remembered
///    territory stays allowed, `docs/combat/visibility.md` §"UX edges").
/// 1. **FIRE** — a fire mode is selected, AND EITHER the hovered cell holds an ENEMY occupant
///    (a [`Faction`] `!=` [`PlayerFaction`](gdtf_battle_sim::PlayerFaction)) OR (GTW-377) it holds SHOOTABLE structure (an intact
///    wall / cover piece: [`is_blocked`](gdtf_battle_sim::OccupancyGrid::is_blocked) AND no
///    occupant), the current selection is a player-faction ganger, the cell is squad-VISIBLE, and
///    the shared [`can_fire`](gdtf_battle_sim::can_fire) guard passes ([`try_fire_request`]) →
///    [`LeftClickOutcome::Fire`]. The user ruling: cover AND walls are valid fire targets, not
///    just gangers; the sim depletes the cover per the GTW-364 model. The enemy case uses the
///    occupant fog relation ([`FactionRelation::Other`]); the cover case uses `relation = None`
///    (a structural cell carries no occupant).
/// 2. **SELECT** — the hovered cell holds one of YOUR gangers
///    ([`Faction`] `==` [`PlayerFaction`](gdtf_battle_sim::PlayerFaction)) → [`LeftClickOutcome::Select`].
/// 3. **MOVE (two-click, GTW-356 OQ-5)** — there is a player-faction selection AND the hovered
///    cell is a VALID move target: empty (`occupant == None`), in-bounds (a [`Some`] hovered
///    cell is always in-grid), unblocked (`!is_blocked`), and NOT a vertical-link tile
///    ([`VerticalLinkGraph::links_from`](gdtf_battle_sim::VerticalLinkGraph::links_from)`(cell).next().is_none()`, OQ-4). It does NOT dispatch
///    immediately; it runs a state machine over the current [`PathPreviewTarget`]:
///    - the clicked cell EQUALS the current target → [`LeftClickOutcome::Move`] (COMMIT: the
///      preview was confirmed; dispatch + clear the target);
///    - the clicked cell does NOT equal the current target ([`None`] or a different cell) →
///      [`LeftClickOutcome::SetMoveTarget`] (CLICK-1 / RE-TARGET: set the target, show the
///      preview, dispatch NOTHING);
///    - a VALID-but-vertical-LINK tile is the GTW-356 OQ-4 NON-target case — it falls to
///      [`LeftClickOutcome::NoOp`] (no target set, no dispatch, no level change), NOT a target.
/// 4. **CLEAR** — a valid in-grid hovered cell (a [`Some`] [`InspectTarget::hovered`]) where none
///    of the above applied → [`LeftClickOutcome::Clear`]. The genuine "nothing to act on" cases
///    (an empty / blocked cell with a non-player or stale selection) still clear.
///
/// # The GTW-356 same-cell-commit semantics (FLAGGED)
///
/// "Click again on the SAME cell COMMITS; a different cell RE-TARGETS" is the user OQ-5
/// two-click interpretation built here — the first click is always a target-select (the
/// preview shows), a second click on that exact cell confirms the move, and a click on any
/// OTHER valid cell re-points the preview without committing. EDGE CASE resolved: a click on
/// a BLOCKED / out-of-range / vertical-link cell while a target is PENDING does NOT commit and
/// does NOT clear the pending target — it is the corresponding non-MOVE outcome (CLEAR for a
/// blocked empty cell with a stale selection, [`NoOp`](LeftClickOutcome::NoOp) for a link tile /
/// enemy / no-hover), leaving the pending target intact so the player keeps their preview.
/// (Affordability — out-of-TU — is
/// the SIM's [`dispatch_move`](gdtf_battle_sim::acts::dispatch_move) gate, not this layer's; the
/// preview's `find_path` cost surfaces it, and a commit that the sim rejects is a sim-side
/// no-op.)
///
/// A NO-HOVER click is NOT clause 4: when the hovered cell is [`None`] — a click with no map
/// cell (cursor over the bottom UI / a margin / off the map, which the GTW-286 viewport gate
/// resolves to no cell) — [`decide_left_click`] returns [`LeftClickOutcome::NoOp`] up front
/// (GTW-288), NOT [`LeftClickOutcome::Clear`]. Clearing on every over-UI click flickered the
/// status panel and broke Mode/Stance (which need the selection to resolve the weapon).
///
/// Between MOVE (clause 3) and CLEAR (clause 4) sits the GTW-287 **NO-OP** rung: when the
/// hovered cell holds an ENEMY occupant ([`Faction`] `!=` [`PlayerFaction`](gdtf_battle_sim::PlayerFaction)) you could not FIRE
/// on, [`decide_left_click`] returns [`LeftClickOutcome::NoOp`] instead of falling through to
/// CLEAR. Clicking an enemy you can't fire on leaves your current player selection UNTOUCHED
/// (enemies are inspected via the GTW-274 hover panel, not selected/cleared as your shooter) —
/// no clear, no transient `None`, no "No ganger selected" flash, no auto-select revert.
///
/// FALL-THROUGH (the user-confirmed precedence): a fire mode over an EMPTY (unblocked,
/// unoccupied) cell FAILS clause 1 (no enemy occupant) AND clause 1b (not blocked structure)
/// and falls through to clause 3 (MOVE) — the fire mode does NOT lock out move. A fire mode over
/// an intact wall / cover cell is the GTW-377 clause-1b FIRE-AT-COVER rung (it is blocked, so it
/// was never a MOVE target). An ENEMY-occupied cell you cannot fire on does NOT fall through to
/// CLEAR (it would wipe the selection); it is the GTW-287 NO-OP rung above.
///
/// Param-only (`bevy-traps.md` #7): the [`LeftClickReads`] read bundle + the [`InspectTarget`]
/// AND the [`PathPreviewTarget`] (BOTH passed as a separate `&` so the caller can hold each as a
/// `ResMut` for its write — the pin write on [`InspectTarget`], the two-click target write on
/// [`PathPreviewTarget`] — without a `Res` + `ResMut` aliasing conflict on the SAME resource
/// (B0002); the [`InspectTarget`] precedent, GTW-300) + read-only `Query<&Faction>` /
/// `Query<ShooterFireData>` / `Query<&Wields>` / the weapon-[`Magazine`](gdtf_battle_sim::Magazine)
/// query + the [`MeleeWeapon`](gdtf_battle_sim::MeleeWeapon) marker probe ([`MeleeQuery`]) + the
/// current [`SelectedShooter`], no `&mut World`. The
/// `Wields` + weapon-magazine + melee-marker queries resolve the fire guard's magazine off the
/// RANGED weapon entity (`ganger → Wields → the ranged weapon entity`, GTW-323 slice 3; GTW-505 C5
/// excludes the melee weapon the ganger also wields via [`Wields::ranged_weapon`](gdtf_battle_sim::Wields::ranged_weapon)).
/// It does NOT read the mouse button (the caller gates on its own press edge), so the gamepad surface
/// reuses it without a `ButtonInput<MouseButton>`.
#[expect(
    clippy::too_many_arguments,
    reason = "GTW-356: the two-click move state machine reads the current PathPreviewTarget, which \
              must be passed as a separate `&` (not in LeftClickReads) so the caller holds the \
              ResMut for the target write without a B0002 Res+ResMut alias — the InspectTarget \
              precedent; on top of the GTW-323 slice-3 Wields + weapon-magazine queries"
)]
#[must_use]
pub fn decide_left_click(
    reads: &LeftClickReads,
    inspect: &InspectTarget,
    move_target: &PathPreviewTarget,
    factions: &Query<&Faction>,
    shooters: &Query<ShooterFireData>,
    wields: &Query<&Wields>,
    weapons: &Query<WeaponMagazine, With<WieldedBy>>,
    melee: &MeleeQuery,
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

    // 0. TARGETING FOG GATE (GTW-11) — a NEW rung AFTER the no-hover guard and BEFORE FIRE:
    //    refuse the fire commit into a cell that is NOT squad-VISIBLE (UNSEEN *or* merely
    //    EXPLORED, both refused identically — `docs/combat/visibility.md` §"UX edges"). It is
    //    scoped to the FIRE-target shape (an ENEMY occupant + a player-faction selection) so it
    //    refuses ONLY the targeting path: walking into REMEMBERED (EXPLORED) territory stays
    //    allowed (§"UX edges": "Walking into EXPLORED ... stays allowed"), so a MOVE into an
    //    explored EMPTY cell is NOT a fire target and never reaches this rung. The verdict is the
    //    SAME shared `cell_squad_visible` read the reticle + the hint consume (FAIL-CLOSED on an
    //    absent fog), so the three can never disagree. Returns the EXISTING `NoOp` (armed, zero
    //    mutation — no fire request, no selection / target write): targeting stays armed, and the
    //    enemy is still inspected via the GTW-274 hover panel. A non-VISIBLE enemy is refused even
    //    when `can_fire` would pass (the fog gate is player policy, never inside `can_fire`,
    //    `docs/combat/resolution.md` §"What's pure math vs sim").
    if let Some(enemy_faction) = occupant_faction
        && selection_is_player
        && enemy_faction != player
        && !cell_squad_visible(
            reads.squad_visibility.as_deref(),
            &target,
            Some(FactionRelation::Other),
        )
        .is_squad_visible()
    {
        return LeftClickOutcome::NoOp; // unseen/explored enemy -> refuse the fire commit.
    }

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
            melee,
        )
    {
        return LeftClickOutcome::Fire(request); // FIRE wins this edge.
    }

    // 1b. FIRE AT COVER (GTW-377) — the hovered cell holds SHOOTABLE structure (an intact
    //     wall / cover piece: `is_blocked` is true, and there is NO occupant so it is not a
    //     ganger target) and the player selection can fire on it. The user ruling: cover AND
    //     walls are VALID fire targets, not just gangers — a fire mode + a player-faction
    //     selection + the shared `can_fire` guard fires a ray AT the cell, and the sim depletes
    //     the cover per the GTW-364 model. The fog gate uses `relation = None` (a structural
    //     cell carries no occupant) — you can only shoot cover the squad can currently SEE, the
    //     SAME `cell_squad_visible` read the reticle + the enemy-fire rung share (FAIL-CLOSED on
    //     an absent fog). Ordered AFTER the enemy-FIRE rung (an occupant always wins the cell)
    //     and BEFORE SELECT/MOVE (a blocked cell is never a SELECT/MOVE target anyway).
    if let Some(shooter) = **selected
        && selection_is_player
        && occupant.is_none()
        && reads.occupancy.is_blocked(&target)
        && cell_squad_visible(reads.squad_visibility.as_deref(), &target, None).is_squad_visible()
        && let Some(request) = try_fire_request(
            shooter,
            target,
            &reads.fire_mode,
            &reads.tuning,
            shooters,
            wields,
            weapons,
            melee,
        )
    {
        return LeftClickOutcome::Fire(request); // FIRE AT COVER wins this edge.
    }

    // 2. SELECT — the hovered cell holds one of YOUR gangers.
    if let (Some(entity), Some(faction)) = (occupant, occupant_faction)
        && faction == player
    {
        return LeftClickOutcome::Select(entity); // SELECT wins — no act emitted.
    }

    // 3. MOVE (two-click, GTW-356) — a player-faction selection + an empty, in-bounds,
    //    unblocked hovered cell.
    if let Some(actor) = **selected
        && selection_is_player
        && occupant.is_none()
        && !reads.occupancy.is_blocked(&target)
    {
        // OQ-4: a vertical-link tile is NOT a move target — clicking it is a no-op (no target
        // set, no dispatch, no active-level change). Reaching another storey is "switch
        // ActiveLevel + click a destination tile there"; the route auto-stitches the link.
        if reads.links.links_from(&target).next().is_some() {
            return LeftClickOutcome::NoOp; // link tile -> non-target, leave everything as-is.
        }
        // OQ-5 two-click state machine over the current PathPreviewTarget: a click on the SAME
        // cell COMMITS the move; a click on a DIFFERENT (or no) target (RE-)TARGETS it.
        return if **move_target == Some(target) {
            LeftClickOutcome::Move(MoveRequested::new(actor, target)) // click-2 -> COMMIT.
        } else {
            LeftClickOutcome::SetMoveTarget(target) // click-1 / re-target -> SET the preview.
        };
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
