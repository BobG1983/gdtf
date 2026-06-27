//! GTW-416 in-engine QA demo for the [`Accordion`](gdtf_ui::Accordion) widget (AC4).
//!
//! A tiny `DefaultPlugins` app that spawns a themed PANEL and, inside a container over it, an
//! [`Accordion`](gdtf_ui::Accordion) with several rows. After a short layout settle it
//! TOGGLES the first row open by setting its content [`AccordionAnim`] to the OPENING state
//! ([`AccordionAnim::toggled`] from `Collapsed` → `Expanding`) — the SAME state a real header
//! press produces — so the production [`drive_accordions`](gdtf_ui::drive_accordions) system
//! starts LERPING that row's content height open. It then captures a frame a FEW frames into the
//! lerp, while the row is PARTWAY expanded (NOT settled), so the PNG genuinely shows the
//! accordion MID-ANIMATION.
//!
//! It drives the animation state DIRECTLY rather than faking a header click: under windowed
//! `DefaultPlugins` with no real cursor, `bevy_ui`'s built-in `ui_focus_system` (`PreUpdate`)
//! REWRITES every `Interaction` from raw mouse state each frame, so a demo-set
//! `Interaction::Pressed` is clobbered back to `None` before `drive_accordions` (`Update`) reads
//! it — the press edge cannot be faked without a real click (bevy-traps). Setting the content's
//! [`AccordionAnim`] to its toggled (opening) state reaches the exact same lerp the press would
//! have started. The press-DETECTION half is covered by the in-crate `MinimalPlugins` unit test;
//! this demo only showcases the lerp + colors.
//!
//! ## How QA runs it
//!
//! Toggle the first row open and capture a mid-lerp PNG, then exit (no manual input):
//!
//! ```text
//! GDTF_ACCORDION_SHOT=/abs/out.png cargo run -p gdtf_ui --example accordion_demo
//! ```
//!
//! Drop the `GDTF_ACCORDION_SHOT` env var to just watch it live (the first row animates open).
//!
//! ## Which frame it captures
//!
//! The first row's content animation is opened on [`TOGGLE_FRAME`] (8). The lerp speed is `4.0`/s,
//! so a full open takes `~0.25 s` (`~15` frames at 60 fps); each `~16 ms` frame advances progress
//! by `~0.067`. The screenshot is requested on [`SHOT_FRAME`] (16) — eight frames (`~0.13 s`,
//! progress `~0.5`) into the open lerp, so the row is roughly HALF-open: a clearly readable
//! content strip (`~9 Vh`, not a 26 px sliver), unmistakably mid-animation and NOT yet settled.
//! The GPU readback is async, so the demo polls until the PNG exists before exiting. The capture
//! path is purely a dev affordance for this example — it is NOT compiled into any shipped binary.
//!
//! The colors are deliberately HIGH-CONTRAST so QA can read the structure: a bright amber header
//! bar, a teal content fill, and a near-black panel/scroll background. Each header carries a
//! legible label ("Member 1".."Member 3"), and the viewport is sized so the expanding first row
//! AND the sibling rows below it are all visible — proving the "siblings drop below" reposition.

use std::{env, path::PathBuf};

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
    ui::{GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{
    AccordionAnim, AccordionColors, AccordionContent, UiPlugin, spawn_accordion,
    spawn_accordion_row, spawn_panel, theme::default_theme,
};

/// Resource holding the optional self-screenshot output path (from `GDTF_ACCORDION_SHOT`).
#[derive(Resource)]
struct ShotPath(Option<PathBuf>);

/// Counts elapsed update frames so the demo toggles + captures at deterministic frames.
#[derive(Resource, Default)]
struct FrameCounter(u32);

/// Tracks whether the screenshot was already requested this run.
#[derive(Resource, Default)]
struct ShotRequested(bool);

/// Tracks whether the first row has been toggled open yet.
#[derive(Resource, Default)]
struct Toggled(bool);

/// Records the FIRST row's content [`Entity`] so the toggle drives a deterministic row's
/// animation directly (a resource over a bare [`Entity`] so the toggle system finds the
/// exact row regardless of query iteration order).
#[derive(Resource)]
struct FirstRowContent(Entity);

/// How many accordion rows the demo stacks. Three rows keeps the expanding first row AND both
/// siblings below it inside the capture viewport (so the "siblings drop below" reposition reads).
const ROW_COUNT: usize = 3;

/// The frame the demo presses the first row's header open (after a short layout settle).
const TOGGLE_FRAME: u32 = 8;

/// The frame the demo requests the screenshot — `~8` frames INTO the open lerp (progress `~0.5`,
/// roughly HALF-open), so the captured row is a clearly readable, partway-expanded content area:
/// unmistakably mid-animation, NOT a thin sliver and NOT a settled fully-open state.
const SHOT_FRAME: u32 = 16;

/// How many Update frames to wait for the PNG before giving up (10 s at 60 fps).
const POLL_CAP: u32 = 600;

fn main() {
    let shot = env::var("GDTF_ACCORDION_SHOT").ok().map(PathBuf::from);
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(UiPlugin)
        .insert_resource(ShotPath(shot))
        .init_resource::<FrameCounter>()
        .init_resource::<ShotRequested>()
        .init_resource::<Toggled>()
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_first_row, maybe_capture, poll_for_png));
    app.run();
}

