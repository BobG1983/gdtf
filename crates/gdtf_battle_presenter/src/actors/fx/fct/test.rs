use std::time::Duration;

use bevy::{
    MinimalPlugins,
    app::{App, SpawnScene, Update},
    asset::AssetPlugin,
    ecs::system::RunSystemOnce,
    prelude::{Alpha, Color, Commands, Transform},
    scene::ScenePlugin,
    text::{FontSize, FontWeight, TextColor, TextFont},
    time::TimeUpdateStrategy,
};
use gdtf_battle_sim::{
    prelude::{Cell, Level},
    severity::Severity,
};

use super::{
    super::tuning::{FctRiseRate, FctTtlSeconds},
    palette::{FctValence, severity_color, valence_color},
    text::{
        CombatText, FctEmphasis, FctStackIndex, FloatingCombatText, animate_floating_text,
        spawn_floating_text,
    },
};

const TEST_TTL: FctTtlSeconds = FctTtlSeconds::new(0.6);

fn fct_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_systems(Update, animate_floating_text);
    app
}

fn materialize_scenes(app: &mut App) {
    app.world_mut().flush();
    app.world_mut().run_schedule(SpawnScene);
}

fn spawn_pop(app: &mut App, color: Color, cell: Cell, level: Level, stack: FctStackIndex) {
    spawn_pop_with(app, color, FctEmphasis::Normal, cell, level, stack);
}

fn spawn_pop_with(
    app: &mut App,
    color: Color,
    emphasis: FctEmphasis,
    cell: Cell,
    level: Level,
    stack: FctStackIndex,
) {
    let ran = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_floating_text(
                &mut commands,
                CombatText::new("-7"),
                color,
                emphasis,
                cell,
                level,
                stack,
                TEST_TTL,
                FctRiseRate::default(),
            );
        });
    assert!(
        ran.is_ok(),
        "the one-shot spawn system must run successfully"
    );
    materialize_scenes(app);
}

fn single_pop(app: &mut App) -> Option<(bevy::math::Vec3, f32)> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Transform, &TextColor)>();
    let mut found: Option<(bevy::math::Vec3, f32)> = None;
    for (_, transform, color) in q.iter(app.world()) {
        if found.is_some() {
            return None;
        }
        found = Some((transform.translation, color.0.alpha()));
    }
    found
}

fn pop_count(app: &mut App) -> usize {
    let mut q = app.world_mut().query::<&FloatingCombatText>();
    q.iter(app.world()).count()
}

fn single_pop_font(app: &mut App) -> Option<(FontWeight, f32)> {
    let mut q = app.world_mut().query::<(&FloatingCombatText, &TextFont)>();
    let mut found: Option<(FontWeight, f32)> = None;
    for (_, font) in q.iter(app.world()) {
        if found.is_some() {
            return None;
        }
        let FontSize::Px(size) = font.font_size else {
            return None;
        };
        found = Some((font.weight, size));
    }
    found
}

#[test]
fn a_pop_rises_then_fades_then_despawns() {
    let mut app = fct_app();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));

    let cell = Cell::new(4, 6);
    let level = Level::new(0);
    spawn_pop(
        &mut app,
        Color::srgba(0.9, 0.13, 0.10, 1.0),
        cell,
        level,
        FctStackIndex::BASE,
    );
    assert_eq!(pop_count(&mut app), 1, "exactly one pop must spawn");

    app.update();
    let first = single_pop(&mut app);
    assert!(first.is_some(), "the pop must still be live after one tick");
    let Some((first_pos, first_alpha)) = first else {
        return;
    };

    app.update();
    let second = single_pop(&mut app);
    assert!(
        second.is_some(),
        "the pop must still be live after two ticks"
    );
    let Some((second_pos, second_alpha)) = second else {
        return;
    };
    assert!(
        second_pos.y > first_pos.y,
        "the pop must RISE: y must increase ({} -> {})",
        first_pos.y,
        second_pos.y,
    );
    assert!(
        second_alpha < first_alpha,
        "the pop must FADE: alpha must decrease ({first_alpha} -> {second_alpha})",
    );

    for _ in 0..8 {
        app.update();
    }
    assert_eq!(
        pop_count(&mut app),
        0,
        "the pop must despawn once its FctTtlSeconds lifetime elapses",
    );
}

