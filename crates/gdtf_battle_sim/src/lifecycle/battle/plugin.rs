//! The sim's battle-lifecycle registration unit [`BattleSimPlugin`] — adding it wires
//! the WHOLE render-free combat runtime into a Bevy [`App`] (E10.5 / GTW-207 / GTW-237).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update, resource_exists};

use crate::{
    acts::SimActsPlugin,
    battle::{
        messages::{
            BattleLost, BattleReady, BattleWon, SetupBattleRequested, TeardownBattleRequested,
        },
        outcome::check_outcome,
        resources::BattleInProgress,
        setup::{setup_battle_on_request, teardown_battle_on_request},
    },
    ganger::{rederive_stats_on_injury_change, rederive_stats_on_tuning_change},
    move_acts::advance_walk,
    occupancy_sync::{
        OccupancyMaintenancePlugin, SimSystems, sync_destroyed_cover, sync_destroyed_slab,
    },
    peek_sync::{peek_population_needed, sync_peek_offsets},
    visibility::{SquadVisibility, recompute_visibility, should_recompute_visibility},
};

/// The sim's **battle-lifecycle registration unit** — adding this ONE plugin wires
/// the whole render-free combat runtime into a Bevy [`App`] (E10.5 / GTW-207).
///
/// In `build()` the plugin:
///
/// - adds [`OccupancyMaintenancePlugin`] (the E1.7 maintenance layer + the
///   [`CoverDestroyed`](crate::occupancy_sync::CoverDestroyed) buffer; it OWNS the
///   [`SimSystems::Simulate`] `configure_sets`) and the E10.2 [`SimActsPlugin`] (the
///   six `*Requested` buffers + their per-act dispatch systems) — so the app adds
///   only THIS plugin to get the full sim runtime;
/// - [`add_message`](App::add_message)s the three lifecycle types
///   ([`SetupBattleRequested`] / [`TeardownBattleRequested`] / [`BattleReady`]) plus the
///   two GTW-237 outcome witnesses ([`BattleWon`] / [`BattleLost`]) exactly once each
///   (`bevy-traps.md` #5 — an unregistered buffer fails a
///   [`MessageReader`](bevy::prelude::MessageReader)'s param validation);
/// - adds the roster-grounded [`check_outcome`] census `.in_set(`[`SimSystems::Simulate`]`)`
///   — so it rides the same [`BattleInProgress`] gate as the bundled runtime (inert and
///   panic-free outside a live battle);
/// - adds the GTW-341 squad-fog writer
///   [`recompute_visibility`](crate::visibility::recompute_visibility) — the SOLE
///   [`SquadVisibility`](crate::visibility::SquadVisibility) mutator — into the same gated
///   `Simulate` band, ordered `.after`
///   [`sync_destroyed_cover`](crate::occupancy_sync::sync_destroyed_cover) (the last
///   `occupancy_sync` grid-maintenance system) and gated on its
///   [`should_recompute_visibility`](crate::visibility::should_recompute_visibility) trigger
///   predicate; and
/// - adds [`setup_battle_on_request`] + [`teardown_battle_on_request`] to [`Update`],
///   ordered around the [`SimSystems::Simulate`] band (setup `.before`, teardown
///   `.after`), so the battle-lifetime resources are created before the bundled
///   in-set systems read them and removed only after they have run.
///
/// **Battle-scoped gating of the bundled runtime.** The bundled [`SimActsPlugin`] +
/// [`OccupancyMaintenancePlugin`] systems (all `.in_set(SimSystems::Simulate)`) read
/// the battle-lifetime resources ([`OccupancyGrid`](crate::occupancy::OccupancyGrid) /
/// [`SurfaceGrid`](crate::surface::SurfaceGrid) /
/// [`CoverLedger`](crate::cover::CoverLedger) / [`ShotRng`](crate::rng::ShotRng)/[`SeverityRng`](crate::rng::SeverityRng) /
/// [`CombatTuning`](crate::tuning::CombatTuning))
/// unconditionally — a Bevy `Res<T>` whose resource is absent fails param validation
/// (`bevy-traps.md` #1). Those resources exist only between a setup and a teardown, so
/// `BattleSimPlugin` gates the whole `Simulate` band on
/// [`resource_exists`]`::<`[`BattleInProgress`]`>` (the setup-inserted "a battle is
/// active" witness — GTW-212's purpose-built tag, replacing the incidental
/// [`OccupancyGrid`](crate::occupancy::OccupancyGrid)-as-gate proxy) — a `run_if` skips a
/// set's member systems entirely (no param validation) when false, so the bundled runtime
/// stays inert (and panic-free) before the first battle and after teardown. The setup
/// inserts [`BattleInProgress`] on the same `Ok` path that inserts
/// [`OccupancyGrid`](crate::occupancy::OccupancyGrid) and the teardown removes it
/// alongside, so the gated span is identical — only the witness is explicit. The lifecycle
/// systems are deliberately NOT in that gated band: the setup system must run to CREATE the
/// witness (it would otherwise dead-gate itself), and the teardown system must run AFTER
/// the band to remove the witness — so both run unconditionally, ordered around the band.
///
/// The plugin names NO `gdtf_app` type: the battle lifecycle is driven entirely by
/// the three lifecycle messages, so the app sends triggers and reads the signal
/// without the sim ever depending back on the app.
#[derive(Debug, Default, Clone, Copy)]
pub struct BattleSimPlugin;