/// Spawns the camera, the theme, a panel, and an accordion with several rows inside a
/// container over the panel.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let theme = default_theme();
    commands.insert_resource(theme.clone());

    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(6.0),
            top: Val::Vh(8.0),
            width: Val::Vw(40.0),
            height: Val::Vh(80.0),
            ..default()
        },
        GlobalZIndex(20),
    ));

    let container = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Vw(9.0),
                top: Val::Vh(12.0),
                width: Val::Vw(32.0),
                height: Val::Vh(70.0),
                ..default()
            },
            GlobalZIndex(21),
        ))
        .id();

    // High-contrast, clearly DISTINCT fills so QA can read the structure: a near-black panel /
    // scroll background, a bright AMBER header bar, and a TEAL content fill that is unmistakable
    // against both the headers and the background when a row expands.
    let colors = AccordionColors {
        area:    Color::srgb(0.04, 0.04, 0.05),
        track:   Color::srgb(0.12, 0.12, 0.14),
        thumb:   Color::srgb(0.45, 0.45, 0.50),
        header:  Color::srgb(0.90, 0.62, 0.10),
        content: Color::srgb(0.05, 0.62, 0.60),
    };
    let stack = spawn_accordion(&mut commands, colors, ());
    // `spawn_accordion` parents the stack under the scroll-list root frame; re-parent that
    // root under the container so the list fills it.
    reparent_and_fill(&mut commands, container, stack);

    let mut first_content = None;
    for i in 0..ROW_COUNT {
        // A legible header label ("Member 1"..) lives ON the row's header button (passed as the
        // header marker), so the captured PNG clearly identifies which row is expanding and which
        // are its siblings. Near-black text reads against the bright amber header bar.
        let header_label = (
            Text::new(format!("Member {}", i + 1)),
            TextColor(Color::srgb(0.05, 0.05, 0.06)),
        );
        let content = spawn_accordion_row(&mut commands, stack, colors, header_label, ());
        if first_content.is_none() {
            first_content = Some(content);
        }
        // A bright revealed-content block so the expanded teal area visibly shows something.
        let label = commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    min_height: Val::Vh(4.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.85, 0.90, 0.92)),
            ))
            .id();
        commands.entity(content).add_child(label);
    }
    if let Some(content) = first_content {
        commands.insert_resource(FirstRowContent(content));
    }
}

/// Re-parents the accordion's scroll-list root frame under the container so the list fills it.
fn reparent_and_fill(commands: &mut Commands, container: Entity, stack: Entity) {
    commands.queue(move |world: &mut World| {
        // `stack` is parented under the scroll AREA, whose parent is the scroll-list ROOT
        // frame; move that root under the container.
        if let Some(area) = world.get::<ChildOf>(stack).map(ChildOf::parent)
            && let Some(root) = world.get::<ChildOf>(area).map(ChildOf::parent)
            && let Ok(mut container_entity) = world.get_entity_mut(container)
        {
            container_entity.add_child(root);
        }
    });
}

/// On [`TOGGLE_FRAME`], opens the FIRST row by setting its content [`AccordionAnim`] to its
/// toggled (opening) state — the SAME state a real header press produces — so the production
/// `drive_accordions` system lerps that row open.
///
/// It drives the animation state DIRECTLY rather than faking `Interaction::Pressed`: under
/// windowed `DefaultPlugins` with no real cursor, `ui_focus_system` (`PreUpdate`) clobbers a
/// demo-set `Interaction` back to `None` before `drive_accordions` (`Update`) reads it, so the
/// press edge can't be faked here (bevy-traps). Fires once.
fn toggle_first_row(
    mut frames: ResMut<FrameCounter>,
    mut toggled: ResMut<Toggled>,
    first: Option<Res<FirstRowContent>>,
    mut contents: Query<&mut AccordionAnim, With<AccordionContent>>,
) {
    frames.0 += 1;
    if toggled.0 || frames.0 != TOGGLE_FRAME {
        return;
    }
    let Some(first) = first else {
        return;
    };
    if let Ok(mut anim) = contents.get_mut(first.0) {
        *anim = anim.toggled();
        toggled.0 = true;
    }
}

/// At [`SHOT_FRAME`] (if `GDTF_ACCORDION_SHOT` was set), requests a screenshot via the observer
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

/// Once a screenshot has been requested, polls every frame until the PNG appears, then exits.
/// A safety cap of [`POLL_CAP`] frames prevents the demo from hanging.
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
            "accordion_demo: PNG written to {}, exiting.",
            path.display()
        );
        exit.write(AppExit::Success);
        return;
    }
    if *poll_frames >= POLL_CAP {
        warn!(
            "accordion_demo: PNG not found after {} poll frames (total app frames: {}); giving \
             up. Was GDTF_ACCORDION_SHOT set to a writable path?",
            POLL_CAP, frames.0,
        );
        exit.write(AppExit::Success);
    }
}
