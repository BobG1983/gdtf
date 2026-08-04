//! Reticle recolour: visible vs non-visible cells render distinct GPU pixels.
use std::sync::{Mutex, MutexGuard};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    camera::{RenderTarget, visibility::RenderLayers},
    ecs::message::Messages,
    image::Image,
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::{TextureFormat, TextureUsages},
    },
    window::{ExitCondition, WindowPlugin},
    winit::WinitPlugin,
};
use gdtf_battle_presenter::{
    CellVisibility, HighlightRequest, HoverHighlight, WORLD_RENDER_LAYER, cell_to_world,
    draw_highlight_on_request,
};
use gdtf_battle_sim::prelude::{BattleInProgress, Cell, CellLevel, Level};
use gdtf_test_utils::gpu_adapter_probe;

const TARGET_PX: u32 = 64;

const MAX_READBACK_UPDATES: usize = 60;

#[derive(Resource, Default, Clone, Copy)]
struct CapturedPixel {
    captured: bool,
    rgba:     [u8; 4],
}

static GPU_LOCK: Mutex<()> = Mutex::new(());

fn lock_gpu() -> MutexGuard<'static, ()> {
    GPU_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
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
            .disable::<WinitPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
            .disable::<bevy::log::LogPlugin>()
            .disable::<bevy::audio::AudioPlugin>(),
    );
    app.add_message::<HighlightRequest>()
        .insert_resource(BattleInProgress)
        .add_systems(Update, draw_highlight_on_request);

    app.finish();
    app.cleanup();
    app.get_sub_app(RenderApp)?;
    Some(app)
}

fn render_reticle(cell: CellLevel, verdict: CellVisibility) -> Option<[u8; 4]> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    let centre = cell_to_world(cell.cell(), Level::new(0));
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(1.0, 0.0, 1.0)),
            ..default()
        },
        Transform::from_translation(centre),
        RenderTarget::Image(target_handle.clone().into()),
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));

    app.world_mut()
        .resource_mut::<Messages<HighlightRequest>>()
        .write(HighlightRequest::new(Some(cell), verdict));

    app.world_mut()
        .spawn(Readback::texture(target_handle))
        .observe(
            |trigger: On<ReadbackComplete>, mut captured: ResMut<CapturedPixel>| {
                let data = &trigger.event().data;
                let row_stride = data.len() / TARGET_PX as usize;
                let cx = (TARGET_PX / 2) as usize;
                let cy = (TARGET_PX / 2) as usize;
                let off = cy * row_stride + cx * 4;
                if let (Some(&r), Some(&g), Some(&b), Some(&a)) = (
                    data.get(off),
                    data.get(off + 1),
                    data.get(off + 2),
                    data.get(off + 3),
                ) {
                    captured.captured = true;
                    captured.rgba = [r, g, b, a];
                }
            },
        );

    for _ in 0..MAX_READBACK_UPDATES {
        app.update();
    }

    let captured = *app.world().resource::<CapturedPixel>();
    assert!(
        captured.captured,
        "GPU readback never fired within {MAX_READBACK_UPDATES} updates"
    );
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<HoverHighlight>>();
    assert_eq!(
        q.iter(app.world()).count(),
        1,
        "exactly one reticle sprite must exist (the draw mutates the one in place)"
    );
    Some(captured.rgba)
}

#[test]
fn reticle_recolours_on_the_gpu_for_a_non_visible_cell() {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — reticle recolour proof not run"
        );
        return;
    }

    let cell = CellLevel::new(Cell::new(8, 8), Level::new(0));

    let Some([vr, vg, vb, va]) = render_reticle(cell, CellVisibility::SquadVisible) else {
        eprintln!("SKIP: no GPU adapter in this environment — reticle recolour proof not run");
        return;
    };
    let Some([ur, ug, ub, ua]) = render_reticle(cell, CellVisibility::NotSquadVisible) else {
        eprintln!("SKIP: no GPU adapter in this environment — reticle recolour proof not run");
        return;
    };

    assert!(va >= 64, "VISIBLE reticle pixel looks unpainted: a={va}");
    assert!(
        ua >= 64,
        "non-VISIBLE reticle pixel looks unpainted: a={ua}"
    );

    let channel_diff = (i16::from(vr) - i16::from(ur)).abs()
        + (i16::from(vg) - i16::from(ug)).abs()
        + (i16::from(vb) - i16::from(ub)).abs();
    assert!(
        channel_diff > 15,
        "the VISIBLE tint ({vr},{vg},{vb}) and the non-VISIBLE tint ({ur},{ug},{ub}) must render \
         DISTINCT pixels (the reticle recolours)",
    );

    let visible_warm = i16::from(vr) - i16::from(vb);
    let unseen_cold = i16::from(ur) - i16::from(ub);
    assert!(
        unseen_cold < visible_warm,
        "the non-VISIBLE tint must be COLDER (less red-over-blue) than the VISIBLE tint: \
         unseen R-B={unseen_cold}, visible R-B={visible_warm}",
    );
}
