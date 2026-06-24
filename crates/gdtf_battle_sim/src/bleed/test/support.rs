//! Shared headless-app fixtures for the relocated `bleed` tests — the
//! [`MinimalPlugins`] bleed harness, the captured-message reader, and the world
//! read-back helpers. Re-exported `pub(super)` so each per-AC sibling can drive the
//! same wiring.

pub(super) use bevy::prelude::{
    App, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut, Resource, Update,
};

pub(super) use crate::{
    bleed::{Bleeding, tick_bleed},
    ganger::{LifeState, Stabilized, Wounds},
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
