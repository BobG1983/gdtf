//! Unit tests for the GTW-544 DOT runtime — the per-round [`tick_dot`] drain (HP-decrement,
//! removal after the profile turn count, the DOT-kills terminal gate) and the [`apply_dot`]
//! refresh-not-stack applier. Driven headlessly on a `MinimalPlugins` app (the sim crate
//! cannot dev-dep `gdtf_test_utils` — a cycle; the bleed-test bare-`App` fallback the contract
//! sanctions). NO `unwrap`/`expect`/`panic` in a test body (the workspace-denied idiom holds
//! in tests too — assert instead).

use bevy::prelude::{
    App, Deref, DerefMut, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut,
    Resource, Update,
};

use crate::{
    dot::{DotApplied, DotTicked, apply_dot, tick_dot},
    ganger::{Hp, LifeState, Position},
    metric::{Cell, CellLevel, Level},
    weapon::{Dot, DotDamage, DotProfile, DotTurns},
};

/// Captures the [`DotTicked`] messages the reader drained, so a test can assert on them after
/// `update()` (no `unwrap` in the test body). Private inner (no-bare-types rule 5): pushed via
/// `DerefMut`, read via `Deref`.
#[derive(Resource, Default, Deref, DerefMut)]
struct Captured(Vec<DotTicked>);

/// Drains the buffered [`DotTicked`] messages (a [`MessageReader`], NOT an observer) into the
/// [`Captured`] resource for assertion.
fn consume(mut reader: MessageReader<DotTicked>, mut captured: ResMut<Captured>) {
    for ticked in reader.read() {
        captured.push(*ticked);
    }
}

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build a headless app wired for the DOT tick: `MinimalPlugins`, the [`DotTicked`] buffer,
/// and `tick_dot` chained before [`consume`] in [`Update`] so the captured messages reflect
/// the same tick. `tick_dot` is run UNGATED here (no `enemy_phase_started` run-if) — the cadence
/// wiring is `SimActsPlugin`'s job (proven by the GTW-544 integration test); this unit harness
/// exercises the drain math itself, one tick per `update()`.
fn tick_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<DotTicked>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (tick_dot, consume).chain());
    app
}

/// Build a headless app wired for the DOT applier: `MinimalPlugins`, the [`DotApplied`]
/// buffer, and `apply_dot` in [`Update`].
fn apply_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<DotApplied>();
    app.add_systems(Update, apply_dot);
    app
}

/// Read a spawned ganger's current Hp (returns `0` if absent, so no `unwrap`).
fn hp_of(app: &App, ganger: Entity) -> u16 {
    app.world().get::<Hp>(ganger).map_or(0, |h| **h)
}

/// Read a spawned ganger's current `LifeState` (defaults Alive if absent).
fn life_of(app: &App, ganger: Entity) -> LifeState {
    app.world()
        .get::<LifeState>(ganger)
        .copied()
        .unwrap_or(LifeState::Alive)
}

/// The ganger's active DOT, if it still carries one (removed after the profile turn count).
fn dot_of(app: &App, ganger: Entity) -> Option<Dot> {
    app.world().get::<Dot>(ganger).copied()
}

/// Count the captured [`DotTicked`] messages carrying a given ganger.
fn tick_count_for(app: &App, ganger: Entity) -> usize {
    app.world()
        .get_resource::<Captured>()
        .map_or(0, |c| c.iter().filter(|t| t.ganger == ganger).count())
}

/// A `Dot` of `damage` per turn for `turns` turns (Kinetic flavour — the tick bypasses the
/// matchup wheel, so the type is irrelevant to the drain).
fn dot(damage: u16, turns: u8) -> Dot {
    Dot::from_profile(DotProfile::new(
        DotDamage::new(damage),
        crate::weapon::DamageType::Kinetic,
        DotTurns::new(turns),
    ))
}

// === (c) tick_dot decrements HP each round with NO armor/injury/RNG, removes after N turns. ===

