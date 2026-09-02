use crate::{
    rows::PairingRow,
    setup::{paint, prefab_app_and_client, read_map, select_tile},
    support::TestResult,
    tiles::a_stair_palette_tile,
};

// A cell inside the default grid with a storey above it, so the pair has somewhere to land.
const PAINTED_X: i32 = 5;
const PAINTED_Y: i32 = 2;

#[test]
fn painting_an_up_connector_names_the_paired_tile_and_the_slot_it_landed_in() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let stair = a_stair_palette_tile(&app)?;
    select_tile(&mut app, &mut client, stair)?;

    let body = paint(&mut app, &mut client, PAINTED_X, PAINTED_Y)?;
    app.update();

    let Some(PairingRow::PairPlaced { paired, at }) = body.pairing else {
        return Err(format!(
            "a staircase on a storey with room above it places its second end one storey up, \
             and without that in the reply the next map read shows a cell the caller never \
             asked for: {body:?}"
        )
        .into());
    };
    assert_eq!(
        paired,
        (*stair).to_string(),
        "the second end is the painted tile itself, now that one def carries every view",
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
        row.tile, paired,
        "the storey above holds the tile the reply named, so a client can read back what it \
         did not ask for: {row:?}",
    );
    Ok(())
}
