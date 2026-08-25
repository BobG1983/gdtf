use crate::{
    canvas::editor_map,
    rows::{FacingRow, IllegalReasonRow, PaintRefusalRow, PairingRow, VerdictRow},
    setup::{paint, prefab_app_and_client, read_map, select_tile},
    support::TestResult,
    tiles::a_plain_palette_tile,
};

// A cell inside the default grid, off the origin, so a paint fixed at 0,0 is caught.
const PAINTED_X: i32 = 3;
const PAINTED_Y: i32 = 7;

// Negative on both axes, so no grid the size fields allow can ever hold it.
const OUTSIDE_X: i32 = -1;
const OUTSIDE_Y: i32 = -4;

#[test]
fn a_legal_paint_lands_the_selected_tile_and_the_map_read_shows_it() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let tile = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, tile)?;

    let body = paint(&mut app, &mut client, PAINTED_X, PAINTED_Y)?;
    app.update();

    let Some(VerdictRow::Legal { auto_clear }) = body.verdict else {
        return Err(format!(
            "an empty grid holds nothing to conflict with, so the rules must allow this slot: \
             {body:?}"
        )
        .into());
    };
    assert_eq!(
        auto_clear, None,
        "nothing was painted above this slot, so the placement clears nothing on its way in: \
         {body:?}",
    );
    assert_eq!(
        body.pairing,
        Some(PairingRow::PlacedNoPair),
        "the case picked a tile that is not an up connector, so the pairing pass places one \
         tile and no second one: {body:?}",
    );

    let read = read_map(&mut app, &mut client, 0)?;
    let [row] = read.painted.as_slice() else {
        return Err(format!("one paint leaves exactly one slot on that storey: {read:?}").into());
    };
    assert_eq!(
        (row.cell.x, row.cell.y, row.cell.level),
        (PAINTED_X, PAINTED_Y, 0),
        "the read names the slot the paint was asked for, x as x and y as y: {row:?}",
    );
    assert_eq!(
        row.tile,
        (*tile).to_string(),
        "the read names the tile the session was told to paint with: {row:?}",
    );
    assert_eq!(
        row.facing,
        FacingRow::North,
        "a canvas click lays every piece at the facing default, so the read reports that side: \
         {row:?}",
    );
    Ok(())
}

#[test]
fn an_illegal_paint_runs_names_its_reason_and_leaves_the_map_alone() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let tile = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, tile)?;
    paint(&mut app, &mut client, PAINTED_X, PAINTED_Y)?;
    app.update();
    let before = editor_map(&app)?.painted_count();

    let body = paint(&mut app, &mut client, OUTSIDE_X, OUTSIDE_Y)?;
    app.update();

    assert_eq!(
        body.verdict,
        Some(VerdictRow::Illegal(IllegalReasonRow::OutOfBounds)),
        "the slot sits off the grid on both axes, and the verdict is the reply's content here, \
         not a host-state refusal: {body:?}",
    );
    assert_eq!(
        body.pairing,
        Some(PairingRow::Rejected),
        "the rules turned the placement down, so the pairing pass wrote nothing either: {body:?}",
    );
    assert_eq!(
        editor_map(&app)?.painted_count(),
        before,
        "an illegal paint writes nothing: the map must hold every slot it held before",
    );
    Ok(())
}

#[test]
fn a_paint_with_no_tile_selected_refuses_and_the_map_stays_empty() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    assert_eq!(
        editor_map(&app)?.painted_count(),
        0,
        "a fresh canvas holds nothing painted, or this case could not tell a refusal from a \
         write",
    );

    let body = paint(&mut app, &mut client, PAINTED_X, PAINTED_Y)?;
    app.update();

    assert_eq!(
        body.refusal,
        Some(PaintRefusalRow::NoSelectedTile),
        "a canvas click with no tile selected lays nothing down, so the command says so in the \
         reply body rather than reporting a write: {body:?}",
    );
    assert_eq!(
        body.verdict, None,
        "there was no tile to evaluate, so the reply carries no verdict for the slot: {body:?}",
    );
    assert_eq!(
        editor_map(&app)?.painted_count(),
        0,
        "a refused paint writes nothing",
    );
    Ok(())
}
