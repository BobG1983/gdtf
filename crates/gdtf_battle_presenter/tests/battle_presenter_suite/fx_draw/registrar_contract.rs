use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    ecs::{error::warn, message::Messages},
    prelude::{Text2d, default},
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    transform::components::Transform,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::asset_plugin_at;
use gdtf_battle_presenter::{FctValence, FloatingCombatText, TopDownRendererPlugin, valence_color};
use gdtf_battle_sim::{
    acts::{InjuryInflicted, MeleeResolved, ThrowResolved},
    armor_wear::ArmorBroken,
    effects::{bleed::Bleeding, dot::DotTicked, on_death::OnDeathOccurred},
    falls::FallOccurred,
    occupancy_sync::TerrainPieceDestroyed,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    weapon::DotDamage,
};

use super::{harness::*, probes::*};

#[test]
fn two_families_on_one_cell_across_consecutive_frames_take_distinct_stack_slots() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));

    let cell = Cell::new(9, 9);
    let level = Level::new(0);
    let at = CellLevel::new(cell, level);
    let pinned = wounded_ganger(&mut app, cell, level, 2);

    play(&mut app, SuppressionApplied::new(pinned, at));
    app.update();

    play(&mut app, DotTicked::new(pinned, at, DotDamage::new(4)));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "SUPPRESSED", valence_color(FctValence::Suppressed)),
        "the suppression family must pop its tag, got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "-4", valence_color(FctValence::Dot)),
        "the DOT family must pop its number, got {pops:?}",
    );

    let mut q = app
        .world_mut()
        .query::<(&Text2d, &FloatingCombatText, &Transform)>();
    let mut suppressed_y: Option<f32> = None;
    let mut dot_y: Option<f32> = None;
    for (text, _, transform) in q.iter(app.world()) {
        match text.as_str() {
            "SUPPRESSED" => suppressed_y = Some(transform.translation.y),
            "-4" => dot_y = Some(transform.translation.y),
            _ => {}
        }
    }
    let (Some(suppressed_y), Some(dot_y)) = (suppressed_y, dot_y) else {
        return;
    };
    assert!(
        (suppressed_y - dot_y).abs() > 0.001,
        "a family popping a cell that already carries a live pop from a prior frame must take a \
         DISTINCT stack slot (distinct ys), got {suppressed_y} == {dot_y} — the lifetime-aware \
         FctSlotAllocator must hand the second-frame pop slot 1",
    );
}

#[test]
fn a_presenter_only_app_with_no_family_buffers_stays_inert() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(asset_plugin_at(&workspace_assets_root())),
    )
    .add_plugins(TopDownRendererPlugin);
    app.set_error_handler(warn);
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        fct_pop_count(&mut app),
        0,
        "with no family Messages<M> buffer registered, no consequence pop may spawn",
    );
    assert!(
        app.world()
            .get_resource::<Messages<gdtf_battle_sim::effects::fields::FieldTicked>>()
            .is_none(),
        "the presenter must not register Messages<FieldTicked> itself",
    );
    assert!(
        app.world()
            .get_resource::<Messages<OnDeathOccurred>>()
            .is_none(),
        "the presenter must not register Messages<OnDeathOccurred> itself",
    );
    for absent in [
        app.world()
            .get_resource::<Messages<Bleeding>>()
            .map(|_| "Bleeding"),
        app.world()
            .get_resource::<Messages<ArmorBroken>>()
            .map(|_| "ArmorBroken"),
        app.world()
            .get_resource::<Messages<SuppressionApplied>>()
            .map(|_| "SuppressionApplied"),
        app.world()
            .get_resource::<Messages<DotTicked>>()
            .map(|_| "DotTicked"),
        app.world()
            .get_resource::<Messages<InjuryInflicted>>()
            .map(|_| "InjuryInflicted"),
        app.world()
            .get_resource::<Messages<MeleeResolved>>()
            .map(|_| "MeleeResolved"),
        app.world()
            .get_resource::<Messages<FallOccurred>>()
            .map(|_| "FallOccurred"),
        app.world()
            .get_resource::<Messages<ThrowResolved>>()
            .map(|_| "ThrowResolved"),
        app.world()
            .get_resource::<Messages<TerrainPieceDestroyed>>()
            .map(|_| "TerrainPieceDestroyed"),
    ] {
        assert!(
            absent.is_none(),
            "the presenter registrar must not register the {absent:?} family buffer",
        );
    }
}

#[test]
fn the_presenter_registers_exactly_the_one_documented_sim_buffer_exception() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::app::TerminalCtrlCHandlerPlugin>()
            .disable::<bevy::gizmos::GizmoPlugin>()
            .disable::<bevy::audio::AudioPlugin>()
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(asset_plugin_at(&workspace_assets_root())),
    )
    .add_plugins(TopDownRendererPlugin);
    app.set_error_handler(warn);
    app.update();

    assert!(
        app.world().get_resource::<Messages<ShotFired>>().is_some(),
        "the presenter must register Messages<ShotFired> itself — the one documented sim-owned add_message exception so update_ganger_life_state keeps running in fire-less harnesses",
    );
}
