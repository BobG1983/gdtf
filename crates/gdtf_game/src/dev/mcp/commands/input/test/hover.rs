use std::sync::mpsc::Receiver;

use bevy::{
    app::App,
    ecs::message::Messages,
    math::Vec2,
    prelude::*,
    window::{CursorMoved, PrimaryWindow, Window},
};
use cobalt_mcp_command::dispatch::{CommandCall, register_command};
use cobalt_mcp_protocol::message::McpResponse;
use cobalt_mcp_transport::{PendingQueue, Responder};
use gdtf_test_utils::WindowedTestAppBuilder;

use super::super::hover::{InputHover, InputHoverArgs};

/// One pixel to hover, written in `input.hover`'s own argument shape.
const AT: &str = "(at:(x:120,y:48))";

/// The pixel `AT` names, as the window stores a logical cursor position.
const AT_PIXEL: Vec2 = Vec2::new(120.0, 48.0);

/// A headless app with a real primary window, carrying `input.hover` as the game registers it.
fn hover_app() -> App {
    let mut app = WindowedTestAppBuilder::new().build();
    register_command::<InputHover>(&mut app);
    app
}

/// Queue one `input.hover` call and run the frame that answers it.
fn hover_once(app: &mut App) -> Receiver<McpResponse> {
    let Ok(args) = ron::de::from_str::<InputHoverArgs>(AT) else {
        unreachable!("`input.hover`'s own argument shape must decode {AT}");
    };
    let (responder, reply_rx) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<CommandCall<InputHover>>>()
        .push_new(CommandCall::<InputHover>::new(args), responder);
    app.update();
    reply_rx
}

/// Where the primary window says its cursor is.
fn cursor(app: &mut App) -> Option<Vec2> {
    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>();
    windows
        .iter(app.world())
        .next()
        .and_then(Window::cursor_position)
}

/// Every cursor move written this frame.
fn moves(app: &App) -> Vec<CursorMoved> {
    app.world()
        .get_resource::<Messages<CursorMoved>>()
        .map(|messages| messages.iter_current_update_messages().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn a_hover_writes_the_pixel_into_the_primary_window() {
    let mut app = hover_app();

    let _reply_rx = hover_once(&mut app);

    assert_eq!(
        cursor(&mut app),
        Some(AT_PIXEL),
        "the hover must land in the window's own cursor, which is where `pick_hovered_cell` \
         reads the mouse from — nothing else in the game carries a mouse position",
    );
}

#[test]
fn a_hover_writes_exactly_one_cursor_move() {
    let mut app = hover_app();

    let _reply_rx = hover_once(&mut app);

    let written = moves(&app);
    assert_eq!(
        written.len(),
        1,
        "one call moves the pointer once: the message is what hands pointer ownership back \
         from the gamepad, and a duplicate would be a second move the client never asked for — \
         got {written:?}",
    );
    let Some(first) = written.first() else {
        unreachable!("the length assertion above already proved one message is there");
    };
    assert_eq!(
        first.position, AT_PIXEL,
        "the cursor move must carry the pixel that was written, not the one it moved from: \
         {first:?}",
    );
    assert_eq!(
        first.window,
        cursor_window(&mut app),
        "the move must name the primary window the pixel was written into, or a reader that \
         checks the window ignores it: {first:?}",
    );
}

/// The primary window entity, which a cursor move must name.
fn cursor_window(app: &mut App) -> Entity {
    let mut windows = app
        .world_mut()
        .query_filtered::<Entity, (With<Window>, With<PrimaryWindow>)>();
    let Some(entity) = windows.iter(app.world()).next() else {
        unreachable!("the windowed harness builds a primary window");
    };
    entity
}
