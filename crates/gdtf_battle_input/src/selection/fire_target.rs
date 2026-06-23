//! The fire-target highlight seam + POPULATE system (GTW-371 · C2): the input-crate half of
//! the hover-on-a-fireable-enemy targeting affordance.
//!
//! The presenter owns the [`FireTargetHighlight`] read-seam + the draw system; this module
//! POPULATES it for the SELECTED shooter hovering a FIREABLE ENEMY. It is the ONLY place
//! [`SelectedShooter`] + [`SelectedFireMode`] + the hovered cell feed the fire-target
//! affordance — keeping selection / fire-mode / hover out of the authoritative sim model (the
//! `input → presenter → sim` direction; the presenter DEFINES the resource, this input crate
//! WRITES it, the [`HighlightRequest`](gdtf_battle_presenter::HighlightRequest) precedent).
//!
//! # The fireable-enemy verdict (mirrors `decide_left_click`'s FIRE rung)
//!
//! The hover is a fire target iff (the SAME conditions
//! [`decide_left_click`](crate::selection::decide_left_click) gates the FIRE branch on):
//!
//! - there IS a [`SelectedShooter`], and it is a PLAYER-faction ganger;
//! - the hovered cell holds an OCCUPANT whose [`Faction`] differs from the player's (an ENEMY);
//! - that cell is squad-VISIBLE — the GTW-346 / GTW-11 targeting-fog gate, the SAME shared
//!   [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) read the reticle + the
//!   fire-refusal consume (FAIL-CLOSED on an absent fog), so the affordance and the fire commit
//!   never disagree.
//!
//! When all hold, it computes the fire TU cost the same way the shot will charge it —
//! [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) over the [`SelectedFireMode`] + the shooter's
//! [`TuMax`] / [`Aiming`] + the [`CombatTuning`] — and writes [`FireTargetHighlight::new`].
//! Otherwise (no selection, an empty / own-ganger / non-visible cell, no hover) it writes
//! [`FireTargetHighlight::cleared`] (the empty highlight). REUSES the sim's `mode_tu_cost` — no
//! re-implemented cost.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::{FireTargetHighlight, cell_squad_visible};
use gdtf_battle_sim::{
    Aiming, CombatTuning, Faction, FactionRelation, OccupancyGrid, PlayerFaction, SquadVisibility,
    Tu, TuMax, mode_tu_cost,
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
/// ENEMY it could fire on — the cell + the [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost) the
/// shot would charge (C2).
///
/// Resolves the fireable verdict via [`resolve_fire_target`] (the SAME FIRE-rung conditions
/// [`decide_left_click`](crate::selection::decide_left_click) gates fire on, plus the GTW-346
/// fog gate): a player-faction selection, an ENEMY occupant at the hovered cell, that cell
/// squad-VISIBLE. When it resolves it reads the shooter's `(`[`TuMax`]`, `[`Aiming`]`)` and
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
/// verdict the system applies (mirrors `decide_left_click`'s FIRE rung + the GTW-346 fog gate).
///
/// Returns [`FireTargetHighlight::new`]`(cell, cost)` when the hovered cell is a fireable enemy
/// (player-faction selection, ENEMY occupant, squad-VISIBLE) and the shooter's TU stats resolve;
/// otherwise [`FireTargetHighlight::cleared`].
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
    // The hovered cell must hold an ENEMY occupant (a faction != the player's).
    let Some(occupant) = reads.occupancy.occupant(&cell) else {
        return FireTargetHighlight::cleared();
    };
    let Ok(occupant_faction) = factions.get(occupant).copied() else {
        return FireTargetHighlight::cleared();
    };
    if occupant_faction == player {
        return FireTargetHighlight::cleared();
    }
    // GTW-346 / GTW-11 fog gate: the enemy cell must be squad-VISIBLE (FAIL-CLOSED on absent
    // fog) — the SAME shared read the reticle + the fire-refusal use, so they never disagree.
    if !cell_squad_visible(reads.squad.as_deref(), &cell, Some(FactionRelation::Other))
        .is_squad_visible()
    {
        return FireTargetHighlight::cleared();
    }
    // Fireable: compute the fire TU cost the same way the shot will charge it (REUSE).
    match fire_cost(shooter, &reads.fire_mode, &reads.tuning, shooters) {
        Some(cost) => FireTargetHighlight::new(cell, cost),
        // The shooter has no TU stats (unarmed / incomplete) → no cost → no highlight.
        None => FireTargetHighlight::cleared(),
    }
}

/// The fire TU cost for `shooter` firing `fire_mode` — [`mode_tu_cost`](gdtf_battle_sim::mode_tu_cost)
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
