//! The wiring unit — [`SimSystems`] (the public ordering anchor) and
//! [`OccupancyMaintenancePlugin`] (registers the [`CoverDestroyed`] buffer + the
//! three chained maintenance systems under [`SimSystems::Simulate`]).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, SystemSet, Update};

use crate::{
    occupancy::{project_path_blocking, project_vision_blocking},
    occupancy_sync::{
        CoverDestroyed, GroundAccrued, SlabDestroyed, sync_accrued_ground, sync_dead_gangers,
        sync_destroyed_cover, sync_destroyed_slab, sync_moved_gangers,
    },
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
    /// The band containing the act-log RECORDER (GTW-727) — the read-only pass that writes
    /// this tick's acts into the [`ActLog`](crate::act_log::ActLog), ordered strictly
    /// `.after(`[`Simulate`](SimSystems::Simulate)`)` so it observes a settled world. It
    /// mutates nothing but the log, so a view can order `.after(SimSystems::Record)` and be
    /// guaranteed that both the world AND its record of how the world got there are final
    /// for the tick.
    Record,
}

/// Wires the three change-driven occupancy-maintenance systems and the
/// [`CoverDestroyed`] message buffer into a Bevy [`App`].
///
/// This is the **registration unit** for the E1.7 maintenance layer:
/// - it registers the [`CoverDestroyed`] + [`SlabDestroyed`] + [`GroundAccrued`] message
///   buffers ([`App::add_message`]), without which
///   [`sync_destroyed_cover`](crate::occupancy_sync::sync_destroyed_cover)'s /
///   [`sync_destroyed_slab`](crate::occupancy_sync::sync_destroyed_slab)'s /
///   [`sync_accrued_ground`](crate::occupancy_sync::sync_accrued_ground)'s
///   [`MessageReader`](bevy::prelude::MessageReader) would fail param validation; and
/// - it adds [`sync_moved_gangers`](crate::occupancy_sync::sync_moved_gangers),
///   [`sync_dead_gangers`](crate::occupancy_sync::sync_dead_gangers),
///   [`sync_destroyed_cover`](crate::occupancy_sync::sync_destroyed_cover), and
///   [`project_path_blocking`](crate::occupancy::project_path_blocking) (GTW-501) to
///   [`Update`] in an **explicit
///   [`chain`](bevy::prelude::IntoScheduleConfigs::chain) order** — all four
///   share `ResMut<`[`OccupancyGrid`](crate::occupancy::OccupancyGrid)`>`, so they
///   MUST be ordered deterministically
///   (`bevy-traps.md` #3). The chain order (move → die → cover → PROJECT) is deliberate:
///   moves settle each entity's slot first, deaths then free a settled slot, cover folds
///   into the independent destroyed-cover set, and the tag-derived path-blocking surface is
///   re-synced LAST (so the pathfinding consumers, ordered `.after(project_path_blocking)`,
///   read the up-to-date surface this frame); and
/// - it adds [`sync_destroyed_slab`](crate::occupancy_sync::sync_destroyed_slab)
///   (GTW-365) to the same [`SimSystems::Simulate`] set. It writes a DIFFERENT resource
///   (`ResMut<`[`SurfaceGrid`](crate::surface::SurfaceGrid)`>`), so it needs no
///   ordering against the `OccupancyGrid` chain (disjoint access — no conflict), but it
///   sits in the same set so the squad-fog recompute (ordered `.after` the set) sees a
///   destroyed slab's surface edit before it re-reveals the opened sightline; and
/// - it adds [`sync_accrued_ground`](crate::occupancy_sync::sync_accrued_ground)
///   (GTW-366) to the same [`SimSystems::Simulate`] set, chained
///   `.after(sync_destroyed_slab)` because both write `ResMut<`[`SurfaceGrid`](crate::surface::SurfaceGrid)`>`
///   (different inner maps — slab state vs the ground accumulator — but Bevy treats them as
///   one resource access, so an explicit order is required, `bevy-traps.md` #3). The accrual
///   is purely cosmetic (it touches no slab / occupancy / visibility state), so NO recompute
///   is ordered after it.
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
            // GTW-365: the slab-destroyed signal `sync_destroyed_slab` reads. Registering
            // the buffer here makes its MessageReader<SlabDestroyed> param valid; it is
            // IDEMPOTENT with SimActsPlugin's own add_message::<SlabDestroyed> (the producer
            // side — Bevy's add_message no-ops a second registration), so both consumer
            // (here) and producer (dispatch_fire) plugins can name it.
            .add_message::<SlabDestroyed>()
            // GTW-366: the ground-accrued signal `sync_accrued_ground` reads. Registering
            // the buffer here makes its MessageReader<GroundAccrued> param valid; it is
            // IDEMPOTENT with SimActsPlugin's own add_message::<GroundAccrued> (the producer
            // side — Bevy's add_message no-ops a second registration), so both consumer
            // (here) and producer (dispatch_fire) plugins can name it.
            .add_message::<GroundAccrued>()
            .configure_sets(Update, SimSystems::Simulate)
            // GTW-727 C12: the act-log recorder band, ordered strictly after the mutation
            // band so `record_acts` reads a SETTLED world — every after-value it records
            // (position, posture, vitals, magazine, life state) is the value the tick
            // actually reached. Configured here because this plugin OWNS the `SimSystems`
            // anchor; the band's live-battle gate is configured separately beside
            // `Simulate`'s in `BattleSimPlugin` (a sibling set variant inherits neither the
            // ordering nor the run condition of its siblings).
            .configure_sets(Update, SimSystems::Record.after(SimSystems::Simulate))
            // GTW-501 C3 / GTW-502 C4: `project_path_blocking` then `project_vision_blocking`
            // are APPENDED to the OccupancyGrid chain (move → die → cover → PATH → VISION).
            // BOTH take `ResMut<OccupancyGrid>`, so they MUST be ordered against the three
            // that share it AND each other (`bevy-traps.md` #3); chaining them LAST means the
            // tag-derived path + vision surfaces are re-synced after the frame's
            // occupant/destroyed-cover maintenance settles, and BEFORE their consumers — the
            // pathfinding ones (`dispatch_move` / `advance_walk` / `enemy_ai_turn` ordered
            // `.after(project_path_blocking)` in SimActsPlugin) and the vision one
            // (`recompute_visibility` ordered `.after(project_vision_blocking)` in
            // BattleSimPlugin). Each drains its `RemovedComponents` reader every run (the
            // special-SystemParam contract, never under a skipping `run_if`) and applies its
            // `Added`/`Changed` inserts (incl. the setup-spawn insert, visible this tick
            // because setup runs `.before(SimSystems::Simulate)`). The two surfaces are
            // INDEPENDENT (GTW-502 C7) — path and vision do not cross — but they share the one
            // `OccupancyGrid` resource, so the chain just orders the writes deterministically.
            .add_systems(
                Update,
                (
                    sync_moved_gangers,
                    sync_dead_gangers,
                    sync_destroyed_cover,
                    project_path_blocking,
                    project_vision_blocking,
                )
                    .chain()
                    .in_set(SimSystems::Simulate),
            )
            // GTW-365: the slab-surface maintenance system. It writes ResMut<SurfaceGrid>
            // (disjoint from the OccupancyGrid chain above — no shared-resource conflict),
            // so it joins the same set WITHOUT chaining into the move → die → cover order.
            // The squad-fog recompute is ordered `.after(sync_destroyed_slab)` in
            // BattleSimPlugin so a freed slab's surface edit lands before sight recomputes.
            .add_systems(Update, sync_destroyed_slab.in_set(SimSystems::Simulate))
            // GTW-366: the ground-accrual maintenance system. It also writes
            // ResMut<SurfaceGrid> (the per-cell ground accumulator — a DIFFERENT map than
            // the slab state `sync_destroyed_slab` writes). Two systems sharing
            // ResMut<SurfaceGrid> must be ordered deterministically (`bevy-traps.md` #3), so
            // it is chained `.after(sync_destroyed_slab)`. The accrual is purely cosmetic
            // (it touches no slab state / occupancy / visibility), so NO recompute follows it.
            .add_systems(
                Update,
                sync_accrued_ground
                    .after(sync_destroyed_slab)
                    .in_set(SimSystems::Simulate),
            );
    }
}
