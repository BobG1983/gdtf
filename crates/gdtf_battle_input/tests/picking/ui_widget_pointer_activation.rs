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
use gdtf_test_utils::UiTestAppBuilder;

use super::harness::TARGET_SIZE;

const BUTTON_SIZE: Vec2 = Vec2::new(200.0, 60.0);

const BUTTON_ORIGIN: Vec2 = Vec2::new(100.0, 100.0);

#[derive(Resource, Default, Debug)]
struct Activations {
    count: usize,
}

fn record_activation(_activate: On<Activate>, mut activations: ResMut<Activations>) {
    activations.count += 1;
}

fn widget_app() -> (App, Entity) {
    let mut app = UiTestAppBuilder::new().with_ui_camera().build();
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

fn button_centre(app: &App, button: Entity) -> Option<Vec2> {
    let node = app.world().get::<ComputedNode>(button)?;
    if node.size.x <= 0.0 || node.size.y <= 0.0 {
        return None;
    }
    Some(app.world().get::<UiGlobalTransform>(button)?.translation)
}

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

#[test]
fn a_first_party_button_activates_from_a_real_pointer_click() {
    let (mut app, button) = widget_app();

    let Some(centre) = button_centre(&app, button) else {
        unreachable!("bevy_ui must lay the probe button out to a non-degenerate rect");
    };

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
