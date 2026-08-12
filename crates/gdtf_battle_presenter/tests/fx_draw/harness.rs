use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::{error::warn, message::Messages},
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    CharacterRoles, EffectRoles, FxTuning, Played, ShotImpactResolved, TopDownAtlases,
    TopDownRendererPlugin,
};
use gdtf_battle_sim::{
    acts::{MeleeResolved, ThrowResolved},
    armor_wear::ArmorBroken,
    effects::{bleed::Bleeding, dot::DotTicked},
    falls::FallOccurred,
    occupancy_sync::CoverDestroyed,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
};
use gdtf_test_utils::advance_until_resource_exists;

pub(crate) const LOAD_SAFETY_NET: u32 = 10_000;

pub(crate) fn workspace_assets_root() -> PathBuf {
    let Some(root) = gdtf_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

pub(crate) fn headless_renderer_app() -> App {
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
            .set(AssetPlugin {
                file_path: workspace_assets_root().to_string_lossy().into_owned(),
                ..default()
            }),
    )
    .add_message::<Bleeding>()
    .add_message::<ArmorBroken>()
    .add_message::<CoverDestroyed>()
    .add_message::<ShotFired>()
    .add_message::<SuppressionApplied>()
    .add_message::<DotTicked>()
    .add_message::<MeleeResolved>()
    .add_message::<FallOccurred>()
    .add_message::<ThrowResolved>()
    .add_plugins(TopDownRendererPlugin);
    app.set_error_handler(warn);
    app
}

pub(crate) fn play<M: bevy::ecs::message::Message + Clone>(app: &mut App, fact: M) {
    let written = app.world_mut().write_message(Played::new(fact)).is_some();
    assert!(
        written,
        "the Played<{}> buffer must be registered by the presenter's playback registration",
        core::any::type_name::<M>(),
    );
}

pub(crate) fn settle_resources(app: &mut App) {
    advance_until_resource_exists::<EffectRoles>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<FxTuning>(app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<CharacterRoles>(app, LOAD_SAFETY_NET);
}

pub(crate) fn effect_roles(app: &App) -> Option<EffectRoles> {
    app.world().get_resource::<EffectRoles>().cloned()
}

pub(crate) fn advance_past_ttl(app: &mut App) {
    const STEP: std::time::Duration = std::time::Duration::from_millis(250);
    const STEPS: u32 = 8;

    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(STEP));
    for _ in 0..STEPS {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

pub(crate) fn fire_with_zero_delta(app: &mut App) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
    app.update();
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

pub(crate) fn step_until_pop(
    app: &mut App,
    text: &str,
    step: std::time::Duration,
    max_steps: u32,
) -> Option<Vec<(String, f32)>> {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    let mut found = None;
    for _ in 0..max_steps {
        app.update();
        let snapshot = super::probes::fct_pops_with_y(app);
        if snapshot.iter().any(|(t, _)| t == text) {
            found = Some(snapshot);
            break;
        }
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    found
}

pub(crate) fn step_app(app: &mut App, step: std::time::Duration, updates: u32) {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..updates {
        app.update();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
}

pub(crate) fn drain_impacts(app: &mut App) -> Vec<ShotImpactResolved> {
    app.world_mut()
        .resource_mut::<Messages<ShotImpactResolved>>()
        .drain()
        .collect()
}

pub(crate) fn step_counting_impacts(
    app: &mut App,
    step: std::time::Duration,
    updates: u32,
) -> usize {
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));
    let mut total = 0;
    for _ in 0..updates {
        app.update();
        total += drain_impacts(app).len();
    }
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    total
}
