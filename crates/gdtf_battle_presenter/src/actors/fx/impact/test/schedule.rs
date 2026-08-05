use std::{path::PathBuf, time::Duration};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    asset::AssetPlugin,
    ecs::error::warn,
    math::Vec3,
    prelude::default,
    render::{RenderPlugin, settings::WgpuSettings},
    time::TimeUpdateStrategy,
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_sim::{prelude::BattleInProgress, weapon::DamageType};
use gdtf_test_utils::advance_until_resource_exists;

use super::super::animation::{ImpactAnimation, ImpactStep};
use crate::{
    EffectRoles, FxTuning, IMPACT_FRAME_COUNT, PendingImpact, TopDownAtlases, TopDownRendererPlugin,
};

// The asset loads resolve in a few frames; this cap is a safety net, not a budget.
const LOAD_SAFETY_NET: u32 = 10_000;

fn workspace_assets_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
}

// The real top-down renderer, headless, with the shipped FX assets loaded.
fn impact_fx_app() -> App {
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
    .add_plugins(TopDownRendererPlugin);
    app.set_error_handler(warn);
    advance_until_resource_exists::<EffectRoles>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<TopDownAtlases>(&mut app, LOAD_SAFETY_NET);
    advance_until_resource_exists::<FxTuning>(&mut app, LOAD_SAFETY_NET);
    app.world_mut().insert_resource(BattleInProgress);
    app
}

// A zero-length tick reports the frame on screen without moving the animation on.
fn shown_frames(app: &mut App) -> Vec<usize> {
    let mut playing = app.world_mut().query::<&mut ImpactAnimation>();
    playing
        .iter_mut(app.world_mut())
        .filter_map(|mut anim| match anim.advance(Duration::ZERO) {
            ImpactStep::Showing(frame) => Some(frame),
            ImpactStep::Finished => None,
        })
        .collect()
}

#[test]
fn a_flash_seeded_this_frame_does_not_advance_until_the_next() {
    let mut app = impact_fx_app();
    let frame_hold = *app.world().resource::<FxTuning>().impact_frame_seconds;
    let step = Duration::from_secs_f32(frame_hold + 0.001);
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(step));

    app.world_mut()
        .spawn(PendingImpact::for_blast(Vec3::ZERO, DamageType::Kinetic));
    app.update();

    assert_eq!(
        shown_frames(&mut app),
        vec![0],
        "the update that seeds a flash must leave it on frame 0 — a sync point between \
         seed_impact_animations and advance_impact_animations flushes the spawn and ticks \
         the flash on its own spawn frame, costing it a frame of life",
    );

    for _ in 1..IMPACT_FRAME_COUNT {
        app.update();
    }
    assert_eq!(
        shown_frames(&mut app).len(),
        1,
        "the flash must still be playing {} updates after it was seeded — it gets all \
         {IMPACT_FRAME_COUNT} frames counted from the update AFTER its spawn frame",
        IMPACT_FRAME_COUNT - 1,
    );

    app.update();
    assert!(
        shown_frames(&mut app).is_empty(),
        "the flash must despawn once its {IMPACT_FRAME_COUNT}-frame window has elapsed",
    );
}
