//! Shared headless-app fixtures for the relocated `bleed` tests — the
//! [`MinimalPlugins`] bleed harness, the GTW-336 LIVE-runtime harness (the real
//! [`BattleSimPlugin`] registration the runtime / entry-gate / turn-start siblings
//! drive), the captured-message reader, and the world read-back helpers.
//! Re-exported `pub(super)` so each per-AC sibling can drive the same wiring.

use bevy::prelude::Messages;
pub(super) use bevy::prelude::{
    App, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut, Resource, Update,
};

use crate::{
    acts::EndTurnRequested,
    battle::{BattleInProgress, BattleRoster, BattleSimPlugin},
    rng::BattleSeed,
    test_support::{GangerEntityBuilder, insert_sim_resources},
    turn::ActiveFaction,
};
pub(super) use crate::{
    effects::bleed::{Bleeding, BleedingOut, tick_bleed},
    ganger::{Faction, LifeState, Wounds},
    tuning::CombatTuning,
};

/// Captures the [`Bleeding`] messages a reader system drained, so a test can
/// assert on them after `update()` (no `unwrap` in the test body).
///
/// The inner `Vec` is private (no-bare-types rule 5): the captured messages are
/// pushed through [`DerefMut`](core::ops::DerefMut) and read through
/// [`Deref`](core::ops::Deref), never a `.0` field access.
#[derive(Resource, Default, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub(super) struct Captured(Vec<Bleeding>);

/// Drains the buffered [`Bleeding`] messages (a [`MessageReader`], NOT an
/// observer) into the [`Captured`] resource for assertion.
fn consume(mut reader: MessageReader<Bleeding>, mut captured: ResMut<Captured>) {
    for bled in reader.read() {
        captured.push(*bled);
    }
}

/// Build a headless app wired for the bleed-out tick: `MinimalPlugins` (no
/// window/renderer), the [`Bleeding`] message buffer registered, a default
/// [`CombatTuning`] resource, and `tick_bleed` chained before [`consume`] in
/// [`Update`] so the captured messages reflect the same tick.
///
/// The sim crate cannot depend on `gdtf_test_utils` (that would cycle through
/// `gdtf_app` → `gdtf_battle_sim`), so this uses the bare-`App` + `MinimalPlugins`
/// fallback the contract sanctions — the same pattern `armor_wear.rs` and
/// `occupancy_sync.rs` already use. The default tuning supplies the bleed rate
/// (read, never pinned — tests assert the drop's relation to it).
pub(super) fn bleed_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<Bleeding>();
    // GTW-547: tick_bleed now also writes OnDeathOccurred on a bleed-out kill — register the
    // buffer so its MessageWriter param validates (an unregistered buffer panics the system).
    app.add_message::<crate::effects::on_death::OnDeathOccurred>();
    // GTW-572: tick_bleed now also writes the once-per-span BleedStarted start fact.
    app.add_message::<crate::effects::bleed::BleedStarted>();
    app.init_resource::<CombatTuning>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (tick_bleed, consume).chain());
    app
}

/// The bleed rate the default [`CombatTuning`] carries — read, never pinned; the
/// tests assert the per-tick drop EQUALS this, a relation to the tuning value.
pub(super) fn bleed_rate() -> u8 {
    *CombatTuning::default().bleed_rate
}

/// Read a spawned ganger's current `Wounds` back out of the app's world (returns
/// `0` if the entity or component is somehow absent, so the test body needs no
/// `unwrap`). A read-only world access — the real component, not a copy.
pub(super) fn wounds_of(app: &App, ganger: Entity) -> u8 {
    app.world().get::<Wounds>(ganger).map_or(0, |w| **w)
}

/// Read a spawned ganger's current `LifeState` back out of the app's world
/// (defaults to [`LifeState::Alive`] if absent, so no `unwrap`).
pub(super) fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

/// Count the captured [`Bleeding`] messages carrying a given ganger.
pub(super) fn bleeding_count_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<Captured>()
        .map_or(0, |c| c.iter().filter(|b| b.ganger == ganger).count())
}

// --- The GTW-336 LIVE-runtime harness (shared by the runtime / entry-gate / turn-start
// siblings — the module-layout 2+-consumers rule homes it here, lifted from `runtime.rs`).

