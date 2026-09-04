use crate::{
    canvas::edit_level,
    setup::{prefab_app_and_client, set_level},
    support::TestResult,
    world::session,
};

// Past every storey count a grid can carry, so the clamp is the only way this can answer.
const OVER_EVERY_EXTENT: u8 = u8::MAX;

#[test]
fn a_storey_past_the_extent_clamps_to_the_top_one_rather_than_being_refused() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let top = session(&app)?.grid_size().levels().saturating_sub(1);

    let body = set_level(&mut app, &mut client, OVER_EVERY_EXTENT)?;
    app.update();

    assert_eq!(
        body.level, top,
        "a storey past the grid's extent clamps to the nearest one the grid has, and the reply \
         echoes it. Answering Unavailable instead would leave a client with no storey at all: \
         {body:?}",
    );
    assert_eq!(
        *edit_level(&app)?.level(),
        body.level,
        "the edit storey in the world matches the echo, so a reply that clamped its own number \
         without writing the resource is caught",
    );
    Ok(())
}

#[test]
fn a_storey_inside_the_extent_is_written_as_it_was_asked_for() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let top = session(&app)?.grid_size().levels().saturating_sub(1);
    assert!(
        top > 0,
        "the default grid carries more than one storey, or a jump could not be told from a \
         clamp",
    );

    let body = set_level(&mut app, &mut client, top)?;
    app.update();

    assert_eq!(
        body.level, top,
        "the top storey is inside the extent, so it is reached rather than clamped away: \
         {body:?}",
    );
    assert_eq!(
        *edit_level(&app)?.level(),
        top,
        "the canvas paints on the storey the jump reached",
    );
    Ok(())
}
