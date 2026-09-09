//! Cross-level signals readback: threat badge renders red-dominant; absent slot stays dark.
use std::path::PathBuf;

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::error::warn,
    image::Image,
    platform::collections::HashSet,
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{TextureFormat, TextureUsages},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use cobalt_test_utils::{asset_plugin_at, gpu_adapter_probe};
use gdtf_battle_presenter::{
    ActiveLevel, CrossLevelBadgeTile, Layer, TopDownRendererPlugin, WORLD_RENDER_LAYER,
    cell_to_world_layered,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{BattleInProgress, Cell, CellLevel, Faction, Level, LifeState, Position},
    visibility::SquadVisibility,
};

use crate::gpu_lock::lock_gpu;

const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

const TARGET_PX: u32 = 8;

/// Frames the hand-inserted scene needs to extract, prepare and draw — per-frame work, no IO.
const SCENE_SETTLE_FRAMES: u32 = 8;

#[derive(Resource, Default, Clone, Copy)]
struct CapturedFrame {
    captured:       bool,
    mean:           [u8; 4],
    max_red_excess: i16,
}

fn workspace_assets_root() -> PathBuf {
    let Some(root) = cobalt_ron_assets::workspace_assets_root() else {
        unreachable!("found no `Cargo.lock` or `[workspace]` manifest above the crate");
    };
    root
}

fn build_render_app() -> Option<App> {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(asset_plugin_at(&workspace_assets_root()))
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    )
    .add_plugins(TopDownRendererPlugin);
    app.insert_resource(BattleInProgress);
    app.set_error_handler(warn);

    app.finish();
    app.cleanup();

    app.get_sub_app(RenderApp)?;
    Some(app)
}

fn render_badge_slot(present: bool) -> Option<CapturedFrame> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedFrame>();

    let active_cell = CellLevel::new(Cell::new(0, 0), Level::new(0));
    let enemy_cell = CellLevel::new(Cell::new(0, 0), Level::new(2));

    app.world_mut()
        .insert_resource(ActiveLevel::new(active_cell.level()));
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));

    if present {
        let visible: HashSet<CellLevel> = std::iter::once(enemy_cell).collect();
        app.world_mut()
            .insert_resource(SquadVisibility::new(visible.clone(), visible));
        app.world_mut()
            .spawn((Position::new(enemy_cell), Faction::new(1), LifeState::Alive));
    } else {
        app.world_mut()
            .insert_resource(SquadVisibility::new(HashSet::default(), HashSet::default()));
    }

    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    let badge_world = cell_to_world_layered(
        active_cell.cell(),
        active_cell.level(),
        Layer::CrossLevelSignal,
    ) + Vec3::new(5.0, 5.0, 0.0);

    let target_for_cam = target_handle.clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(DARK_CLEAR),
            ..default()
        },
        Transform::from_translation(badge_world),
        RenderTarget::Image(target_for_cam.into()),
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));

    // Let the scene draw before any copy is submitted, so every readback is of a settled frame.
    for _ in 0..SCENE_SETTLE_FRAMES {
        app.update();
    }

    app.world_mut()
        .spawn(Readback::texture(target_handle))
        .observe(
            |trigger: On<ReadbackComplete>, mut captured: ResMut<CapturedFrame>| {
                let data = &trigger.event().data;
                let row_stride = data.len() / TARGET_PX as usize;
                let edge = TARGET_PX as usize;
                let mut sums = [0u32; 4];
                let mut count = 0u32;
                let mut max_red_excess = i16::MIN;
                for cy in 0..edge {
                    for cx in 0..edge {
                        let off = cy * row_stride + cx * 4;
                        if let (Some(&r), Some(&g), Some(&b), Some(&a)) = (
                            data.get(off),
                            data.get(off + 1),
                            data.get(off + 2),
                            data.get(off + 3),
                        ) {
                            sums[0] += u32::from(r);
                            sums[1] += u32::from(g);
                            sums[2] += u32::from(b);
                            sums[3] += u32::from(a);
                            count += 1;
                            let excess = i16::from(r) - i16::from(g).max(i16::from(b));
                            max_red_excess = max_red_excess.max(excess);
                        }
                    }
                }
                let mean = |sum: u32| {
                    sum.checked_div(count)
                        .and_then(|m| u8::try_from(m).ok())
                        .unwrap_or(0)
                };
                if count > 0 {
                    captured.captured = true;
                    captured.mean = [mean(sums[0]), mean(sums[1]), mean(sums[2]), mean(sums[3])];
                    captured.max_red_excess = max_red_excess;
                }
            },
        );

    while !app.world().resource::<CapturedFrame>().captured {
        app.update();
    }

    let captured = *app.world().resource::<CapturedFrame>();
    if present {
        let mut q = app.world_mut().query::<&CrossLevelBadgeTile>();
        assert!(
            q.iter(app.world()).next().is_some(),
            "the REAL draw_cross_level_signals must have spawned the pooled CrossLevelBadgeTile",
        );
    }
    Some(captured)
}

#[test]
fn threat_badge_slot_renders_red_dominant_absent_slot_stays_dark() {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — cross-level-signal pixel proof \
             not run"
        );
        return;
    }

    let Some(absent) = render_badge_slot(false) else {
        eprintln!(
            "SKIP: no GPU adapter in this environment — cross-level-signal pixel proof not run"
        );
        return;
    };
    let Some(present) = render_badge_slot(true) else {
        eprintln!(
            "SKIP: no GPU adapter in this environment — cross-level-signal pixel proof not run"
        );
        return;
    };

    assert!(
        absent.mean[0] < 40 && absent.mean[1] < 40 && absent.mean[2] < 40,
        "with no cross-level signal the badge slot must render the DARK clear colour: \
         got mean {:?}",
        absent.mean,
    );
    assert!(
        absent.max_red_excess < 20,
        "with no cross-level signal the badge slot must have NO red-dominant texel: \
         got max red-excess {}",
        absent.max_red_excess,
    );

    assert!(
        present.max_red_excess > absent.max_red_excess + 100,
        "a squad-visible enemy two storeys above must render a RED-DOMINANT badge texel: \
         present max red-excess {} vs absent {}",
        present.max_red_excess,
        absent.max_red_excess,
    );
}
