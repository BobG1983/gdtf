//! GTW-811 SPIKE PROBE — does a first-party `bevy_ui_widgets::Button` activate from a REAL
//! pointer click, with no project-side bridge?
//!
//! This is the second half of the spike's evidence. GTW-637 shipped
//! `bridge_continue_activation` on the Options screen on the stated premise that "the
//! project does not enable Bevy's `ui_picking` backend", so the widgets' own pointer
//! observers could never fire
//! (`crates/gdtf_app/src/states/running/options/systems/actions.rs`). That premise is
//! wrong: the `ui` feature pulls `picking` → `ui_picking`, and `UiPlugin` installs the
//! backend. This probe drives real `bevy_picking` press/release input at a real
//! `bevy_ui_widgets::Button` and observes whether the widget's native
//! `button_on_pointer_click` path produces an `Activate` with zero bridging code.
//!
//! Every world mutation is in a TEST BODY — the accepted headless idiom
//! (`bevy-traps.md` #7 carve-out (a)).

use bevy::{
    app::App,
    camera::{RenderTarget, visibility::Visibility},
    math::Vec2,
    picking::pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform},
    ui_widgets::{Activate, Button as WidgetButton},
    window::{PrimaryWindow, Window, WindowRef, WindowResolution},
};
use gdtf_test_utils::GdtfUiTestAppBuilder;

use super::harness::TARGET_SIZE;

/// The probe button's size in logical px.
const BUTTON_SIZE: Vec2 = Vec2::new(200.0, 60.0);

/// The probe button's top-left offset from the window origin, in logical px.
const BUTTON_ORIGIN: Vec2 = Vec2::new(100.0, 100.0);

/// Counts the `Activate` triggers the widget produced — the probe's observable.
#[derive(Resource, Default, Debug)]
struct Activations {
    /// How many `Activate` events named the probe button.
    count: usize,
}

/// Records every `Activate` trigger, so the test can assert the NATIVE widget path fired.
fn record_activation(_activate: On<Activate>, mut activations: ResMut<Activations>) {
    activations.count += 1;
}

/// Builds a `DefaultPlugins` UI app carrying one first-party widget button and an observer
/// counting its activations. No project input code is added — whatever fires here is
/// Bevy's own wiring.
fn widget_app() -> (App, Entity) {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.init_resource::<Activations>();
    app.add_observer(record_activation);

    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(TARGET_SIZE.x as u32, TARGET_SIZE.y as u32),
            ..default()
        },
        PrimaryWindow,
    ));

    let button = app
        .world_mut()
        .spawn((
            WidgetButton,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BUTTON_ORIGIN.x),
                top: Val::Px(BUTTON_ORIGIN.y),
                width: Val::Px(BUTTON_SIZE.x),
                height: Val::Px(BUTTON_SIZE.y),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            Visibility::Visible,
        ))
        .id();

    for _ in 0..4 {
        app.update();
    }
    (app, button)
}

/// The button's laid-out centre, read back from `bevy_ui` rather than predicted.
fn button_centre(app: &App, button: Entity) -> Option<Vec2> {
    let node = app.world().get::<ComputedNode>(button)?;
    if node.size.x <= 0.0 || node.size.y <= 0.0 {
        return None;
    }
    Some(app.world().get::<UiGlobalTransform>(button)?.translation)
}

/// Writes one `PointerInput` message for the mouse pointer at `position`.
fn send_pointer(app: &mut App, position: Vec2, action: PointerAction) {
    let Some(window) = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .iter(app.world())
        .next()
    else {
        return;
    };
    let Some(target) = RenderTarget::Window(WindowRef::Primary).normalize(Some(window)) else {
        return;
    };
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location { target, position },
        action,
    ));
}

/// GTW-811 finding 3 — a first-party `bevy_ui_widgets::Button` DOES activate from a real
/// pointer press/release with NO project-side bridge, because the `ui_picking` backend is
/// already installed.
///
/// This is the fact that retires the GTW-637 premise. Pin-discriminating: the assertion is
/// a nonzero `Activate` count produced solely by Bevy's `ButtonPlugin` observers — the test
/// adds no bridge, no `Interaction` write, and no `FocusActivated`.
#[test]
fn a_first_party_button_activates_from_a_real_pointer_click() {
    let (mut app, button) = widget_app();

    let Some(centre) = button_centre(&app, button) else {
        unreachable!("bevy_ui must lay the probe button out to a non-degenerate rect");
    };

    // Move onto the button, then press and release over it — the sequence a real mouse
    // produces. Each message is consumed by `PointerInput::receive` in `PreUpdate`.
    send_pointer(&mut app, centre, PointerAction::Move { delta: Vec2::ZERO });
    app.update();
    send_pointer(
        &mut app,
        centre,
        PointerAction::Press(PointerButton::Primary),
    );
    app.update();
    send_pointer(
        &mut app,
        centre,
        PointerAction::Release(PointerButton::Primary),
    );
    app.update();
    app.update();

    assert!(
        app.world().resource::<Activations>().count > 0,
        "a real pointer click over a bevy_ui_widgets::Button must produce an Activate with \
         NO project bridge — the ui_picking backend is installed by UiPlugin",
    );
}
