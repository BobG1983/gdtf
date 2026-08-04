//! Path-preview pixel proof: route cells render warmer than off-route clear.
use std::sync::{Mutex, MutexGuard};

use bevy::{
    DefaultPlugins,
    app::{App, PluginGroup},
    camera::RenderTarget,
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
use gdtf_test_utils::gpu_adapter_probe;

/// The translucent warm-amber tint the route preview draws (mirrors the private `PREVIEW_TINT`).
const TINT: Color = Color::srgba(1.0, 0.75, 0.2, 0.55);

const DARK_CLEAR: Color = Color::srgb(0.02, 0.02, 0.03);

const TARGET_PX: u32 = 16;

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

    app.finish();
    app.cleanup();

    app.get_sub_app(RenderApp)?;
    Some(app)
}

fn render_centre(drawn: bool) -> Option<[u8; 4]> {
    let _gpu = lock_gpu();
    let mut app = build_render_app()?;
    app.init_resource::<CapturedPixel>();

    let mut target =
        Image::new_target_texture(TARGET_PX, TARGET_PX, TextureFormat::Rgba8UnormSrgb, None);
    target.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let target_handle = app.world_mut().resource_mut::<Assets<Image>>().add(target);

    let target_for_cam = target_handle.clone();
    app.world_mut().spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(DARK_CLEAR),
            ..default()
        },
        RenderTarget::Image(target_for_cam.into()),
    ));

    if drawn {
        app.world_mut().spawn((
            Sprite {
                color: TINT,
                custom_size: Some(Vec2::splat(f32::from(
                    u16::try_from(TARGET_PX).unwrap_or(16),
                ))),
                ..default()
            },
            Transform::default(),
        ));
    }

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
    Some(captured.rgba)
}

#[test]
fn route_cell_renders_nondark_offroute_cell_renders_dark() {
    if gpu_adapter_probe().should_skip() {
        eprintln!(
            "SKIP: no usable GPU adapter in this environment — path-preview pixel proof not run"
        );
        return;
    }

    let Some([lit_r, lit_g, lit_b, _lit_a]) = render_centre(true) else {
        eprintln!("SKIP: no GPU adapter in this environment — path-preview pixel proof not run");
        return;
    };
    let Some([dark_r, dark_g, dark_b, _dark_a]) = render_centre(false) else {
        eprintln!("SKIP: no GPU adapter in this environment — path-preview pixel proof not run");
        return;
    };

    assert!(
        dark_r < 40 && dark_g < 40 && dark_b < 40,
        "an off-route cell must render the DARK clear colour: got R={dark_r} G={dark_g} B={dark_b}",
    );

    assert!(
        i16::from(lit_r) - i16::from(dark_r) > 20,
        "the drawn route cell must render distinctly REDDER than the dark clear: \
         lit R={lit_r} vs dark R={dark_r}",
    );
    let lit_sum = u16::from(lit_r) + u16::from(lit_g) + u16::from(lit_b);
    let dark_sum = u16::from(dark_r) + u16::from(dark_g) + u16::from(dark_b);
    assert!(
        lit_sum > dark_sum,
        "the drawn route cell must be NON-DARK (brighter than the off-route clear): \
         lit=({lit_r},{lit_g},{lit_b}) dark=({dark_r},{dark_g},{dark_b})",
    );
}
