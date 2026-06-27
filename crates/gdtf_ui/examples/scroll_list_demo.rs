//! GTW-412 in-engine QA demo for the [`ScrollList`](gdtf_ui::ScrollList) widget (AC3).
//!
//! A tiny `DefaultPlugins` app that spawns a themed PANEL and, inside a SHORT container over
//! it, a [`ScrollList`](gdtf_ui::ScrollList) filled with ~15 visibly-distinct rows — far more
//! than fit, so the content overflows and clips. After a settle it SCROLLS the list partway
//! by driving a real wheel [`Pointer<Scroll>`](bevy::picking::events::Pointer) over several
//! frames (the production [`ScrollAreaPlugin`](bevy::ui_widgets::ScrollAreaPlugin) observer),
//! so QA sees the clipped + scrolled state: the top rows clipped OUT, a mid set shown, and the
//! scrollbar thumb offset down its track.
//!
//! ## How QA runs it
//!
//! Scroll the list partway and capture a PNG, then exit (no manual input):
//!
//! ```text
//! GDTF_SCROLL_SHOT=/abs/out.png cargo run -p gdtf_ui --example scroll_list_demo
//! ```
//!
//! Drop the `GDTF_SCROLL_SHOT` env var to just watch it live. `Read` the PNG to confirm: the
//! list clips its rows to the short container, a MIDDLE band of rows is visible (the first
//! rows scrolled off the top), and the scrollbar thumb sits partway down its track. The
//! capture path is purely a dev affordance for this example — it is NOT compiled into any
//! shipped binary.

use std::{env, path::PathBuf};

use bevy::{
    camera::NormalizedRenderTarget,
    input::{mouse::MouseScrollUnit, touch::TouchPhase},
    picking::{
        backend::HitData,
        events::{Pointer, Scroll},
        pointer::{Location, PointerId},
    },
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
    ui::{GlobalZIndex, Node, PositionType, Val},
    window::{PrimaryWindow, WindowRef},
};
use gdtf_ui::{
    ButtonLabel, ScrollListColors, UiPlugin, spawn_button, spawn_panel, spawn_scroll_list,
    theme::default_theme,
};

/// Resource holding the optional self-screenshot output path (from `GDTF_SCROLL_SHOT`).
#[derive(Resource)]
struct ShotPath(Option<PathBuf>);

/// Counts elapsed update frames so the demo scrolls after a settle delay and captures a
/// frame after the scrolled state has laid out.
#[derive(Resource, Default)]
struct FrameCounter(u32);

/// Tracks whether the screenshot was already requested this run.
#[derive(Resource, Default)]
struct ShotRequested(bool);

/// The scroll-area entity, stashed so the wheel-drive system can target it.
#[derive(Resource)]
struct DemoArea(Entity);

/// How many rows the demo stacks — well past what fits, so overflow + clipping are obvious.
const ROW_COUNT: usize = 15;

/// The first frame the demo starts wheel-scrolling (after a short layout settle).
const SCROLL_START_FRAME: u32 = 8;

/// The last frame the demo wheel-scrolls — it scrolls a bit each frame in this window so the
/// list ends up PARTWAY down (a mid band visible), not pinned to top or bottom.
const SCROLL_END_FRAME: u32 = 16;

/// The frame the demo requests the screenshot (a few frames after the last scroll, so the
/// scrolled state is laid out + drawn; the GPU readback is ASYNC, so exit waits for the file).
const SHOT_FRAME: u32 = 22;

/// How many Update frames to wait for the PNG before giving up. At 60 fps this is 10 seconds.
const POLL_CAP: u32 = 600;

fn main() {
    let shot = env::var("GDTF_SCROLL_SHOT").ok().map(PathBuf::from);
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(UiPlugin)
        .insert_resource(ShotPath(shot))
        .init_resource::<FrameCounter>()
        .init_resource::<ShotRequested>()
        .add_systems(Startup, setup)
        .add_systems(Update, (auto_scroll, maybe_capture, poll_for_png));
    app.run();
}

