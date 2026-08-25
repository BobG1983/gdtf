use bevy::app::App;
use gdtf_battle_presenter::{IsolateView, ViewMode};
use gdtf_battle_sim::{level::GridSize, metric::Level};
use gdtf_content_editor::{CanvasZoom, CurrentEditLevel, MapEditorSession, PreviewPan};

use crate::{
    harness::editing_app_and_client,
    load_case::reply_answered_during_load,
    names::EDITOR_SESSION,
    outcome::{ran_body, unavailable_code},
    rows::{IsolateRow, SessionReplyRow},
    socket::run_editor,
    support::{TestError, TestResult},
};

// A storey and a zoom the editor does not start on, both inside their own bounds.
const MOVED_TO: Level = Level::new(3);
const ZOOM_FACTOR: f32 = 2.0;

fn session_of(app: &App) -> Result<MapEditorSession, TestError> {
    let Some(session) = app.world().get_resource::<MapEditorSession>() else {
        return Err("the session is a resource the editor creates on entering Editing".into());
    };
    Ok(session.clone())
}

fn read_session(
    app: &mut App,
    client: &mut crate::socket::Client,
) -> Result<SessionReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SESSION, "()"))?;
    ran_body(&reply, EDITOR_SESSION)
}

#[test]
fn the_reply_names_every_resource_the_authoring_session_is_made_of() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let body = read_session(&mut app, &mut client)?;
    let session = session_of(&app)?;

    assert_eq!(
        body.theme,
        (*session.theme()).to_string(),
        "the theme key is the session's own, rendered as the text a load takes",
    );
    assert_eq!(
        body.default_floor,
        session.default_floor().map(|floor| (*floor).to_string()),
        "the default floor comes across as the session holds it, absent included",
    );
    assert_eq!(
        body.selected_tile,
        session.selected_tile().map(|tile| (*tile).to_string()),
        "the selected paint tile comes across as the session holds it, absent included",
    );
    let size = session.grid_size();
    assert_eq!(
        (
            body.grid_size.width,
            body.grid_size.height,
            body.grid_size.levels
        ),
        (*size.width(), *size.height(), *size.levels()),
        "all three grid spans come across on their own axes",
    );

    let Some(view) = app.world().get_resource::<ViewMode>() else {
        return Err("the view mode is scoped to Editing, so the world holds one".into());
    };
    let Some(isolate) = app.world().get_resource::<IsolateView>() else {
        return Err("the isolation is scoped to Editing, so the world holds one".into());
    };
    let Some(pan) = app.world().get_resource::<PreviewPan>() else {
        return Err("the preview pan is scoped to Editing, so the world holds one".into());
    };
    assert_eq!(
        format!("{:?}", body.view.mode),
        format!("{view:?}"),
        "the reply names the presenter's own view mode",
    );
    assert_eq!(
        body.view.isolate,
        match *isolate {
            IsolateView::Off => IsolateRow::Off,
            IsolateView::On(depth) => IsolateRow::On(*depth),
        },
        "the reply names the world's own isolation, onion depth included",
    );
    assert!(
        (body.view.pan.x - pan.offset().x).abs() < f32::EPSILON
            && (body.view.pan.y - pan.offset().y).abs() < f32::EPSILON,
        "the reply carries the preview camera's own offset: {:?} against {:?}",
        body.view.pan,
        pan.offset(),
    );
    Ok(())
}

#[test]
fn the_reply_follows_the_level_and_zoom_after_they_move_off_their_defaults() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let before = read_session(&mut app, &mut client)?;

    let moved = CurrentEditLevel::jumped(MOVED_TO, GridSize::default());
    let zoomed = CanvasZoom::identity().scaled(ZOOM_FACTOR);
    assert_ne!(
        *moved.level(),
        before.level,
        "the case must move the storey off the one the reply already reported, or it would \
         pass against a handler that answers a fixed level",
    );
    assert!(
        (*zoomed - before.view.zoom).abs() > f32::EPSILON,
        "the case must move the zoom off the one the reply already reported, or it would pass \
         against a handler that answers a fixed scale: {zoomed:?} against {}",
        before.view.zoom,
    );
    app.world_mut().insert_resource(moved);
    app.world_mut().insert_resource(zoomed);

    let after = read_session(&mut app, &mut client)?;
    assert_eq!(
        after.level,
        *moved.level(),
        "the reply reads the world's own edit level on the frame that answered",
    );
    assert!(
        (after.view.zoom - *zoomed).abs() < f32::EPSILON,
        "the reply reads the world's own canvas zoom on the frame that answered: {} against {}",
        after.view.zoom,
        *zoomed,
    );
    assert_eq!(
        after.view.mode, before.view.mode,
        "moving the level and the zoom leaves the draw mode where it was",
    );
    Ok(())
}

#[test]
fn session_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply =
        reply_answered_during_load(run_editor(EDITOR_SESSION, "()"), "the editor.session run")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "every resource this read is made of is scoped to Editing, so a Load-pass call is \
         refused rather than answering defaults the editor is not on",
    );
    Ok(())
}
