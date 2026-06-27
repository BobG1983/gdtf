//! GTW-410 in-engine QA demo for the [`Dropdown`](gdtf_ui::Dropdown) widget (AC4).
//!
//! A tiny `DefaultPlugins` app that spawns a themed PANEL and, OVER it, a
//! [`Dropdown`](gdtf_ui::Dropdown) closed control. After a few settle frames it OPENS the
//! dropdown by driving the closed control's [`Interaction`](bevy::ui::Interaction) to
//! `Pressed` — so QA sees the floating option list rendered ABOVE the panel (the
//! [`GlobalZIndex`](bevy::ui::GlobalZIndex) anti-occlusion path, bevy-traps #8).
//!
//! ## How QA runs it
//!
//! Open the dropdown over the panel and capture a PNG, then exit (no manual clicking):
//!
//! ```text
//! GDTF_DROPDOWN_SHOT=/abs/out.png cargo run -p gdtf_ui --example dropdown_demo
//! ```
//!
//! Drop the `GDTF_DROPDOWN_SHOT` env var to just watch it live (it stays open). `Read` the
//! PNG to confirm: the closed control shows the current selection, the floating option list
//! is drawn ON TOP of the panel (not occluded), and the rows are legible. The capture path
//! is purely a dev affordance for this example — it is NOT compiled into any shipped binary.

use std::{env, path::PathBuf};

use bevy::{
    prelude::*,
    render::view::window::screenshot::{Screenshot, save_to_disk},
    ui::{GlobalZIndex, Interaction, Node, PositionType, Val},
};
use gdtf_ui::{
    DropdownColors, DropdownOption, UiPlugin, register_dropdown, spawn_dropdown, spawn_panel,
    theme::default_theme,
};

/// The demo's option identity — a named newtype over the choice name (no-bare-types rule),
/// standing in for a real domain id (a weapon / armor / theme name) a caller would use.
#[derive(Clone, PartialEq, Eq, Debug)]
struct DemoChoice(String);

impl DemoChoice {
    /// Wraps a demo choice name.
    fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Marker on the demo dropdown so the auto-open system finds exactly it.
#[derive(Component)]
struct DemoDropdown;

/// Resource holding the optional self-screenshot output path (from `GDTF_DROPDOWN_SHOT`).
#[derive(Resource)]
struct ShotPath(Option<PathBuf>);

/// Counts elapsed update frames so the demo opens the dropdown after a settle delay and
/// captures a frame after it has had time to lay out.
#[derive(Resource, Default)]
struct FrameCounter(u32);

/// Tracks whether the screenshot was already requested this run (so `poll_for_png` knows
/// when to start checking the output file).
#[derive(Resource, Default)]
struct ShotRequested(bool);

/// The frame the demo auto-opens the dropdown (after a short layout settle).
const OPEN_FRAME: u32 = 6;

/// The frame the demo requests the screenshot (a few frames after open, so the popup is
/// laid out + drawn; the GPU readback is ASYNC, so exit happens only after the file lands).
const SHOT_FRAME: u32 = 16;

/// How many Update frames to wait for the PNG to appear on disk before giving up and
/// exiting with a warning. At 60 fps this is 10 seconds — generous enough for any GPU
/// readback + PNG encode, but finite so the demo never hangs.
const POLL_CAP: u32 = 600;

fn main() {
    let shot = env::var("GDTF_DROPDOWN_SHOT").ok().map(PathBuf::from);
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(UiPlugin)
        .insert_resource(ShotPath(shot))
        .init_resource::<FrameCounter>()
        .init_resource::<ShotRequested>()
        .add_systems(Startup, setup)
        .add_systems(Update, (auto_open, maybe_capture, poll_for_png));
    // Register the demo's concrete option-id type so its dropdown drivers run (an
    // unregistered generic system never runs — gate 4b). This MUST happen at build time
    // (`&mut App`), not inside a system.
    register_dropdown::<DemoChoice>(&mut app);
    app.run();
}

/// Spawns the camera, the theme, a panel, and the dropdown over the panel.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let theme = default_theme();
    commands.insert_resource(theme.clone());

    // A panel anchored top-left, sized so the floating list visibly overlaps it.
    let panel = spawn_panel(&mut commands, &theme);
    commands.entity(panel).insert((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Vw(8.0),
            top: Val::Vh(12.0),
            width: Val::Vw(30.0),
            height: Val::Vh(40.0),
            ..default()
        },
        GlobalZIndex(20),
    ));

    // The dropdown closed control, positioned near the panel's top edge so its OPEN list
    // drops down OVER the panel body — the occlusion case AC4 wants captured.
    let options = vec![
        DropdownOption::new(DemoChoice::new("autopistol"), "Autopistol"),
        DropdownOption::new(DemoChoice::new("lasgun"), "Lasgun"),
        DropdownOption::new(DemoChoice::new("bolter"), "Bolter"),
        DropdownOption::new(DemoChoice::new("shotgun"), "Shotgun"),
    ];
    let colors = DropdownColors {
        control_bg: Color::srgb(0.18, 0.18, 0.22),
        text:       Color::srgb(0.92, 0.92, 0.86),
        popup_bg:   Color::srgb(0.10, 0.10, 0.14),
        option_bg:  Color::srgb(0.16, 0.16, 0.20),
    };
    let dropdown = spawn_dropdown(&mut commands, options, 0, colors, DemoDropdown);
    commands.entity(dropdown).insert(Node {
        position_type: PositionType::Absolute,
        left: Val::Vw(10.0),
        top: Val::Vh(14.0),
        width: Val::Vw(20.0),
        ..default()
    });
}

/// Opens the demo dropdown at [`OPEN_FRAME`] by driving its closed control's
/// [`Interaction`](bevy::ui::Interaction) to `Pressed` (the same edge a click produces), so
/// the open drivers spawn the floating list — no real mouse needed.
fn auto_open(
    mut frames: ResMut<FrameCounter>,
    mut triggers: Query<&mut Interaction, With<DemoDropdown>>,
) {
    frames.0 += 1;
    if frames.0 == OPEN_FRAME {
        for mut interaction in &mut triggers {
            *interaction = Interaction::Pressed;
        }
    }
}

/// At [`SHOT_FRAME`] (if `GDTF_DROPDOWN_SHOT` was set), requests a screenshot via the
/// observer API — `save_to_disk` fires when the GPU readback completes (several frames
/// later). The demo does NOT exit here; [`poll_for_png`] polls until the file appears.
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
    // Spawn the screenshot entity with the save observer. `save_to_disk` writes the PNG
    // synchronously inside the `ScreenshotCaptured` observer, so the file will exist by
    // the time `poll_for_png` sees it via `Path::exists()`.
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    requested.0 = true;
}

/// Once a screenshot has been requested, polls every frame until the PNG file appears on
/// disk (written by the `save_to_disk` observer after GPU readback completes), then exits
/// cleanly. A safety cap of [`POLL_CAP`] frames prevents the demo from hanging if no
/// `GDTF_DROPDOWN_SHOT` is set or the save fails for any reason.
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
        info!("dropdown_demo: PNG written to {}, exiting.", path.display());
        exit.write(AppExit::Success);
        return;
    }
    if *poll_frames >= POLL_CAP {
        warn!(
            "dropdown_demo: PNG not found after {} poll frames (total app frames: {}); \
             giving up. Was GDTF_DROPDOWN_SHOT set to a writable path?",
            POLL_CAP, frames.0,
        );
        exit.write(AppExit::Success);
    }
}