impl Plugin for BattleSimPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(OccupancyMaintenancePlugin)
            .add_plugins(SimActsPlugin)
            .add_message::<SetupBattleRequested>()
            .add_message::<TeardownBattleRequested>()
            .add_message::<BattleReady>()
            // The roster-grounded outcome witnesses the Simulate-band census emits (GTW-237;
            // bevy-traps.md #5 — an unregistered buffer fails a MessageReader's validation).
            .add_message::<BattleWon>()
            .add_message::<BattleLost>()
            // Gate the whole Simulate band (the bundled dispatch + occupancy systems)
            // on the setup-inserted BattleInProgress witness — the purpose-built
            // "a battle is active" tag (GTW-212), replacing the incidental OccupancyGrid
            // proxy — so the bundled runtime is inert + panic-free outside a live battle
            // (bevy-traps.md #1; conditions still accumulate across configure_sets #5).
            .configure_sets(
                Update,
                SimSystems::Simulate.run_if(resource_exists::<BattleInProgress>),
            )
            // The roster-grounded outcome census joins the gated Simulate band, so it is
            // INERT (and its Res<PlayerFaction>/Res<BattleRoster> reads panic-free) outside
            // a live battle — same window as BattleInProgress (GTW-237). No configure_sets
            // (the set is owned upstream by OccupancyMaintenancePlugin).
            .add_systems(Update, check_outcome.in_set(SimSystems::Simulate))
            // GTW-406: the automatic positional PeekOffset populator, joining the gated
            // Simulate band. Ordered `.after(advance_walk)` (read each accepted step's
            // settled Position — walk.rs writes *position directly, same-frame) and
            // `.after(sync_destroyed_cover)` (read the updated destroyed-cover set before
            // is_blocked), and `.before(recompute_visibility)` so the peek it writes IN
            // PLACE this tick (direct &mut → same-frame Changed<PeekOffset>) is consumed by
            // THIS tick's recompute — no spurious next-tick recompute (bevy-traps.md #3).
            // Gated on its own trigger predicate so the full-scan runs only on a
            // move/spawn or a wall/cover destruction (the corner-changing signals).
            .add_systems(
                Update,
                sync_peek_offsets
                    .in_set(SimSystems::Simulate)
                    .after(advance_walk)
                    .after(sync_destroyed_cover)
                    .before(recompute_visibility)
                    .run_if(peek_population_needed),
            )
            // The squad-fog writer (GTW-341): the SOLE SquadVisibility mutator, joining the
            // gated Simulate band so its grid/tuning/SquadVisibility reads stay panic-free
            // in the battle-active window (bevy-traps.md #1). Ordered `.after`
            // sync_destroyed_cover — the LAST system in the occupancy_sync move → die →
            // cover maintenance chain — so the grids reflect the new occupant
            // positions/bands BEFORE sight is recomputed (clause 3; union_fov reads
            // OccupancyGrid::occupant_band, bevy-traps.md #3). Gated again on its trigger
            // predicate so the expensive union scan runs only on a sight-changing update.
            .add_systems(
                Update,
                recompute_visibility
                    .in_set(SimSystems::Simulate)
                    .after(sync_destroyed_cover)
                    // GTW-365: after the slab-surface maintenance too, so a destroyed slab's
                    // SurfaceGrid edit (destroy_slab) lands BEFORE sight recomputes — the
                    // reopened sightline (the round + LOS march already fly through a
                    // Destroyed slab) is reflected in the squad FOV this same tick
                    // (the cover-destroyed precedent; bevy-traps.md #3).
                    .after(sync_destroyed_slab)
                    // GTW-355: after the committed-walk engine too, so each accepted step's
                    // new Position is reflected in the squad FOV this same tick — the next
                    // walk tick then reads the refreshed fog before stepping again (the §44
                    // ambush invariant: revealed iff the mover's sight reaches it from where
                    // it stopped; bevy-traps.md #3).
                    .after(advance_walk)
                    // Guard the ResMut<SquadVisibility> read on the resource's presence
                    // (bevy-traps.md #1): setup inserts it on the Ok path, so it shares the
                    // BattleInProgress window — but a headless harness can open the
                    // BattleInProgress gate WITHOUT going through setup (the bleed runtime
                    // tests do), so this extra guard keeps the writer inert until the fog
                    // resource actually exists, rather than panicking on its absence.
                    .run_if(resource_exists::<SquadVisibility>)
                    .run_if(should_recompute_visibility),
            )
            // The lifecycle drivers run UNCONDITIONALLY (outside the gated band): setup
            // before the band (it creates the witness), teardown after (it removes it).
            .add_systems(
                Update,
                (
                    setup_battle_on_request.before(SimSystems::Simulate),
                    teardown_battle_on_request.after(SimSystems::Simulate),
                ),
            )
            // GTW-384: the LIVE stat-derivation re-derive — when the GangerStatTuning
            // resource CHANGES (the app-side hot-reload overwrites it on a
            // `stat.tuning.ron` edit, GTW-374 pattern), re-derive every spawned ganger's
            // computed stats + clamp its current pools to the new maxes. Runs UNGATED by
            // BattleInProgress (it just iterates ganger entities — a no-op if none exist).
            // The system SELF-GUARDS on the resource's presence (Option<Res>) + its
            // `is_changed()` INSIDE the body, rather than via a `Res`-param run condition,
            // so a harness that adds BattleSimPlugin without the persistent-Load
            // GangerStatTuning does not panic (bevy-traps.md #1 / #3). Ordered AFTER
            // setup_battle_on_request so a same-frame setup's freshly-spawned gangers are
            // present before this re-derive iterates (no one-frame split).
            .add_systems(
                Update,
                rederive_stats_on_tuning_change.after(setup_battle_on_request),
            )
            // GTW-436: the injury-ledger re-derive — when a ganger's InflictedInjuries
            // ledger CHANGES (a freshly-inflicted injury, GTW-437), re-project THAT
            // ganger's computed stats so the injury's modifier-layer deltas land
            // immediately. Same shared `rederive_one` projection as the tuning path; the
            // only difference is the trigger (Changed<InflictedInjuries> query filter).
            // Runs UNGATED by BattleInProgress (it iterates ganger entities — a no-op if
            // none changed) and self-guards on the Option<Res<GangerStatTuning>> presence
            // (bevy-traps.md #1 / #7 — no &mut World). Ordered AFTER the SimSystems::Simulate
            // band so a SAME-FRAME injury inflicted by the bundled fire runtime (the
            // GTW-437 apply boundary, in the Simulate band) is settled onto the derived
            // stats this tick, before the next tick's stat reads (bevy-traps.md #3).
            .add_systems(
                Update,
                rederive_stats_on_injury_change.after(SimSystems::Simulate),
            );
    }
}
