//! GTW-572 (C5/C6) — the combat log's STATE-CHANGE coverage, driven END-TO-END through the
//! REAL app stack: a sim fact message written to the live buffer → the registered per-source
//! FORWARDER (resolving the entity to its `GangerName`) → the buffered `CombatLogEvent` → the
//! ONE APPENDER → a rendered line under the combat-log root.
//!
//! One test per clause-6 producer, each asserting the line content POSITIVELY (the Q2
//! ruling: ALL state changes log — falls, melee damage, on-death kills, suppression,
//! armor-broken — and the DOT / field / bleed afflictions log ONCE at affliction start,
//! with an explicit mid-affliction-tick-logs-NOTHING assert per affliction family).

use bevy::{ecs::entity::Entity, prelude::*, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, CombatLogLine, RunningState};
use gdtf_battle_sim::{
    ArmorBroken, BleedStarted, Bleeding, BodyPart, Cell, CellLevel, DotAfflicted, DotDamage,
    DotTicked, FallOccurred, FieldAfflicted, FieldDamage, FieldTicked, GangerName, HpDamage, Level,
    MeleeStruck, OnDeathOccurred, StoreysFallen, injuries::InjuryRegistry, tuning::CombatTuning,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine
/// that never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// Reads the current [`BattleScapeState`] if active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Drives the real stack to `BattleScapeState::BattleRunning`, where the combat log is live
/// (the `combat_log.rs` harness, verbatim — integration test binaries cannot share code).
fn battle_running_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::MeleeWeaponRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::AttachmentRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::ganger::GangRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::PrefabRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::terrain::def::TerrainDefRegistry::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::level::UuidThemeRegistry::default());

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(at_menu, "the walk should reach RunningState::Menu");
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        at_battle,
        "the walk should reach BattleScapeState::BattleRunning; last was {:?}",
        battlescape_state(&app),
    );
    app
}

/// The rendered `Text` strings of every combat-log line.
fn line_texts(app: &mut App) -> Vec<String> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<CombatLogLine>>();
    let entities: Vec<Entity> = q.iter(app.world()).collect();
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.as_str().to_owned()))
        .collect()
}

/// How many combat-log lines currently read exactly `text`.
fn count_of(app: &mut App, text: &str) -> usize {
    line_texts(app).into_iter().filter(|t| t == text).count()
}

/// Spawns a NAMED ganger (so the forwarders can resolve its `Entity` to a `GangerName`).
fn spawn_named(app: &mut App, name: &str) -> Entity {
    app.world_mut().spawn(GangerName::new(name.to_owned())).id()
}

/// A ground-floor `(cell, level)` key.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// A `FallOccurred` appends `"<name> fell <N> storeys"` — the GTW-572 fall gain line,
/// end-to-end through the forwarder → appender path.
#[test]
fn a_fall_occurred_appends_the_fell_line() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut().write_message(FallOccurred::new(
        ganger,
        Level::new(2),
        Level::new(0),
        StoreysFallen::new(2),
    ));
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex fell 2 storeys"),
        1,
        "a FallOccurred must append exactly one \"Vex fell 2 storeys\" line, got {:?}",
        line_texts(&mut app),
    );
}

/// A `MeleeStruck` appends `"<attacker> struck <target> (-N)"` — the GTW-572 melee-damage
/// line the new number-bearing sim fact enables (`MeleeResolved` carries no actor / amount).
#[test]
fn a_melee_struck_appends_the_damage_line() {
    let mut app = battle_running_app();
    let attacker = spawn_named(&mut app, "Vex");
    let target = spawn_named(&mut app, "Skar");
    app.update();

    app.world_mut()
        .write_message(MeleeStruck::new(attacker, target, HpDamage::new(4)));
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex struck Skar (-4)"),
        1,
        "a MeleeStruck must append exactly one \"Vex struck Skar (-4)\" line, got {:?}",
        line_texts(&mut app),
    );
}

/// A ganger `OnDeathOccurred` appends the NAMED `"<name> dies"` line; a COVER death
/// (placeholder entity) appends NOTHING — no phantom `"Someone dies"` (the forwarder skips
/// the placeholder; the shot path already logs `"Cover Destroyed"`).
#[test]
fn an_on_death_appends_the_named_dies_line_and_a_cover_death_does_not() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut()
        .write_message(OnDeathOccurred::new(ganger, ground(4, 4)));
    app.update();
    assert_eq!(
        count_of(&mut app, "Vex dies"),
        1,
        "a ganger death must append exactly one \"Vex dies\" line, got {:?}",
        line_texts(&mut app),
    );

    // A COVER death — Entity::PLACEHOLDER — must yield NO line at all (never "Someone dies").
    app.world_mut()
        .write_message(OnDeathOccurred::cover(ground(5, 5)));
    app.update();
    app.update();
    assert_eq!(
        count_of(&mut app, "Someone dies"),
        0,
        "a cover death (placeholder entity) must NOT log a phantom \"Someone dies\", got {:?}",
        line_texts(&mut app),
    );
}

