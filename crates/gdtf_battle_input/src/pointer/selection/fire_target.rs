//! The fire-target highlight seam + POPULATE system (GTW-371 · C2; GTW-377): the input-crate
//! half of the hover-on-a-fireable-target affordance.
//!
//! The presenter owns the [`FireTargetHighlight`] read-seam + the draw system; this module
//! POPULATES it for the SELECTED shooter hovering a FIREABLE TARGET (an ENEMY *or*, since
//! GTW-377, a shootable COVER / WALL cell). It is the ONLY place [`SelectedShooter`] +
//! [`SelectedFireMode`] + the hovered cell feed the fire-target affordance — keeping selection /
//! fire-mode / hover out of the authoritative sim model (the `input → presenter → sim`
//! direction; the presenter DEFINES the resource, this input crate WRITES it, the
//! [`HighlightRequest`](gdtf_battle_presenter::HighlightRequest) precedent).
//!
//! # The fireable-target verdict (mirrors `decide_left_click`'s FIRE + FIRE-AT-COVER rungs)
//!
//! The hover is a fire target iff there IS a [`SelectedShooter`] that is a PLAYER-faction ganger,
//! AND the hovered cell is EITHER (the SAME conditions
//! [`decide_left_click`](crate::selection::decide_left_click) gates the FIRE branches on):
//!
//! - a fireable ENEMY — the cell holds an OCCUPANT whose [`Faction`] differs from the player's,
//!   and the cell is squad-VISIBLE under the occupant relation
//!   ([`FactionRelation::Other`]); or
//! - shootable COVER / WALL (GTW-377) — the cell holds NO occupant but DOES block
//!   ([`is_blocked`](gdtf_battle_sim::occupancy::OccupancyGrid::is_blocked) — an intact wall / cover piece,
//!   `false` once GTW-364 has smashed it), and the cell is squad-VISIBLE under the empty-cell
//!   relation (`None`).
//!
//! Both branches share the GTW-346 / GTW-11 targeting-fog gate — the SAME shared
//! [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) read the reticle + the
//! fire-refusal consume (FAIL-CLOSED on an absent fog), so the affordance and the fire commit
//! never disagree.
//!
//! When the verdict holds, it computes the fire TU cost the same way the shot will charge it —
//! [`mode_tu_cost`](gdtf_battle_sim::magazine::mode_tu_cost) over the [`SelectedFireMode`] + the shooter's
//! [`TuMax`] / [`Aiming`] + the [`CombatTuning`] — and writes [`FireTargetHighlight::new`].
//! Otherwise (no selection, an empty / own-ganger / non-visible cell, no hover) it writes
//! [`FireTargetHighlight::cleared`] (the empty highlight). REUSES the sim's `mode_tu_cost` — no
//! re-implemented cost.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::{FireTargetHighlight, cell_squad_visible};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Aiming, TuMax},
    magazine::mode_tu_cost,
    prelude::{CellLevel, Faction, OccupancyGrid, Tu},
    tuning::CombatTuning,
    visibility::{FactionRelation, SquadVisibility},
};

use crate::{InspectTarget, SelectedFireMode, selection::resources::SelectedShooter};

/// The read-only resources [`populate_fire_target`] consults, grouped into ONE [`SystemParam`]
/// so the system's parameter list stays under clippy's argument-count gate (the
/// [`LeftClickReads`](crate::LeftClickReads) / [`PreviewGrids`](crate::PreviewGrids) precedent).
///
/// A transparent system-param bundle of framework resources + landed newtypes — not itself a
/// wrapped domain scalar.
#[derive(SystemParam)]
pub struct FireTargetReads<'w> {
    /// The selected shooter — the FIRE-rung subject (must be a player-faction ganger).
    selected:  Res<'w, SelectedShooter>,
    /// The selected fire mode — the `mode_tu_cost` mode.
    fire_mode: Res<'w, SelectedFireMode>,
    /// The inspect target — its LIVE hovered cell is the candidate fire target.
    inspect:   Res<'w, InspectTarget>,
    /// The coarse occupancy grid — the occupant lookup at the hovered cell.
    occupancy: Res<'w, OccupancyGrid>,
    /// The player's own faction — the friend/foe gate (the occupant must be an ENEMY).
    player:    Res<'w, PlayerFaction>,
    /// The combat tuning — the aim TU premium `mode_tu_cost` reads.
    tuning:    Res<'w, CombatTuning>,
    /// The squad fog — the GTW-346 / GTW-11 targeting-fog gate (read as `Option` so the verdict
    /// FAILS CLOSED when the fog is absent, the same fail-closed `cell_squad_visible` the
    /// reticle + fire-refusal use).
    squad:     Option<Res<'w, SquadVisibility>>,
}

/// `Update` ([`InputSystems::Gather`](crate::InputSystems)): POPULATE the presenter-owned
/// [`FireTargetHighlight`] when the SELECTED player-faction shooter hovers a squad-VISIBLE
/// fireable TARGET it could fire on — an ENEMY *or* (GTW-377) a shootable COVER / WALL cell —
/// the cell + the [`mode_tu_cost`](gdtf_battle_sim::magazine::mode_tu_cost) the shot would charge (C2).
///
/// Resolves the fireable verdict via [`resolve_fire_target`] (the SAME FIRE + FIRE-AT-COVER
/// conditions [`decide_left_click`](crate::selection::decide_left_click) gates fire on, plus the
/// GTW-346 fog gate): a player-faction selection, AND either an ENEMY occupant or intact
/// blocking cover/wall at the hovered cell, that cell squad-VISIBLE. When it resolves it reads
/// the shooter's `(`[`TuMax`]`, `[`Aiming`]`)` and
/// computes the cost; otherwise it clears the highlight. Writes only on a real CHANGE (the `!=`
/// guard) so an unchanged hover / selection does not spuriously trip
/// `Changed<FireTargetHighlight>`. REUSES `mode_tu_cost` — no re-implemented cost; selection /
/// fire-mode / hover NEVER pushed into the sim. Param-only (`bevy-traps.md` #7).
pub fn populate_fire_target(
    reads: FireTargetReads,
    factions: Query<&Faction>,
    shooters: Query<(&TuMax, &Aiming)>,
    mut highlight: ResMut<FireTargetHighlight>,
) {
    let next = resolve_fire_target(&reads, &factions, &shooters);

    // Mutate only on a real change (change-detection hygiene).
    if *highlight != next {
        *highlight = next;
    }
}