/// A fixed seed for the per-test RNG streams (arbitrary, not tuned).
pub(super) const SEED: u64 = 0x5A1C_AC75;

/// Gang `0` is the player team (the litany's `PlayerFaction` gang).
pub(super) const PLAYER: Faction = Faction::new(0);
/// The enemy team.
pub(super) const ENEMY: Faction = Faction::new(1);

/// Seed the shared battle-lifetime resources a live battle has (everything BUT the
/// [`BattleInProgress`] gate witness): the three grids, the five seeded RNG streams (GTW-14),
/// [`CombatTuning::default`] (persistent `Load`-tier in production), the
/// [`BattleRoster`] the `check_outcome` census reads, and `PlayerFaction` +
/// [`ActiveFaction`] on the player (the player acts first). [`BattleSimPlugin`] (added
/// by the caller) bundles
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) and
/// [`SimActsPlugin`](crate::acts::SimActsPlugin) and installs the real `BattleInProgress`
/// gate on the `SimSystems::Simulate` band — so these tests drive the exact production
/// registration, gate included.
pub(super) fn seed_battle_resources(app: &mut App) {
    // The canonical seeding litany (GTW-576) — grids, ledgers, five RNG streams, empty
    // injury content, default tuning + uniform floor costs, PlayerFaction on gang 0
    // (== PLAYER here).
    insert_sim_resources(app, BattleSeed::new(SEED));
    // The live-runtime extras on top of the litany: the BattleRoster the `check_outcome`
    // census reads, and ActiveFaction on the player (the player acts first).
    app.insert_resource(BattleRoster::new([PLAYER, ENEMY]));
    app.insert_resource(ActiveFaction::new(PLAYER));
}

/// Build the FULL live-runtime harness: [`MinimalPlugins`] + the real production
/// [`BattleSimPlugin`] (which bundles
/// [`OccupancyMaintenancePlugin`](crate::occupancy_sync::OccupancyMaintenancePlugin) +
/// [`SimActsPlugin`](crate::acts::SimActsPlugin) — the turn-cycle engine + the GTW-336
/// bleed wiring — AND installs the `run_if(resource_exists::<BattleInProgress>)` gate on
/// the `SimSystems::Simulate` band), seeded with the battle-lifetime resources a live
/// battle has PLUS the [`BattleInProgress`] gate witness (so the gated Simulate band
/// actually runs). Driving through `BattleSimPlugin` means these tests exercise the SAME
/// registration the live runtime uses — gate included — not a hand-rolled subset.
pub(super) fn live_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(BattleSimPlugin);
    seed_battle_resources(&mut app);
    app.insert_resource(BattleInProgress);
    app
}

/// Spawn a ganger of `faction` in the given [`LifeState`] with a Wounds pool, plus a full
/// TU pool (so the turn engine's regen has something to act on, matching a real ganger). A
/// [`LifeState::Downed`] ganger is spawned carrying the [`BleedingOut`] condition — a
/// freshly-downed ganger is bleeding out (GTW-695); pass it through
/// [`entity_mut`](bevy::prelude::World::entity_mut)`.remove::<BleedingOut>()` to model a
/// stabilized one.
pub(super) fn bleeding_ganger(
    app: &mut App,
    faction: Faction,
    life: LifeState,
    wounds: u8,
) -> Entity {
    let ganger = GangerEntityBuilder::new()
        .faction(faction)
        .life_state(life)
        .wounds(wounds)
        .tu(0)
        .tu_max(100)
        .spawn(app.world_mut());
    if life == LifeState::Downed {
        app.world_mut().entity_mut(ganger).insert(BleedingOut);
    }
    ganger
}

/// Send one End-Turn request and run the cycle (player → enemy → auto-pass → player).
pub(super) fn end_turn(app: &mut App) {
    app.world_mut().write_message(EndTurnRequested);
    app.update();
}

/// Drain the buffered [`Bleeding`] messages emitted this run, in order.
pub(super) fn drain_bleeding(app: &mut App) -> Vec<Bleeding> {
    app.world_mut()
        .resource_mut::<Messages<Bleeding>>()
        .drain()
        .collect()
}
