use gdtf_battle_sim::{level::MAX_GRID_SPAN, metric::MAX_LEVELS};

use crate::{
    canvas::edit_level,
    setup::{prefab_app_and_client, set_grid_size, set_level},
    support::TestResult,
    world::session,
};

// Above every band the size fields allow, on all three axes at once.
const OVER_EVERY_MAXIMUM: u8 = u8::MAX;

// A grid with room for more than one storey, so a re-clamp has somewhere to land.
const SHORT_SPAN: u8 = 12;
const SHORT_LEVELS: u8 = 2;

#[test]
fn spans_above_the_maxima_clamp_into_the_bands_the_size_fields_allow() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let body = set_grid_size(
        &mut app,
        &mut client,
        OVER_EVERY_MAXIMUM,
        OVER_EVERY_MAXIMUM,
        OVER_EVERY_MAXIMUM,
    )?;
    app.update();

    assert_eq!(
        (body.grid_size.width, body.grid_size.height),
        (MAX_GRID_SPAN, MAX_GRID_SPAN),
        "a span past the field's own band clamps into it rather than failing, and the reply \
         echoes what was written so a caller can see the clamp: {body:?}",
    );
    assert_eq!(
        body.grid_size.levels, MAX_LEVELS,
        "the storey count clamps to its own maximum, which is a different band from the \
         width and height: {body:?}",
    );
    let held = session(&app)?.grid_size();
    assert_eq!(
        (*held.width(), *held.height(), *held.levels()),
        (
            body.grid_size.width,
            body.grid_size.height,
            body.grid_size.levels
        ),
        "the session in the world carries the grid the reply reported, so a reply that clamped \
         its own numbers without writing them is caught",
    );
    Ok(())
}

#[test]
fn a_shrink_below_the_edit_storey_re_clamps_it_in_the_same_call() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let top = session(&app)?.grid_size().levels().saturating_sub(1);
    let raised = set_level(&mut app, &mut client, top)?;
    assert_eq!(
        raised.level, top,
        "the case must climb above the storey the shrink allows, or nothing here could be \
         re-clamped: {raised:?}",
    );

    let body = set_grid_size(&mut app, &mut client, SHORT_SPAN, SHORT_SPAN, SHORT_LEVELS)?;
    app.update();

    assert_eq!(
        body.grid_size.levels, SHORT_LEVELS,
        "the storey count is inside its band, so it is written as asked: {body:?}",
    );
    assert_eq!(
        body.level,
        body.grid_size.levels.saturating_sub(1),
        "the commit re-clamps the edit storey to the shrunk grid's top, and the reply echoes \
         it so a caller knows which storey it is now painting on: {body:?}",
    );
    assert_eq!(
        *edit_level(&app)?.level(),
        body.level,
        "the edit storey in the world matches the echo, so a reply that clamped its own number \
         without writing the resource is caught",
    );
    Ok(())
}
