//! The wiring unit — [`SimSystems`] (the public ordering anchor) and
//! [`OccupancyMaintenancePlugin`] (registers the [`CoverDestroyed`] buffer + the
//! three chained maintenance systems under [`SimSystems::Simulate`]).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, SystemSet, Update};

use crate::occupancy_sync::{
    CoverDestroyed, sync_dead_gangers, sync_destroyed_cover, sync_moved_gangers,
};

/// The sim's public system-ordering anchor — the band every sim-mutation system runs in.
///
/// [`Simulate`](SimSystems::Simulate) names the `Update`-schedule band that holds the
/// authoritative sim's world mutations, so the wiring around it can be expressed against
/// a stable, public name instead of reaching into individual system identifiers
/// (`bevy-traps.md` #3 — explicit ordering via named sets). It is the SOLE owner of this
/// anchor: later E10 slices register their per-act dispatch systems `.in_set(SimSystems::Simulate)`,
/// and a downstream reader/presenter can order `.after(SimSystems::Simulate)` for reliable
/// `Changed<T>` observation — all without naming concrete sim systems. The derive set
/// mirrors the landed `gdtf_ui` precedent (`UiSystems` / `FocusNavSystems`) so it is a
/// hashable, copyable `SystemSet` reachable from downstream crates.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimSystems {
    /// The band containing the authoritative sim's world-mutation systems — today the
    /// change-driven occupancy-maintenance chain, tomorrow the per-act dispatch systems.
    Simulate,
}

/// Wires the three change-driven occupancy-maintenance systems and the
/// [`CoverDestroyed`] message buffer into a Bevy [`App`].
///
/// This is the **registration unit** for the E1.7 maintenance layer:
/// - it registers the [`CoverDestroyed`] message buffer
///   ([`App::add_message`]), without which
///   [`sync_destroyed_cover`](crate::occupancy_sync::sync_destroyed_cover)'s
///   [`MessageReader`](bevy::prelude::MessageReader) would fail param validation; and
/// - it adds [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers),
///   [`sync_dead_gangers`](crate::occupancy_sync::sync_dead_gangers), and
///   [`sync_destroyed_cover`](crate::occupancy_sync::sync_destroyed_cover) to
///   [`Update`] in an **explicit
///   [`chain`](bevy::prelude::IntoScheduleConfigs::chain) order** — the three
///   share `ResMut<`[`OccupancyGrid`](crate::occupancy::OccupancyGrid)`>`, so they
///   MUST be ordered deterministically
///   (`bevy-traps.md` #3). The chain order (move → die → cover) is deliberate:
///   moves settle each entity's slot first, deaths then free a settled slot, and
///   cover folds into the independent destroyed-cover set last.
///
/// The chain is nested under the public [`SimSystems::Simulate`] set: the plugin
/// [`configure_sets`](App::configure_sets) that set on [`Update`] ONCE, before its
/// [`add_systems`](App::add_systems) (`bevy-traps.md` #5 — `configure_sets` precedes
/// `.in_set`), and applies `.in_set(SimSystems::Simulate)` to the chain while RETAINING
/// `.chain()` (`.in_set` composes with `.chain()`). This is purely additive: same three
/// systems, same `Update` schedule, same deterministic chain order, now reachable as a
/// named ordering band by downstream slices.
///
/// The production app adds this plugin when the sim is wired into the runtime
/// (E1.8 / E5) — that app-wiring is **out of scope** for GTW-157, so this plugin
/// is the registration the headless tests exercise (it is NOT unwired dead code).
#[derive(Debug, Default, Clone, Copy)]
pub struct OccupancyMaintenancePlugin;

impl Plugin for OccupancyMaintenancePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CoverDestroyed>()
            .configure_sets(Update, SimSystems::Simulate)
            .add_systems(
                Update,
                (sync_moved_gangers, sync_dead_gangers, sync_destroyed_cover)
                    .chain()
                    .in_set(SimSystems::Simulate),
            );
    }
}
