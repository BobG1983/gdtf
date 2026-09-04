//! Moving the world camera by cell, the cell the reply names it ended on, and the refusal
//! a call gets when there is no camera on the screen to move.

use std::sync::mpsc;

use bevy::{
    app::App,
    ecs::{entity::Entity, query::With},
    math::Vec2,
    transform::components::Transform,
};
use cobalt_mcp_protocol::{
    command::{CommandOutcome, UnavailableCode},
    message::QaResponse,
};
use cobalt_mcp_transport::IncomingRequest;
use gdtf_battle_input::world_to_cell;
use gdtf_battle_presenter::{ActiveLevel, WorldCamera, cell_to_world};
use gdtf_game::qa_wire::cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet};
use serde::Deserialize;

use super::{
    battle_fixture::{decoded, drive_into_battle_running, menu_app_with_mcp, run_one_frame},
    command_exchange::{VIEW_LOOK_AT, VIEW_PAN},
};

/// A cell near the middle of the 60x60 battlefield, well clear of the bounds clamp.
const MIDDLE: CellLevelNet = CellLevelNet::new(
    CellNet::new(CellXNet::new(30), CellYNet::new(30)),
    LevelNet::new(0),
);

/// Two cells along x and nothing on y.
const ALONG_X: CellNet = CellNet::new(CellXNet::new(2), CellYNet::new(0));

/// Two cells further along x, which is where a pan by [`ALONG_X`] must land.
const TWO_ALONG_X: CellLevelNet = CellLevelNet::new(
    CellNet::new(CellXNet::new(32), CellYNet::new(30)),
    LevelNet::new(0),
);

/// A diagonal offset. Both terms are non-zero, so dropping or flipping either one lands the
/// camera somewhere the case can tell apart.
const PAN_BY: CellNet = CellNet::new(CellXNet::new(2), CellYNet::new(1));

/// Two cells further along x and one further down y, which is where a pan by [`PAN_BY`] lands.
const PANNED_TO: CellLevelNet = CellLevelNet::new(
    CellNet::new(CellXNet::new(32), CellYNet::new(31)),
    LevelNet::new(0),
);

/// What both camera commands answer with.
#[derive(Debug, Deserialize)]
struct CameraBody {
    at: Option<CellLevelNet>,
}

fn camera_centre(app: &mut App) -> Vec2 {
    let mut cameras = app
        .world_mut()
        .query_filtered::<&Transform, With<WorldCamera>>();
    let Some(transform) = cameras.iter(app.world()).next() else {
        unreachable!("the battle screen spawns exactly one world camera");
    };
    Vec2::new(transform.translation.x, transform.translation.y)
}

/// The cell the world's own camera transform projects onto, read after the frame has run.
fn camera_cell(app: &mut App) -> Option<CellLevelNet> {
    let Some(active) = app.world().get_resource::<ActiveLevel>().copied() else {
        unreachable!("the presenter inits ActiveLevel when its plugin is built");
    };
    world_to_cell(camera_centre(app), *active).map(CellLevelNet::from_sim)
}

fn centre_of(at: CellLevelNet) -> Vec2 {
    let (cell, level) = at.to_sim().split();
    let world = cell_to_world(cell, level);
    Vec2::new(world.x, world.y)
}

fn target_argument(at: CellLevelNet) -> String {
    let Ok(text) = ron::ser::to_string(&at) else {
        unreachable!("a wire cell-level serializes to compact RON");
    };
    format!("(at:{text})")
}

fn offset_argument(by: CellNet) -> String {
    let Ok(text) = ron::ser::to_string(&by) else {
        unreachable!("a wire cell serializes to compact RON");
    };
    format!("(by:{text})")
}

