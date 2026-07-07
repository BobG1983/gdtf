//! Shared headless-app fixtures for the GTW-544 DOT runtime tests — the
//! `MinimalPlugins` tick / apply harnesses, the captured-message reader, and the
//! world read-back helpers. Driven headlessly on a bare `App` (the sim crate cannot
//! dev-dep `gdtf_test_utils` — a cycle; the bleed-test bare-`App` fallback the
//! contract sanctions). NO `unwrap`/`expect`/`panic` in a test body (the
//! workspace-denied idiom holds in tests too — assert instead). Re-exported
//! `pub(super)` so each per-concern sibling drives the same wiring.

pub(super) use bevy::prelude::{
    App, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut, Resource, Update,
};

pub(super) use crate::{
    effects::dot::{DotAfflicted, DotApplied, DotTicked, apply_dot, tick_dot},
    ganger::{Hp, LifeState, Position},
    weapon::{DamageType, Dot, DotDamage, DotProfile},
};
use crate::{
    metric::{Cell, CellLevel, Level},
    test_support::dot_turns,
};

/// Captures the [`DotTicked`] messages the reader drained, so a test can assert on them after
/// `update()` (no `unwrap` in the test body). Private inner (no-bare-types rule 5): pushed via
/// `DerefMut`, read via `Deref`.
#[derive(Resource, Default, bevy::prelude::Deref, bevy::prelude::DerefMut)]
pub(super) struct Captured(Vec<DotTicked>);

/// Drains the buffered [`DotTicked`] messages (a [`MessageReader`], NOT an observer) into the
/// [`Captured`] resource for assertion.
fn consume(mut reader: MessageReader<DotTicked>, mut captured: ResMut<Captured>) {
    for ticked in reader.read() {
        captured.push(*ticked);
    }
}

/// A ground-floor `(cell, level)` key.
pub(super) fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build a headless app wired for the DOT tick: `MinimalPlugins`, the [`DotTicked`] buffer,
/// and `tick_dot` chained before [`consume`] in [`Update`] so the captured messages reflect
/// the same tick. `tick_dot` is run UNGATED here (no `enemy_phase_started` run-if) — the cadence
/// wiring is `SimActsPlugin`'s job (proven by the GTW-544 integration test); this unit harness
/// exercises the drain math itself, one tick per `update()`.
pub(super) fn tick_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<DotTicked>();
    // GTW-547: tick_dot now also writes OnDeathOccurred on a DOT-kill — register the buffer so
    // its MessageWriter param validates (an unregistered buffer panics the system).
    app.add_message::<crate::effects::on_death::OnDeathOccurred>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (tick_dot, consume).chain());
    app
}

/// Build a headless app wired for the DOT applier: `MinimalPlugins`, the [`DotApplied`]
/// buffer, and `apply_dot` in [`Update`].
pub(super) fn apply_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<DotApplied>();
    // GTW-572: apply_dot now also writes the once-per-span DotAfflicted start fact on the
    // fresh-ATTACH branch — register the buffer so its MessageWriter param validates.
    app.add_message::<DotAfflicted>();
    app.add_systems(Update, apply_dot);
    app
}

/// Read a spawned ganger's current Hp (returns `0` if absent, so no `unwrap`).
pub(super) fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

/// Read a spawned ganger's current `LifeState` (defaults Alive if absent).
pub(super) fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

/// The ganger's active DOT, if it still carries one (removed at expiry / on the killing tick).
pub(super) fn dot_of(app: &App, ganger: Entity) -> Option<Dot> {
    app.world().get::<Dot>(ganger).copied()
}

/// Count the captured [`DotTicked`] messages carrying a given ganger.
pub(super) fn tick_count_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<Captured>()
        .map_or(0, |c| c.iter().filter(|t| t.ganger == ganger).count())
}

/// A `Dot` of `damage` per turn for `turns` turns (Kinetic flavour — the tick bypasses the
/// matchup wheel, so the type is irrelevant to the drain). `turns` is a positive test
/// literal — a zero-turn DOT is UNREPRESENTABLE ([`crate::weapon::DotTurns`] wraps
/// `NonZeroU8`, GTW-643).
pub(super) fn dot(damage: u16, turns: u8) -> Dot {
    Dot::from_profile(DotProfile::new(
        DotDamage::new(damage),
        DamageType::Kinetic,
        dot_turns(turns),
    ))
}
