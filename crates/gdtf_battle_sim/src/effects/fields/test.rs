//! Unit tests for the GTW-545 area-damage-field runtime — the per-round [`tick_fields`] drain
//! (HP-decrement, whole-source immunity skip, the field-kills terminal gate) and the
//! [`FieldRegistry`] lifetime (Turns countdown removal, Permanent persistence). Driven
//! headlessly on a `MinimalPlugins` app (the sim crate cannot dev-dep `gdtf_test_utils` — a
//! cycle; the DOT-test bare-`App` fallback the contract sanctions). NO
//! `unwrap`/`expect`/`panic` in a test body (the workspace-denied idiom holds in tests too —
//! assert instead).

use bevy::prelude::{
    App, Deref, DerefMut, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut,
    Resource, Update,
};

use super::{
    FieldDamage, FieldDef, FieldDuration, FieldRegistry, FieldTicked, FieldTurns, ImmuneArmorTypes,
    tick_fields,
};
use crate::{
    armor::{ArmorType, WornBy},
    ganger::{Hp, LifeState},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    weapon::DamageType,
};

/// Captures the [`FieldTicked`] messages the reader drained, so a test can assert on them after
/// `update()` (no `unwrap` in the test body). Private inner (no-bare-types rule 5): pushed via
/// `DerefMut`, read via `Deref`.
#[derive(Resource, Default, Deref, DerefMut)]
struct Captured(Vec<FieldTicked>);

/// Drains the buffered [`FieldTicked`] messages (a [`MessageReader`], NOT an observer) into the
/// [`Captured`] resource for assertion.
fn consume(mut reader: MessageReader<FieldTicked>, mut captured: ResMut<Captured>) {
    for ticked in reader.read() {
        captured.push(*ticked);
    }
}

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// Build a headless app wired for the field tick: `MinimalPlugins`, the [`FieldTicked`] buffer,
/// and `tick_fields` chained before [`consume`] in [`Update`] so the captured messages reflect
/// the same tick. `tick_fields` is run UNGATED here (no `enemy_phase_started` run-if) — the
/// cadence wiring is `SimActsPlugin`'s job (proven by the GTW-545 integration test); this unit
/// harness exercises the drain math itself, one tick per `update()`.
fn tick_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_message::<FieldTicked>();
    // GTW-547: tick_fields now also writes OnDeathOccurred on a field-kill — register the buffer
    // so its MessageWriter param validates (an unregistered buffer panics the system).
    app.add_message::<crate::effects::on_death::OnDeathOccurred>();
    // GTW-572: tick_fields now also writes the once-per-span FieldAfflicted start fact.
    app.add_message::<crate::effects::fields::FieldAfflicted>();
    app.init_resource::<Captured>();
    app.add_systems(Update, (tick_fields, consume).chain());
    app
}

/// A field def of `damage` per turn with the given `immune` set and `duration` (Chem flavour —
/// the tick bypasses the matchup wheel, so the type is irrelevant to the drain).
fn field(damage: u16, immune: &[ArmorType], duration: FieldDuration) -> FieldDef {
    FieldDef::new(
        FieldDamage::new(damage),
        DamageType::Chem,
        ImmuneArmorTypes::new(immune.iter().copied()),
        duration,
    )
}

/// Insert an [`OccupancyGrid`] with `occupant` standing at `cell` (no terrain).
fn grid_with_occupant(app: &mut App, cell: CellLevel, occupant: Entity) {
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(cell, Some(occupant));
    app.world_mut().insert_resource(grid);
}