#[test]
fn a_higher_stack_index_offsets_the_pop_downward() {
    let base_y = {
        let mut app = fct_app();
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        spawn_pop(
            &mut app,
            Color::WHITE,
            Cell::new(2, 2),
            Level::new(0),
            FctStackIndex::BASE,
        );
        single_pop(&mut app).map(|(pos, _)| pos.y)
    };
    let stacked_y = {
        let mut app = fct_app();
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        spawn_pop(
            &mut app,
            Color::WHITE,
            Cell::new(2, 2),
            Level::new(0),
            FctStackIndex::new(2),
        );
        single_pop(&mut app).map(|(pos, _)| pos.y)
    };
    assert!(
        base_y.is_some() && stacked_y.is_some(),
        "both pops must spawn"
    );
    let (Some(base_y), Some(stacked_y)) = (base_y, stacked_y) else {
        return;
    };
    assert!(
        stacked_y < base_y,
        "a higher stack index must offset the pop DOWNWARD ({base_y} -> {stacked_y})",
    );
}

#[test]
fn a_bold_pop_is_drawn_heavier_than_a_normal_pop() {
    let normal = {
        let mut app = fct_app();
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        spawn_pop_with(
            &mut app,
            Color::WHITE,
            FctEmphasis::Normal,
            Cell::new(1, 1),
            Level::new(0),
            FctStackIndex::BASE,
        );
        single_pop_font(&mut app)
    };
    let bold = {
        let mut app = fct_app();
        app.world_mut()
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
        spawn_pop_with(
            &mut app,
            Color::WHITE,
            FctEmphasis::Bold,
            Cell::new(1, 1),
            Level::new(0),
            FctStackIndex::BASE,
        );
        single_pop_font(&mut app)
    };
    assert!(
        normal.is_some() && bold.is_some(),
        "both pops must spawn exactly one entity"
    );
    let (Some((normal_weight, normal_size)), Some((bold_weight, bold_size))) = (normal, bold)
    else {
        return;
    };
    assert!(
        bold_weight.0 > normal_weight.0,
        "a bold pop must carry a heavier FontWeight ({} -> {})",
        normal_weight.0,
        bold_weight.0,
    );
    assert_eq!(
        bold_weight,
        FontWeight::BOLD,
        "a bold pop must be drawn in FontWeight::BOLD",
    );
    assert!(
        bold_size > normal_size,
        "a bold pop must be drawn LARGER ({normal_size} -> {bold_size})",
    );
}

#[test]
fn valence_color_maps_each_valence_to_its_family() {
    assert_eq!(
        valence_color(FctValence::Damage),
        valence_color(FctValence::Lethal),
        "damage + lethal must share the blood-red family",
    );
    let damage = valence_color(FctValence::Damage);
    let wound = valence_color(FctValence::Status);
    let neutral = valence_color(FctValence::Neutral);
    assert_ne!(damage, wound, "damage red must differ from wound amber");
    assert_ne!(wound, neutral, "wound amber must differ from neutral grey");
    assert_ne!(damage, neutral, "damage red must differ from neutral grey");
}

#[test]
fn severity_color_ramps_through_the_wound_family_to_lethal() {
    assert_eq!(
        severity_color(Severity::None),
        valence_color(FctValence::Neutral),
        "a graze (Severity::None) must read NEUTRAL, not a wound amber",
    );
    assert_eq!(
        severity_color(Severity::Fatal),
        valence_color(FctValence::Lethal),
        "a Fatal severity must read the LETHAL red, not a wound amber",
    );
    let minor = severity_color(Severity::Minor);
    let major = severity_color(Severity::Major);
    let critical = severity_color(Severity::Critical);
    assert_ne!(minor, major, "the wound ramp must climb (Minor != Major)");
    assert_ne!(
        major, critical,
        "the wound ramp must climb (Major != Critical)"
    );
    let neutral = severity_color(Severity::None);
    let lethal = severity_color(Severity::Fatal);
    for (tier, color) in [
        (Severity::Minor, minor),
        (Severity::Major, major),
        (Severity::Critical, critical),
    ] {
        assert_ne!(
            color, neutral,
            "a wounding tier ({tier:?}) must not read NEUTRAL",
        );
        assert_ne!(
            color, lethal,
            "a wounding tier ({tier:?}) must not read the LETHAL red",
        );
    }
}