/// A `SuppressionApplied` appends `"<name> is suppressed"` — the signal GTW-572 extended to
/// carry the pinned ganger, resolved to its name at the forwarder boundary.
#[test]
fn a_suppression_applied_appends_the_suppressed_line() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut()
        .write_message(gdtf_battle_sim::SuppressionApplied::new(
            ganger,
            ground(3, 3),
        ));
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex is suppressed"),
        1,
        "a SuppressionApplied must append exactly one \"Vex is suppressed\" line, got {:?}",
        line_texts(&mut app),
    );
}

/// An `ArmorBroken` appends `"<name>: armor broken"` — the GTW-572 armor-broken gain line.
#[test]
fn an_armor_broken_appends_the_armor_broken_line() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut()
        .write_message(ArmorBroken::new(ganger, BodyPart::Torso));
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex: armor broken"),
        1,
        "an ArmorBroken must append exactly one \"Vex: armor broken\" line, got {:?}",
        line_texts(&mut app),
    );
}

/// The DOT affliction logs ONCE at its start and NEVER per tick (the Q2 ruling): a
/// `DotAfflicted` appends exactly one `"<name> is afflicted (-N/turn)"` line, and a
/// subsequent mid-affliction `DotTicked` appends NO new line of any kind.
#[test]
fn a_dot_affliction_logs_once_at_start_and_never_on_a_tick() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut()
        .write_message(DotAfflicted::new(ganger, DotDamage::new(2)));
    app.update();
    assert_eq!(
        count_of(&mut app, "Vex is afflicted (-2/turn)"),
        1,
        "the affliction START must append exactly one line, got {:?}",
        line_texts(&mut app),
    );

    // A mid-affliction tick: the per-round drain signal must log NOTHING new.
    let before = line_texts(&mut app).len();
    app.world_mut()
        .write_message(DotTicked::new(ganger, ground(2, 2), DotDamage::new(2)));
    app.update();
    app.update();
    assert_eq!(
        line_texts(&mut app).len(),
        before,
        "a mid-affliction DotTicked must append NO new log line (once at start, never per \
         tick), got {:?}",
        line_texts(&mut app),
    );
}

/// The field exposure logs ONCE at its start and NEVER per round: a `FieldAfflicted`
/// appends exactly one `"<name> is caught in a hazard field"` line, and a subsequent
/// mid-exposure `FieldTicked` appends NO new line.
#[test]
fn a_field_exposure_logs_once_at_start_and_never_on_a_tick() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut()
        .write_message(FieldAfflicted::new(ganger, ground(6, 6)));
    app.update();
    assert_eq!(
        count_of(&mut app, "Vex is caught in a hazard field"),
        1,
        "the exposure START must append exactly one line, got {:?}",
        line_texts(&mut app),
    );

    let before = line_texts(&mut app).len();
    app.world_mut()
        .write_message(FieldTicked::new(ganger, ground(6, 6), FieldDamage::new(3)));
    app.update();
    app.update();
    assert_eq!(
        line_texts(&mut app).len(),
        before,
        "a mid-exposure FieldTicked must append NO new log line (once at start, never per \
         round), got {:?}",
        line_texts(&mut app),
    );
}

/// The bleed affliction logs ONCE at its start and NEVER per tick: a `BleedStarted`
/// appends exactly one `"<name> is bleeding"` line, and a subsequent mid-span `Bleeding`
/// tick appends NO new line.
#[test]
fn a_bleed_span_logs_once_at_start_and_never_on_a_tick() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut().write_message(BleedStarted::new(ganger));
    app.update();
    assert_eq!(
        count_of(&mut app, "Vex is bleeding"),
        1,
        "the bleed span START must append exactly one line, got {:?}",
        line_texts(&mut app),
    );

    let before = line_texts(&mut app).len();
    app.world_mut().write_message(Bleeding::new(ganger));
    app.update();
    app.update();
    assert_eq!(
        line_texts(&mut app).len(),
        before,
        "a mid-span Bleeding tick must append NO new log line (once at start, never per \
         tick), got {:?}",
        line_texts(&mut app),
    );
}
