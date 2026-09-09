use crate::{
    mcp_editor_prefab::{
        names::EDITOR_SELECT_TILE,
        rows::SelectTileRefusalRow,
        setup::{prefab_app_and_client, select_tile, select_tile_args},
        tiles::{
            A_KEY_THAT_IS_NOT_UUID_TEXT, a_key_no_terrain_holds, a_plain_palette_tile,
            a_terrain_off_the_palette,
        },
    },
    mcp_shared::{
        outcome::unavailable_code, socket::run_editor, support::TestResult, world::session,
    },
};

#[test]
fn a_tile_the_palette_offers_becomes_the_session_paint_tile() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let tile = a_plain_palette_tile(&app)?;
    assert_ne!(
        session(&app)?.selected_tile(),
        Some(tile),
        "a fresh session has selected nothing, so a handler that wrote nothing would still \
         look right without this",
    );

    let body = select_tile(&mut app, &mut client, tile)?;
    app.update();

    assert_eq!(
        body.refusal, None,
        "the tile is on the theme's palette and the registry holds its def, so both of the \
         palette's conditions hold: {body:?}",
    );
    assert_eq!(
        body.selected_tile,
        Some((*tile).to_string()),
        "the reply names the tile the session now paints with: {body:?}",
    );
    assert_eq!(
        session(&app)?.selected_tile(),
        Some(tile),
        "the session in the world carries the tile the reply reported, read after a later frame",
    );
    Ok(())
}

#[test]
fn a_terrain_the_theme_does_not_list_is_refused_and_the_selection_stands() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let held = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, held)?;
    app.update();
    let off_the_palette = a_terrain_off_the_palette(&app)?;

    let body = select_tile(&mut app, &mut client, off_the_palette)?;
    app.update();

    assert_eq!(
        body.refusal,
        Some(SelectTileRefusalRow::NotInTheThemePalette),
        "the registry holds that def, so the condition it fails is the theme's own list, and a \
         reply that named the other condition would send a client to the wrong fix: {body:?}",
    );
    assert_eq!(
        session(&app)?.selected_tile(),
        Some(held),
        "a refused call writes nothing: a handler that selected first and refused second would \
         leave the session painting with a tile the palette never offered",
    );
    Ok(())
}

#[test]
fn a_key_no_terrain_registry_holds_is_refused_and_the_selection_stands() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let held = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, held)?;
    app.update();
    let unheld = a_key_no_terrain_holds(&app)?;

    let body = select_tile(&mut app, &mut client, unheld)?;
    app.update();

    assert_eq!(
        body.refusal,
        Some(SelectTileRefusalRow::NoTerrainDef),
        "no def answers to that key, so the palette draws no row for it at all: {body:?}",
    );
    assert_eq!(
        session(&app)?.selected_tile(),
        Some(held),
        "a refused call writes nothing: the session must paint with the tile it held before",
    );
    Ok(())
}

#[test]
fn a_tile_key_that_is_not_uuid_text_is_refused_and_the_selection_stands() -> TestResult {
    let (mut app, mut client) = prefab_app_and_client()?;
    let held = a_plain_palette_tile(&app)?;
    select_tile(&mut app, &mut client, held)?;
    app.update();

    let arguments = select_tile_args(A_KEY_THAT_IS_NOT_UUID_TEXT);
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SELECT_TILE, &arguments))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "text no UUID parser accepts names a terrain the editor cannot hold, so it is refused \
         the way every other editor key command refuses it, not as one of the palette's two \
         typed conditions",
    );
    app.update();
    assert_eq!(
        session(&app)?.selected_tile(),
        Some(held),
        "a refused call writes nothing: the session must paint with the tile it held before",
    );
    Ok(())
}
