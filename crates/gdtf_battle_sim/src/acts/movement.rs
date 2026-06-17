//! The **move** dispatch — drain each buffered [`MoveRequested`] message and run the
//! landed [`move_ganger`] verb once per message (E4 / GTW-234).
//!
//! No act logic is reimplemented: the system REUSES the landed
//! [`move_ganger`](crate::move_acts::move_ganger) verb verbatim (whose gates + the
//! destination-terrain TU cost hold end-to-end) and fetches the actor's components via a
//! Bevy query (`bevy-traps.md` #7 — no `&mut World`); it only READS the grid.

use bevy::prelude::{MessageReader, Query, Res};

use crate::{
    acts::request::MoveRequested,
    ganger::{LifeState, Position, Tu},
    move_acts::move_ganger,
    occupancy::OccupancyGrid,
    tuning::CombatTuning,
};

/// **Dispatch** buffered [`MoveRequested`] messages — drain each and run the landed
/// [`move_ganger`] verb once per message (E4 / GTW-234).
///
/// Queries the actor's `(&mut `[`Position`]`, &mut `[`Tu`]`, &`[`LifeState`]`)`, reads
/// the [`OccupancyGrid`] (for the gates AND the destination-terrain cost lookup) and the
/// [`CombatTuning`] resource (for the [`MoveCosts`](crate::tuning::MoveCosts) table), and
/// calls [`move_ganger`] — whose gates (liveness / in-bounds / not-blocked / unoccupied /
/// affordable) hold end-to-end, charging the DESTINATION terrain's move cost. REUSES the
/// landed verb verbatim. A message for an actor missing any queried component is skipped
/// (fail-closed, no panic — the `dispatch_set_*` precedent).
///
/// This dispatch ONLY READS the grid (`Res<OccupancyGrid>`) for the gates + the terrain
/// cost; it never writes it. The grid's slot maintenance is the landed
/// [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers) reacting to the
/// `Changed<`[`Position`]`>` this verb produces — both sit in the
/// [`SimSystems::Simulate`](crate::occupancy_sync::SimSystems::Simulate) set and compose
/// with no ambiguity (`dispatch_move`'s `&mut Position` writes, `sync_moved_gangers`'s
/// `&Position` reads it next).
pub fn dispatch_move(
    mut requests: MessageReader<MoveRequested>,
    mut actors: Query<(&'static mut Position, &'static mut Tu, &'static LifeState)>,
    grid: Res<OccupancyGrid>,
    tuning: Res<CombatTuning>,
) {
    for request in requests.read() {
        let Ok((mut position, mut tu, &life)) = actors.get_mut(request.actor) else {
            continue;
        };
        let _outcome = move_ganger(
            &mut position,
            &mut tu,
            life,
            request.dest,
            &grid,
            &tuning.move_costs,
        );
    }
}