#[test]
fn tick_dot_drains_hp_each_round_and_removes_after_the_profile_turn_count() {
    // A 3-turn DOT of 5 HP/turn on a deep HP pool (so no tick reaches 0 — we measure the
    // per-turn drop + the removal, not the kill).
    let per_turn = 5u16;
    let turns = 3u8;
    let start_hp = 100u16;

    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(start_hp),
            LifeState::Alive,
            dot(per_turn, turns),
            Position::new(ground(4, 4)),
        ))
        .id();

    // Each of the `turns` ticks drains exactly `per_turn` HP (no armor / injury / RNG — a
    // flat direct drain) and the ganger stays Alive.
    for n in 1..=turns {
        app.update();
        assert_eq!(
            hp_of(&app, ganger),
            start_hp - per_turn * u16::from(n),
            "after {n} ticks the cumulative HP drop must be {n}×per_turn (a flat direct drain)",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Alive,
            "a non-lethal DOT tick leaves the ganger Alive",
        );
    }

    // After the profile turn count the DOT is REMOVED — a further tick drains nothing.
    assert!(
        dot_of(&app, ganger).is_none(),
        "the DOT is removed once its profile turn count runs out",
    );
    let hp_after_expiry = hp_of(&app, ganger);
    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        hp_after_expiry,
        "with the DOT removed a further round drains no HP",
    );
    // Exactly `turns` ticks were emitted (none after removal).
    assert_eq!(
        tick_count_for(&app, ganger),
        usize::from(turns),
        "exactly one DotTicked per turn of the profile (none after removal)",
    );
}

// === (d) a DOT tick that brings HP to 0 flips LifeState to Dead. ===

#[test]
fn a_dot_tick_that_empties_hp_flips_the_ganger_to_dead() {
    // A single 10 HP/turn tick on a ganger with exactly 8 HP — the tick floors HP at 0
    // (saturating) and the GTW-544 locked gate flips it to Dead (NOT Downed).
    let mut app = tick_app();
    let ganger = app
        .world_mut()
        .spawn((
            Hp::new(8),
            LifeState::Alive,
            dot(10, 3),
            Position::new(ground(2, 2)),
        ))
        .id();

    app.update();

    assert_eq!(
        hp_of(&app, ganger),
        0,
        "the DOT tick floors HP at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "the GTW-544 locked design: a DOT tick that empties HP KILLS (Dead, not Downed)",
    );
    // The DOT is removed on the killing tick (a corpse never ticks again).
    assert!(
        dot_of(&app, ganger).is_none(),
        "the DOT is removed on the killing tick",
    );
}

// === (e) refresh-not-stack: a second DOT attach RESETS turns (does not stack). ===

#[test]
fn apply_dot_refreshes_not_stacks() {
    let mut app = apply_app();
    let ganger = app.world_mut().spawn((Hp::new(50), LifeState::Alive)).id();

    // First attach: 4 HP/turn for 2 turns.
    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(4, 2)));
    app.update();
    let first = dot_of(&app, ganger);
    assert_eq!(
        first,
        Some(dot(4, 2)),
        "the first DOT attach lands its full profile",
    );

    // Second attach on the SAME ganger with DIFFERENT values: 9 HP/turn for 5 turns. The
    // affliction is RESET to the new profile — NOT stacked (turns are 5, not 2+5=7; damage is
    // 9, not 4+9).
    app.world_mut()
        .write_message(DotApplied::new(ganger, dot(9, 5)));
    app.update();
    let refreshed = dot_of(&app, ganger);
    assert_eq!(
        refreshed,
        Some(dot(9, 5)),
        "refresh-not-stack: a second DOT attach RESETS turns + per-turn damage to the new \
         profile (not additive)",
    );
    // Explicitly assert the turns did NOT accumulate.
    if let Some(d) = refreshed {
        assert_eq!(
            *d.remaining_turns, 5,
            "the refreshed turns are the NEW profile's 5, never the stacked 2+5",
        );
    }
}