/// Spawns the camera, the theme, a panel, and a scroll list (with 15 distinct rows) inside a
/// short container over the panel.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let theme = default_theme();
    commands.insert_resource(theme.clone());

    // A panel anchored top-left, so the scroll list visibly sits over it.
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(6.0),
            top: Val::Vh(8.0),
            width: Val::Vw(34.0),
            height: Val::Vh(70.0),
            ..default()
        },
        GlobalZIndex(20),
    ));

    // A SHORT container over the panel that bounds the scroll list, so its rows overflow.
    let container = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Vw(9.0),
                top: Val::Vh(12.0),
                width: Val::Vw(26.0),
                height: Val::Vh(28.0),
                ..default()
            },
            GlobalZIndex(21),
        ))
        .id();

    let colors = ScrollListColors {
        area:  Color::srgb(0.10, 0.10, 0.14),
        track: Color::srgb(0.20, 0.20, 0.26),
        thumb: Color::srgb(0.55, 0.55, 0.64),
    };
    let area = spawn_scroll_list(&mut commands, colors, ());
    // `spawn_scroll_list` parents the area under its own root frame; move that root under the
    // short container so the list fills it.
    reparent_and_fill(&mut commands, container, area);

    // 15 visibly-distinct rows (a labeled button each), parented onto the scroll area.
    for i in 0..ROW_COUNT {
        let row = spawn_button(
            &mut commands,
            &theme,
            ButtonLabel::new(format!("Row {i:02}")),
            (),
        );
        commands.entity(row).insert(Node {
            width: Val::Percent(100.0),
            min_height: Val::Vh(6.0),
            margin: bevy::ui::UiRect::bottom(Val::Vh(0.6)),
            ..default()
        });
        commands.entity(area).add_child(row);
    }

    commands.insert_resource(DemoArea(area));
}

/// Moves the scroll list's root frame under the short container. The root is `area`'s parent
/// (set by `spawn_scroll_list` this same frame); a deferred command reads it after the spawn
/// flushes by simply re-parenting the area's root once available.
fn reparent_and_fill(commands: &mut Commands, container: Entity, area: Entity) {
    // `spawn_scroll_list` already parented `area` under its root frame within the same command
    // buffer; re-parenting the ROOT under the container is done in a startup-deferred closure
    // via `queue` so it runs after the structural commands apply.
    commands.queue(move |world: &mut World| {
        if let Some(root) = world.get::<ChildOf>(area).map(ChildOf::parent)
            && let Ok(mut container_entity) = world.get_entity_mut(container)
        {
            container_entity.add_child(root);
        }
    });
}

/// Wheel-scrolls the demo list partway between [`SCROLL_START_FRAME`] and
/// [`SCROLL_END_FRAME`] by triggering a real [`Pointer<Scroll>`] on the scroll area each
/// frame — the same event the picking backend dispatches on a wheel turn, so the production
/// `ScrollAreaPlugin` observer scrolls it. Ends partway down (a mid band visible).
fn auto_scroll(
    mut frames: ResMut<FrameCounter>,
    area: Res<DemoArea>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    frames.0 += 1;
    if frames.0 < SCROLL_START_FRAME || frames.0 > SCROLL_END_FRAME {
        return;
    }
    let primary_window = primary.iter().next();
    let Some(window_ref) = WindowRef::Primary
        .normalize(primary_window)
        .or_else(|| WindowRef::Entity(Entity::PLACEHOLDER).normalize(Some(Entity::PLACEHOLDER)))
    else {
        return;
    };
    let location = Location {
        target:   NormalizedRenderTarget::Window(window_ref),
        position: Vec2::ZERO,
    };
    let scroll = Scroll {
        unit:  MouseScrollUnit::Line,
        x:     0.0,
        // Negative-Y scrolls the content DOWN by one line each frame.
        y:     -1.0,
        hit:   HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        phase: TouchPhase::Moved,
    };
    commands.trigger(Pointer::new(PointerId::Mouse, location, scroll, area.0));
}

/// At [`SHOT_FRAME`] (if `GDTF_SCROLL_SHOT` was set), requests a screenshot via the observer
/// API; `save_to_disk` fires when the GPU readback completes. The demo does NOT exit here;
/// [`poll_for_png`] polls until the file appears.
fn maybe_capture(
    frames: Res<FrameCounter>,
    shot: Res<ShotPath>,
    mut requested: ResMut<ShotRequested>,
    mut commands: Commands,
) {
    if requested.0 {
        return;
    }
    let Some(path) = shot.0.clone() else {
        return;
    };
    if frames.0 != SHOT_FRAME {
        return;
    }
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    requested.0 = true;
}

/// Once a screenshot has been requested, polls every frame until the PNG appears on disk,
/// then exits cleanly. A safety cap of [`POLL_CAP`] frames prevents the demo from hanging.
fn poll_for_png(
    shot: Res<ShotPath>,
    requested: Res<ShotRequested>,
    frames: Res<FrameCounter>,
    mut exit: MessageWriter<AppExit>,
    mut poll_frames: Local<u32>,
) {
    let Some(path) = shot.0.as_ref() else {
        return;
    };
    if !requested.0 {
        return;
    }
    *poll_frames += 1;
    if std::path::Path::new(path).exists() {
        info!(
            "scroll_list_demo: PNG written to {}, exiting.",
            path.display()
        );
        exit.write(AppExit::Success);
        return;
    }
    if *poll_frames >= POLL_CAP {
        warn!(
            "scroll_list_demo: PNG not found after {} poll frames (total app frames: {}); \
             giving up. Was GDTF_SCROLL_SHOT set to a writable path?",
            POLL_CAP, frames.0,
        );
        exit.write(AppExit::Success);
    }
}
