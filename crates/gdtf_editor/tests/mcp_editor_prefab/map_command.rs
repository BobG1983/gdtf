use crate::{
    setup::{paint, prefab_app_and_client, read_map, select_tile, set_grid_size, set_level},
    support::TestResult,
    tiles::a_plain_palette_tile,
    world::session,
};

// One cell per storey, off each other's coordinates, so a whole-map read is caught.
const GROUND_X: i32 = 2;
const GROUND_Y: i32 = 9;
const UPPER_X: i32 = 8;
const UPPER_Y: i32 = 1;

// The storey the second paint lands on, which the default grid has room for.
const UPPER_LEVEL: u8 = 1;

// Far enough out that a shrink to a small grid leaves it outside.
const FAR_X: i32 = 50;
const FAR_Y: i32 = 44;

// A grid small enough to leave the far cell outside it, and inside every band.
const SHRUNK_SPAN: u8 = 10;

#[test]
fn each_storey_reads_back_its_own_cells_and_nothing_from_the_other() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let tile = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, tile)?;
    paint(&mut app, &mut client, GROUND_X, GROUND_Y)?;
    set_level(&mut app, &mut client, UPPER_LEVEL)?;
    paint(&mut app, &mut client, UPPER_X, UPPER_Y)?;
    app.update();

    let ground = read_map(&mut app, &mut client, 0)?;
    let [on_ground] = ground.painted.as_slice() else {
        return Err(
            format!("the ground storey holds the one cell painted there: {ground:?}").into(),
        );
    };
    assert_eq!(
        (on_ground.cell.x, on_ground.cell.y, on_ground.cell.level),
        (GROUND_X, GROUND_Y, 0),
        "the ground read answers its own storey's cell: {ground:?}",
    );

    let upper = read_map(&mut app, &mut client, UPPER_LEVEL)?;
    let [on_upper] = upper.painted.as_slice() else {
        return Err(format!("the upper storey holds the one cell painted there: {upper:?}").into());
    };
    assert_eq!(
        (on_upper.cell.x, on_upper.cell.y, on_upper.cell.level),
        (UPPER_X, UPPER_Y, UPPER_LEVEL),
        "the upper read answers its own storey's cell, so a read that ignored the argument \
         would hand back the ground cell here: {upper:?}",
    );
    Ok(())
}

#[test]
fn a_storey_with_nothing_painted_reads_back_an_empty_list() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let read = read_map(&mut app, &mut client, 0)?;
    assert!(
        read.painted.is_empty(),
        "a canvas nothing has been painted on answers no rows rather than failing: {read:?}",
    );
    assert_eq!(
        read.grid_size.levels,
        *session(&app)?.grid_size().levels(),
        "the read carries the grid the session holds, whatever is painted on it: {read:?}",
    );
    Ok(())
}

#[test]
fn a_cell_left_outside_the_grid_by_a_shrink_is_still_listed() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let tile = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, tile)?;
    paint(&mut app, &mut client, FAR_X, FAR_Y)?;
    let levels = *session(&app)?.grid_size().levels();
    let shrunk = set_grid_size(&mut app, &mut client, SHRUNK_SPAN, SHRUNK_SPAN, levels)?;
    app.update();
    assert!(
        i32::from(shrunk.grid_size.width) <= FAR_X,
        "the shrink must leave the painted cell outside the grid, or this case proves nothing: \
         {shrunk:?}",
    );

    let read = read_map(&mut app, &mut client, 0)?;
    let [row] = read.painted.as_slice() else {
        return Err(format!(
            "nothing prunes the map on a shrink, so the slot left outside the grid is still \
             painted and must still be listed: {read:?}"
        )
        .into());
    };
    assert_eq!(
        (row.cell.x, row.cell.y),
        (FAR_X, FAR_Y),
        "the leftover slot is the one a save would report as an illegal cell, so the read must \
         hand it back rather than filter it out: {read:?}",
    );
    assert_eq!(
        read.grid_size.width, shrunk.grid_size.width,
        "the read carries the grid the session holds now, not the one the cell was painted on: \
         {read:?}",
    );
    Ok(())
}
