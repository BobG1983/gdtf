//! The orthogonal GTW-300 inspect-panel pin effect: its outcome vocabulary, decide, and apply.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::Faction;

use super::reads::LeftClickReads;
use crate::InspectTarget;

/// The PARALLEL inspect-panel pin effect of one left-click edge — ORTHOGONAL to the
/// [`LeftClickOutcome`](super::LeftClickOutcome) selection/act effect (GTW-300).
///
/// A SECOND domain enum, NOT folded into [`LeftClickOutcome`](super::LeftClickOutcome), because the pin is a VIEW
/// concern that must COEXIST with the selection/act effect: clicking your own ganger both
/// SELECTS it AND keeps the pin; clicking an empty tile both CLEARS the selection AND unpins.
/// One enum can't carry both effects without a compound case per combination — keeping them as
/// two orthogonal outcomes (decided together, applied together) is the "composes correctly,
/// no double-dispatch" the contract names: each effect is committed exactly once, on its own
/// resource. [`decide_pin`] computes it alongside [`decide_left_click`](super::decide_left_click); [`apply_pin`] commits
/// it to the [`InspectTarget`] right after [`apply_left_click`](super::apply_left_click).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinOutcome {
    /// PIN the inspect panel to the carried cell — the click landed on COVER (a wall / cover
    /// cell) or an ENEMY fighter. The panel freezes on that cell's occupant / terrain until an
    /// [`Unpin`](PinOutcome::Unpin).
    Pin(gdtf_battle_sim::metric::CellLevel),
    /// UNPIN the inspect panel — the click landed on an EMPTY in-grid tile; hover resumes.
    Unpin,
    /// KEEP the current pin untouched — the click was on your OWN ganger (a SELECT), a FIRE on
    /// an enemy, or a no-hover click (over the UI / a margin / off the map). The pin is a
    /// separate view state; these clicks leave it exactly as it was.
    Keep,
}

/// Resolves ONE left-click edge into its PARALLEL inspect-panel [`PinOutcome`] (GTW-300) — the
/// SHARED pin decision the mouse and the gamepad both use, ORTHOGONAL to [`decide_left_click`](super::decide_left_click).
///
/// READ-ONLY: it reads the LIVE hovered cell (where you clicked), the occupant + its faction at
/// that cell, and the terrain, and returns the resolved pin effect WITHOUT touching any state.
/// Computed ALONGSIDE [`decide_left_click`](super::decide_left_click) so a single click yields both effects; they are
/// committed by [`apply_pin`] / [`apply_left_click`](super::apply_left_click) independently (the contract's "composes
/// correctly"). The pin precedence (each tested against the LIVE hovered cell):
///
/// 1. **PIN to cover** — the cell's [`TerrainKind`](gdtf_battle_sim::occupancy::TerrainKind) is a wall /
///    cover ([`is_blocked`](gdtf_battle_sim::occupancy::OccupancyGrid::is_blocked)) → [`PinOutcome::Pin`].
/// 2. **PIN to an enemy** — the cell holds an occupant whose [`Faction`] differs from the
///    [`PlayerFaction`](gdtf_battle_sim::battle::PlayerFaction) → [`PinOutcome::Pin`]. (So clicking an enemy pins it whether or not the
///    click ALSO fires — the FIRE branch in [`decide_left_click`](super::decide_left_click) is independent.)
/// 3. **KEEP** — the cell holds one of YOUR OWN gangers (a SELECT) → [`PinOutcome::Keep`]
///    (selecting your ganger never disturbs the pin).
/// 4. **UNPIN** — an EMPTY in-grid tile (no enemy, no cover, no own ganger) → [`PinOutcome::Unpin`]
///    (hover resumes normally).
///
/// A NO-HOVER click (the live hovered cell is [`None`] — over the UI / a margin / off the map,
/// the GTW-286 viewport gate) → [`PinOutcome::Keep`]: a click that hit no map cell must never
/// disturb the pin (mirrors [`decide_left_click`](super::decide_left_click)'s NO-OP for the same case, GTW-288).
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
    if *reads.occupancy.is_blocked(&cell) {
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

/// Commits a [`decide_pin`] [`PinOutcome`] to the [`InspectTarget`]'s pin (GTW-300) — the SHARED
/// pin apply step the mouse and gamepad both run, ORTHOGONAL to [`apply_left_click`](super::apply_left_click).
///
/// [`Pin`](PinOutcome::Pin) pins the cell ([`set_pinned`](InspectTarget::set_pinned));
/// [`Unpin`](PinOutcome::Unpin) clears it ([`clear_pin`](InspectTarget::clear_pin));
/// [`Keep`](PinOutcome::Keep) does NOTHING, leaving the pin exactly as it was. It writes ONLY the
/// pin — never the live hovered cell — so the cursor keeps tracking under a pin. Because pin and
/// selection live on DIFFERENT resources ([`InspectTarget`] vs
/// [`SelectedShooter`](crate::SelectedShooter) / [`PendingActIntent`](crate::PendingActIntent)), this commit and
/// [`apply_left_click`](super::apply_left_click) never conflict: each click's two effects are applied once each, no
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
