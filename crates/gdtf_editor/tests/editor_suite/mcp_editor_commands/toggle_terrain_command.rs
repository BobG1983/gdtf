use crate::{
    mcp_editor_commands::{
        drafts::theme_draft,
        keys::{
            A_KEY_THAT_IS_NOT_UUID_TEXT, a_key_no_terrain_holds, a_terrain_outside_the_draft,
            key_args,
        },
        names::EDITOR_TOGGLE_TERRAIN,
        rows::{ToggleRow, ToggleTerrainReplyRow},
        setup::theme_tab_app_and_client,
    },
    mcp_shared::{
        outcome::{ran_body, unavailable_code},
        socket::run_editor,
        support::TestResult,
    },
};

#[test]
fn a_toggle_adds_a_terrain_then_takes_the_default_floor_back_off() -> TestResult {
    let (mut app, mut client) = theme_tab_app_and_client()?;
    let added = a_terrain_outside_the_draft(&app)?;
    let Some(floor) = theme_draft(&app)?.default_floor() else {
        return Err("this case needs the loaded theme to carry a default floor".into());
    };

    let args = key_args(&(*added).to_string());
    let reply = client.exchange(&mut app, &run_editor(EDITOR_TOGGLE_TERRAIN, &args))?;
    let body: ToggleTerrainReplyRow = ran_body(&reply, EDITOR_TOGGLE_TERRAIN)?;
    app.update();

    assert_eq!(
        body.toggle,
        ToggleRow::Added,
        "the draft did not hold that terrain, so the tick put it in and the reply must say which \
         way it went",
    );
    assert_eq!(
        body.default_floor,
        Some((*floor).to_string()),
        "adding a terrain leaves the default floor where it was",
    );
    assert!(
        theme_draft(&app)?.has_terrain(added),
        "the draft in the world holds the terrain the reply reported adding",
    );

    let args = key_args(&(*floor).to_string());
    let reply = client.exchange(&mut app, &run_editor(EDITOR_TOGGLE_TERRAIN, &args))?;
    let body: ToggleTerrainReplyRow = ran_body(&reply, EDITOR_TOGGLE_TERRAIN)?;
    app.update();

    assert_eq!(
        body.toggle,
        ToggleRow::Removed,
        "the draft held that terrain, so the same tick took it out, and a reply that always says \
         Added cannot tell the two apart",
    );
    assert_eq!(
        body.default_floor, None,
        "taking out the terrain that was the default floor clears that floor, and a client that \
         cannot read the floor back would not know",
    );
    let draft = theme_draft(&app)?;
    assert!(
        !draft.has_terrain(floor),
        "the draft in the world no longer holds the terrain the reply reported removing",
    );
    assert_eq!(
        draft.default_floor(),
        None,
        "the draft in the world lost the default floor with it",
    );
    Ok(())
}

#[test]
fn a_terrain_key_no_registry_holds_is_refused_and_the_draft_stands() -> TestResult {
    let (mut app, mut client) = theme_tab_app_and_client()?;
    let key = a_key_no_terrain_holds(&mut app, &mut client)?;
    let before = theme_draft(&app)?;

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_TOGGLE_TERRAIN, &key_args(&key)),
    )?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "the terrain library only ever offers keys the registry holds, so a key it does not hold \
         names a model the editor does not have",
    );
    app.update();
    assert_eq!(
        theme_draft(&app)?,
        before,
        "a refused call writes nothing: the draft must hold every value it held before",
    );
    Ok(())
}

#[test]
fn a_terrain_key_that_is_not_uuid_text_is_refused_and_the_draft_stands() -> TestResult {
    let (mut app, mut client) = theme_tab_app_and_client()?;
    let before = theme_draft(&app)?;

    let args = key_args(A_KEY_THAT_IS_NOT_UUID_TEXT);
    let reply = client.exchange(&mut app, &run_editor(EDITOR_TOGGLE_TERRAIN, &args))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "text no UUID parser accepts names a terrain the editor cannot hold, so it is refused \
         the way a key the registry misses is",
    );
    app.update();
    assert_eq!(
        theme_draft(&app)?,
        before,
        "a refused call writes nothing: the draft must hold every value it held before",
    );
    Ok(())
}
