use crate::{
    rows::PairingRow,
    setup::{paint, prefab_app_and_client, read_map, select_tile},
    support::TestResult,
    tiles::an_up_connector_palette_tile,
};

// A cell inside the default grid with a storey above it, so the pair has somewhere to land.
const PAINTED_X: i32 = 5;
const PAINTED_Y: i32 = 2;

#[test]
fn painting_an_up_connector_names_the_paired_tile_and_the_slot_it_landed_in() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let up = an_up_connector_palette_tile(&app)?;
    select_tile(&mut app, &mut client, up)?;

    let body = paint(&mut app, &mut client, PAINTED_X, PAINTED_Y)?;
    app.update();

    let Some(PairingRow::PairPlaced { down, at }) = body.pairing else {
        return Err(format!(
            "an up connector on a storey with room above it pairs a down connector one storey \
             up, and without that in the reply the next map read shows a cell the caller never \
             asked for: {body:?}"
        )
        .into());
    };
    assert_ne!(
        down,
        (*up).to_string(),
        "the pair is the down counterpart, not a second copy of the tile that was painted",
    );
    assert_eq!(
        (at.x, at.y, at.level),
        (PAINTED_X, PAINTED_Y, 1),
        "the pair lands in the same cell one storey up, and a reply naming the painted storey \
         would send a client to the wrong level: {at:?}",
    );

    let above = read_map(&mut app, &mut client, at.level)?;
    let [row] = above.painted.as_slice() else {
        return Err(format!(
            "the storey above holds exactly the one slot the pairing pass wrote: {above:?}"
        )
        .into());
    };
    assert_eq!(
        row.tile, down,
        "the storey above holds the down connector the reply named, so a client can read back \
         what it did not ask for: {row:?}",
    );
    Ok(())
}
