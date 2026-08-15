//! Combat log: each Played fact appends the right line once.
use bevy::{ecs::entity::Entity, prelude::*, state::state::State};
use gdtf_app::test_support::{AppState, BattleScapeState, CombatLogLine, RunningState};
use gdtf_battle_presenter::Played;
use gdtf_battle_sim::{
    acts::MeleeStruck,
    armor::BodyPart,
    armor_wear::ArmorBroken,
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldDamage, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::{FallOccurred, StoreysFallen},
    ganger::GangerName,
    injuries::InjuryRegistry,
    prelude::{Cell, CellLevel, Level},
    resolve_hit::HpDamage,
    tuning::CombatTuning,
    weapon::DotDamage,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// Fixed steps a frame runs, so the descent advances per frame and never off the real clock.
const ONE_STEP_A_FRAME: u32 = 1;

fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

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
        .insert_resource(gdtf_battle_sim::equipment::attachments::AttachmentRegistry::default());
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
    app.insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(
        ONE_STEP_A_FRAME,
    ));

    advance_until(&mut app, |app| {
        running_state(app) == Some(RunningState::Menu)
    });
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    advance_until(&mut app, |app| {
        battlescape_state(app) == Some(BattleScapeState::BattleRunning)
    });
    app
}

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

fn count_of(app: &mut App, text: &str) -> usize {
    line_texts(app).into_iter().filter(|t| t == text).count()
}

fn spawn_named(app: &mut App, name: &str) -> Entity {
    app.world_mut().spawn(GangerName::new(name.to_owned())).id()
}

fn play<M: Message + Clone>(app: &mut App, fact: M) {
    let written = app.world_mut().write_message(Played::new(fact)).is_some();
    assert!(
        written,
        "the Played<{}> buffer must be registered by the presenter's playback registration",
        core::any::type_name::<M>(),
    );
}

fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

#[test]
fn a_fall_occurred_appends_the_fell_line() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(
        &mut app,
        FallOccurred::new(ganger, Level::new(2), Level::new(0), StoreysFallen::new(2)),
    );
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex fell 2 storeys"),
        1,
        "a FallOccurred must append exactly one \"Vex fell 2 storeys\" line, got {:?}",
        line_texts(&mut app),
    );
}

#[test]
fn a_melee_struck_appends_the_damage_line() {
    let mut app = battle_running_app();
    let attacker = spawn_named(&mut app, "Vex");
    let target = spawn_named(&mut app, "Skar");
    app.update();

    play(
        &mut app,
        MeleeStruck::new(attacker, target, HpDamage::new(4)),
    );
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex struck Skar (-4)"),
        1,
        "a MeleeStruck must append exactly one \"Vex struck Skar (-4)\" line, got {:?}",
        line_texts(&mut app),
    );
}

#[test]
fn an_on_death_appends_the_named_dies_line_and_a_cover_death_does_not() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(&mut app, OnDeathOccurred::new(ganger, ground(4, 4)));
    app.update();
    assert_eq!(
        count_of(&mut app, "Vex dies"),
        1,
        "a ganger death must append exactly one \"Vex dies\" line, got {:?}",
        line_texts(&mut app),
    );

    play(&mut app, OnDeathOccurred::cover(ground(5, 5)));
    app.update();
    app.update();
    assert_eq!(
        count_of(&mut app, "Someone dies"),
        0,
        "a cover death (placeholder entity) must NOT log a phantom \"Someone dies\", got {:?}",
        line_texts(&mut app),
    );
}

#[test]
fn a_suppression_applied_appends_the_suppressed_line() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(
        &mut app,
        gdtf_battle_sim::suppression::SuppressionApplied::new(ganger, ground(3, 3)),
    );
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex is suppressed"),
        1,
        "a SuppressionApplied must append exactly one \"Vex is suppressed\" line, got {:?}",
        line_texts(&mut app),
    );
}

#[test]
fn an_armor_broken_appends_the_armor_broken_line() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(&mut app, ArmorBroken::new(ganger, BodyPart::Torso));
    app.update();

    assert_eq!(
        count_of(&mut app, "Vex: armor broken"),
        1,
        "an ArmorBroken must append exactly one \"Vex: armor broken\" line, got {:?}",
        line_texts(&mut app),
    );
}

#[test]
fn a_dot_affliction_logs_once_at_start_and_never_on_a_tick() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(&mut app, DotAfflicted::new(ganger, DotDamage::new(2)));
    app.update();
    assert_eq!(
        count_of(&mut app, "Vex is afflicted (-2/turn)"),
        1,
        "the affliction START must append exactly one line, got {:?}",
        line_texts(&mut app),
    );

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

#[test]
fn a_field_exposure_logs_once_at_start_and_never_on_a_tick() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(&mut app, FieldAfflicted::new(ganger, ground(6, 6)));
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

#[test]
fn a_bleed_span_logs_once_at_start_and_never_on_a_tick() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    play(&mut app, BleedStarted::new(ganger));
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