/// Spawn a ganger at full-ish `hp`, Alive, wearing ONE armor piece of `armor_type` — the
/// worn-piece entity relates back via [`WornBy`], populating the ganger's `Wears` collection.
fn spawn_armored_ganger(app: &mut App, hp: u16, armor_type: ArmorType) -> Entity {
    let ganger = app.world_mut().spawn((Hp::new(hp), LifeState::Alive)).id();
    // The worn piece relates to the ganger via WornBy — the framework populates Wears.
    app.world_mut().spawn((armor_type, WornBy::new(ganger)));
    ganger
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

/// The live field count in the registry (0 if the resource is absent).
fn field_count(app: &App) -> usize {
    app.world()
        .get_resource::<FieldRegistry>()
        .map_or(0, FieldRegistry::len)
}

/// Count the captured [`FieldTicked`] messages carrying a given occupant.
fn tick_count_for(app: &App, occupant: Entity) -> usize {
    app.world()
        .get_resource::<Captured>()
        .map_or(0, |c| c.iter().filter(|t| t.occupant == occupant).count())
}

// === (a) tick_fields damages an unprotected occupant a flat amount each round. ===

#[test]
fn field_damages_an_unprotected_occupant_each_round() {
    let cell = ground(4, 4);
    let per_turn = 3u16;
    let start_hp = 100u16;

    let mut app = tick_app();
    // A Permanent field so it never expires across the ticks we measure.
    let ganger = spawn_armored_ganger(&mut app, start_hp, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    // The occupant wears Plated — NOT in the Chem field's immune set (Flak/Hazard), so it takes
    // damage.
    registry.spawn(
        cell,
        field(per_turn, &[ArmorType::Flak], FieldDuration::Permanent),
    );
    app.world_mut().insert_resource(registry);

    // Each `update()` runs one field round; measure the drop over exactly three rounds — the
    // cumulative drop is 3×per_turn (a flat direct drain, no matchup / RNG).
    let rounds = 3u16;
    for _ in 0..rounds {
        app.update();
    }
    assert_eq!(
        hp_of(&app, ganger),
        start_hp - per_turn * rounds,
        "each field round drains exactly per_turn HP (a flat direct drain, no matchup / RNG)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Alive,
        "a non-lethal field tick leaves the occupant Alive",
    );
    // Exactly one FieldTicked was emitted per draining round.
    assert_eq!(
        tick_count_for(&app, ganger),
        usize::from(rounds),
        "a FieldTicked is emitted once per draining round",
    );
}

// === (b) whole-source immunity: a matching worn ArmorType skips the field's damage entirely. ===

#[test]
fn immune_armor_skips_the_field_damage() {
    let cell = ground(2, 2);
    let start_hp = 50u16;

    let mut app = tick_app();
    // The occupant wears Flak — which IS in the field's immune set, so it takes ZERO damage.
    let ganger = spawn_armored_ganger(&mut app, start_hp, ArmorType::Flak);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(9, &[ArmorType::Flak], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);
    app.update();

    app.update();
    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        start_hp,
        "whole-source immunity: a worn ArmorType in the immune set skips ALL field damage",
    );
    assert_eq!(
        tick_count_for(&app, ganger),
        0,
        "an immune occupant emits NO FieldTicked (it took no damage)",
    );
}

// === (c) a Turns field counts down and is REMOVED after its turn count runs out. ===

#[test]
fn turns_field_counts_down_and_is_removed_after_n_rounds() {
    let cell = ground(3, 3);
    let turns = 2u8;

    let mut app = tick_app();
    let ganger = spawn_armored_ganger(&mut app, 100, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(
        cell,
        field(3, &[], FieldDuration::Turns(FieldTurns::new(turns))),
    );
    app.world_mut().insert_resource(registry);
    app.update();

    assert_eq!(field_count(&app), 1, "the field is live before any tick");
    // Tick exactly `turns` rounds — the countdown reaches zero and the field is removed.
    for _ in 0..turns {
        app.update();
    }
    assert_eq!(
        field_count(&app),
        0,
        "a Turns field is removed once its turn count runs out",
    );
    // A further round drains no more HP (the field is gone).
    let hp_after_expiry = hp_of(&app, ganger);
    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        hp_after_expiry,
        "with the field removed a further round drains no HP",
    );
}

// === (d) a Permanent field NEVER expires (persists across many rounds). ===

#[test]
fn permanent_field_never_expires() {
    let cell = ground(5, 5);

    let mut app = tick_app();
    let ganger = spawn_armored_ganger(&mut app, 100, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(1, &[], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);
    app.update();

    // Many rounds — the Permanent field is still present every time.
    for _ in 0..10 {
        app.update();
        assert_eq!(
            field_count(&app),
            1,
            "a Permanent field never expires, however many rounds tick",
        );
    }
}

// === (e) a field drain that empties HP flips the occupant to Dead. ===

#[test]
fn a_field_tick_that_empties_hp_flips_the_occupant_to_dead() {
    let cell = ground(1, 1);

    let mut app = tick_app();
    // 8 HP against a 10/turn field — the tick floors HP at 0 (saturating) and KILLS.
    let ganger = spawn_armored_ganger(&mut app, 8, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);
    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(10, &[], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);
    app.update();

    app.update();
    assert_eq!(
        hp_of(&app, ganger),
        0,
        "the field tick floors HP at 0 (saturating, no underflow)",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Dead,
        "a field drain that empties HP KILLS (Dead — the GTW-544 DOT-kills precedent)",
    );
}

// === (f) GTW-572: the once-per-span FieldAfflicted exposure-start fact. ===

/// Captures every [`FieldAfflicted`](crate::effects::fields::FieldAfflicted) start fact across the
/// run (the [`Captured`] idiom — a consumer system, NOT a raw buffer peek, so the count is
/// cumulative and independent of the double-buffer swap timing). Private inner (rule 5).
#[derive(Resource, Default, Deref, DerefMut)]
struct CapturedAfflicted(Vec<crate::effects::fields::FieldAfflicted>);

/// Drains the buffered [`FieldAfflicted`](crate::effects::fields::FieldAfflicted) facts into
/// [`CapturedAfflicted`] for assertion.
fn consume_afflicted(
    mut reader: MessageReader<crate::effects::fields::FieldAfflicted>,
    mut captured: ResMut<CapturedAfflicted>,
) {
    for afflicted in reader.read() {
        captured.push(*afflicted);
    }
}

/// The CUMULATIVE count of captured [`FieldAfflicted`](crate::effects::fields::FieldAfflicted)
/// facts for `occupant` across the run.
fn afflicted_count_for(app: &App, occupant: Entity) -> usize {
    app.world()
        .get_resource::<CapturedAfflicted>()
        .map_or(0, |c| c.iter().filter(|a| a.occupant == occupant).count())
}

/// GTW-572 (the Q2 ruling): the FIRST round a live field drains an occupant emits exactly
/// ONE [`FieldAfflicted`](crate::effects::fields::FieldAfflicted) exposure-start fact; the following
/// mid-exposure rounds emit no further start fact (the per-round `FieldTicked` keeps
/// firing); stepping OFF the field for a round and back ON starts a NEW span that
/// re-announces.
#[test]
fn a_field_exposure_announces_once_per_span_and_reannounces_after_leaving() {
    let cell = ground(5, 5);
    let mut app = tick_app();
    app.init_resource::<CapturedAfflicted>();
    app.add_systems(Update, consume_afflicted.after(tick_fields));
    let ganger = spawn_armored_ganger(&mut app, 100, ArmorType::Plated);
    grid_with_occupant(&mut app, cell, ganger);

    let mut registry = FieldRegistry::new();
    registry.spawn(cell, field(3, &[], FieldDuration::Permanent));
    app.world_mut().insert_resource(registry);

    // Round one — the exposure span starts: exactly one start fact.
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "the FIRST draining round emits exactly one FieldAfflicted",
    );

    // Round two — mid-exposure: the drain ticks again, but NO new start fact (the
    // cumulative capture stays at one).
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "a mid-exposure round emits NO further FieldAfflicted (once per span)",
    );
    assert!(
        tick_count_for(&app, ganger) >= 2,
        "the per-round FieldTicked drain keeps firing mid-span",
    );

    // Step OFF the field for a round — the span ends (the marker removal is deferred; the
    // empty-cell round both skips the drain and unmarks).
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, None);
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        1,
        "an off-field round emits nothing",
    );

    // Step back ON — a NEW exposure span, a NEW start fact (the cumulative capture climbs
    // to two).
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    app.update();
    assert_eq!(
        afflicted_count_for(&app, ganger),
        2,
        "re-entering the field is a NEW span and must re-announce",
    );
}