#[test]
fn look_at_centres_the_camera_on_the_cell_it_was_given() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);

    let body: CameraBody = decoded(
        VIEW_LOOK_AT,
        run_one_frame(&mut app, &tx, VIEW_LOOK_AT, &target_argument(MIDDLE)),
    );

    assert_eq!(
        camera_centre(&mut app),
        centre_of(MIDDLE),
        "look_at focuses the same projection the framing system uses, so the camera has to sit \
         exactly on that cell's world centre",
    );
    assert_eq!(
        body.at,
        Some(MIDDLE),
        "the reply names the cell the camera ended on: {body:?}",
    );
    assert_eq!(
        body.at,
        camera_cell(&mut app),
        "the reply is answered after the frame's bounds clamp, so it has to agree with what the \
         world's own camera transform projects onto — answered before the clamp and a clamped \
         move would be reported as the target it never reached",
    );
}

/// Centre on the middle cell, pan by `by`, and require the camera to land on `landing_on`.
fn pan_from_the_middle(by: CellNet, landing_on: CellLevelNet) {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    let _centred: CameraBody = decoded(
        VIEW_LOOK_AT,
        run_one_frame(&mut app, &tx, VIEW_LOOK_AT, &target_argument(MIDDLE)),
    );
    let before = camera_centre(&mut app);

    let body: CameraBody = decoded(
        VIEW_PAN,
        run_one_frame(&mut app, &tx, VIEW_PAN, &offset_argument(by)),
    );

    let after = camera_centre(&mut app);
    assert_eq!(
        after,
        centre_of(landing_on),
        "the offset asked for is the offset taken on both axes, and cells run down the screen \
         while world y runs up it, so a dropped or flipped term shows up here — {before} -> \
         {after}",
    );
    assert_eq!(
        body.at,
        Some(landing_on),
        "the reply names the cell the pan landed on: {body:?}",
    );
    assert_eq!(
        body.at,
        camera_cell(&mut app),
        "the reply is answered after the frame's bounds clamp, so it has to agree with what the \
         world's own camera transform projects onto",
    );
}

#[test]
fn a_pan_along_x_moves_the_camera_the_cells_it_was_given() {
    pan_from_the_middle(ALONG_X, TWO_ALONG_X);
}

#[test]
fn a_pan_moves_the_camera_on_both_axes_at_once() {
    pan_from_the_middle(PAN_BY, PANNED_TO);
}

/// A live battle with the world camera taken away, which is the world the battle screen leaves
/// behind on the frame it tears down.
fn battle_without_a_world_camera() -> (App, mpsc::Sender<IncomingRequest>) {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    let mut cameras = app
        .world_mut()
        .query_filtered::<Entity, With<WorldCamera>>();
    let spawned: Vec<Entity> = cameras.iter(app.world()).collect();
    assert!(
        !spawned.is_empty(),
        "the battle screen spawns a world camera, so this case has one to take away",
    );
    for camera in spawned {
        app.world_mut().entity_mut(camera).despawn();
    }
    (app, tx)
}

/// The refusal a camera command owes a claimed call it cannot serve.
fn refused_for_want_of_a_camera(name: &str, answer: QaResponse) {
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = answer else {
        unreachable!("`{name}` must refuse when there is no camera to move, got {answer:?}");
    };
    assert_eq!(
        code,
        UnavailableCode::MissingModel,
        "the battle is live and the only missing piece is the camera entity — {note:?}",
    );
    assert!(
        note.as_str().contains("no camera is on the screen"),
        "the note has to name the camera as the missing piece, not the view resources, or a \
         client is sent looking for the wrong thing: `{name}` — {note:?}",
    );
}

#[test]
fn look_at_answers_the_call_itself_when_no_camera_is_on_the_screen() {
    let (mut app, tx) = battle_without_a_world_camera();

    // `run_one_frame` is the pin: it demands the answer inside the frame the call was claimed,
    // so parking the responder for the pending-queue sweep to time out fails here.
    let answer = run_one_frame(&mut app, &tx, VIEW_LOOK_AT, &target_argument(MIDDLE));

    refused_for_want_of_a_camera(VIEW_LOOK_AT, answer);
}

#[test]
fn a_pan_answers_the_call_itself_when_no_camera_is_on_the_screen() {
    let (mut app, tx) = battle_without_a_world_camera();

    let answer = run_one_frame(&mut app, &tx, VIEW_PAN, &offset_argument(PAN_BY));

    refused_for_want_of_a_camera(VIEW_PAN, answer);
}
