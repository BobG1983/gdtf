use crate::{
    mcp_editor_prefab::{
        names::EDITOR_SELECT_FACING,
        rows::{FacingRow, SelectFacingReplyRow},
        setup::{paint, prefab_app_and_client, read_map, select_tile},
        tiles::a_plain_palette_tile,
    },
    mcp_shared::{outcome::ran_body, socket::run_editor, support::TestResult},
};

// A cell inside the default grid, off the origin.
const PAINTED_X: i32 = 4;
const PAINTED_Y: i32 = 6;

#[test]
fn the_chosen_facing_reaches_the_piece_the_next_paint_lays() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let tile = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, tile)?;

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_SELECT_FACING, "(facing: East)"),
    )?;
    let chosen: SelectFacingReplyRow = ran_body(&reply, EDITOR_SELECT_FACING)?;
    assert_eq!(
        chosen.facing,
        FacingRow::East,
        "the reply names the facing the session now holds",
    );

    paint(&mut app, &mut client, PAINTED_X, PAINTED_Y)?;
    app.update();

    let read = read_map(&mut app, &mut client, 0)?;
    let [row] = read.painted.as_slice() else {
        return Err(format!("one paint leaves exactly one slot on that storey: {read:?}").into());
    };
    assert_eq!(
        row.facing,
        FacingRow::East,
        "the paint turns its piece to the facing the session was told to hold, and a handler \
         writing the default would leave it North: {row:?}",
    );
    Ok(())
}