/// Resolve the [`FireTargetHighlight`] for the current selection + hover — the pure fireable
/// verdict the system applies (mirrors `decide_left_click`'s FIRE + FIRE-AT-COVER rungs + the
/// GTW-346 fog gate).
///
/// Returns [`FireTargetHighlight::new`]`(cell, cost)` when the hovered cell is a fireable target —
/// EITHER a fireable ENEMY (player-faction selection, ENEMY occupant, squad-VISIBLE) OR a
/// shootable COVER / WALL cell (GTW-377: player-faction selection, intact blocking structure with
/// no occupant, squad-VISIBLE) — and the shooter's TU stats resolve; otherwise
/// [`FireTargetHighlight::cleared`].
fn resolve_fire_target(
    reads: &FireTargetReads,
    factions: &Query<&Faction>,
    shooters: &Query<(&TuMax, &Aiming)>,
) -> FireTargetHighlight {
    let player: Faction = **reads.player;
    // No hovered map cell (over the UI / a margin / off the map) → no fire target.
    let Some(cell) = reads.inspect.hovered() else {
        return FireTargetHighlight::cleared();
    };
    // The current selection must be a PLAYER-faction ganger (the FIRE-rung subject gate).
    let Some(shooter) = **reads.selected else {
        return FireTargetHighlight::cleared();
    };
    let selection_is_player = factions.get(shooter).copied().is_ok_and(|f| f == player);
    if !selection_is_player {
        return FireTargetHighlight::cleared();
    }
    // The hovered cell is a fire target iff it holds a fireable ENEMY *or* shootable COVER —
    // both gated on squad-visibility (the SAME shared read the reticle + fire-refusal use).
    if !cell_is_fireable_target(reads, factions, &cell, player) {
        return FireTargetHighlight::cleared();
    }
    // Fireable: compute the fire TU cost the same way the shot will charge it (REUSE).
    match fire_cost(shooter, &reads.fire_mode, &reads.tuning, shooters) {
        Some(cost) => FireTargetHighlight::new(cell, cost),
        // The shooter has no TU stats (unarmed / incomplete) → no cost → no highlight.
        None => FireTargetHighlight::cleared(),
    }
}

/// Whether `cell` is a fireable TARGET for the player — a fireable ENEMY occupant OR shootable
/// COVER / WALL structure, each gated on the GTW-346 / GTW-11 squad-visibility fog
/// (FAIL-CLOSED on an absent fog) so the affordance and the fire commit never disagree.
///
/// - A fireable ENEMY: the cell holds an occupant whose [`Faction`] differs from the player's,
///   and the cell is squad-VISIBLE under the occupant relation
///   ([`FactionRelation::Other`]) — exactly `decide_left_click`'s FIRE rung.
/// - Shootable COVER / WALL (GTW-377): the cell holds NO occupant but DOES block
///   ([`is_blocked`](gdtf_battle_sim::occupancy::OccupancyGrid::is_blocked) — an intact wall / cover piece,
///   `false` once GTW-364 has smashed it), and the cell is squad-VISIBLE under the empty-cell
///   relation (`None`, a structural cell carries no occupant) — `decide_left_click`'s
///   FIRE-AT-COVER rung. The user ruling: cover AND walls are valid fire targets.
fn cell_is_fireable_target(
    reads: &FireTargetReads,
    factions: &Query<&Faction>,
    cell: &CellLevel,
    player: Faction,
) -> bool {
    match reads.occupancy.occupant(cell) {
        // An occupant cell is a fire target iff the occupant is an ENEMY on a squad-VISIBLE cell.
        Some(occupant) => {
            let is_enemy = factions
                .get(occupant)
                .copied()
                .is_ok_and(|faction| faction != player);
            is_enemy
                && cell_squad_visible(reads.squad.as_deref(), cell, Some(FactionRelation::Other))
                    .is_squad_visible()
        }
        // An UNOCCUPIED cell is a fire target iff it is intact blocking structure (cover / wall)
        // on a squad-VISIBLE cell (GTW-377). A bare floor cell (`!is_blocked`) is NOT a target.
        None => {
            *reads.occupancy.is_blocked(cell)
                && cell_squad_visible(reads.squad.as_deref(), cell, None).is_squad_visible()
        }
    }
}

/// The fire TU cost for `shooter` firing `fire_mode` — [`mode_tu_cost`](gdtf_battle_sim::magazine::mode_tu_cost)
/// over the shooter's `(`[`TuMax`]`, `[`Aiming`]`)` + the tuning, or [`None`] when the shooter
/// has no TU stats (fail-closed).
fn fire_cost(
    shooter: Entity,
    fire_mode: &SelectedFireMode,
    tuning: &CombatTuning,
    shooters: &Query<(&TuMax, &Aiming)>,
) -> Option<Tu> {
    let (tu_max, aiming) = shooters.get(shooter).ok()?;
    Some(mode_tu_cost(fire_mode, tu_max, aiming, tuning))
}
