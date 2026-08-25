use crate::{
    keys::{A_KEY_THAT_IS_NOT_UUID_TEXT, a_non_slab_in_the_draft, a_slab_in_the_draft, key_args},
    names::EDITOR_SET_DEFAULT_FLOOR,
    outcome::{ran_body, unavailable_code},
    rows::SetDefaultFloorReplyRow,
    setup::theme_tab_app_and_client,
    socket::run_editor,
    support::TestResult,
    world::theme_draft,
};

#[test]
fn setting_a_slab_the_draft_holds_moves_the_default_floor_onto_it() -> TestResult {
    let (mut app, mut client) = theme_tab_app_and_client()?;
    let slab = a_slab_in_the_draft(&app)?;
    let before = theme_draft(&app)?.default_floor();

    let args = key_args(&(*slab).to_string());
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SET_DEFAULT_FLOOR, &args))?;
    let body: SetDefaultFloorReplyRow = ran_body(&reply, EDITOR_SET_DEFAULT_FLOOR)?;
    app.update();

    assert_ne!(
        before,
        Some(slab),
        "the case picked a slab the draft was not already floored on, so a handler that wrote \
         nothing leaves the floor where it was and this must catch it",
    );
    assert_eq!(
        body.default_floor,
        Some((*slab).to_string()),
        "the reply names the floor the draft now holds",
    );
    assert_eq!(
        theme_draft(&app)?.default_floor(),
        Some(slab),
        "the draft in the world carries the floor the reply reported, read after a later frame",
    );
    Ok(())
}

#[test]
fn a_terrain_that_is_not_a_slab_is_refused_and_the_draft_stands() -> TestResult {
    let (mut app, mut client) = theme_tab_app_and_client()?;
    let other = a_non_slab_in_the_draft(&app)?;
    let before = theme_draft(&app)?;
    assert!(
        before.has_terrain(other),
        "the case must name a terrain the draft already holds, or it would prove nothing about \
         the slab rule",
    );

    let args = key_args(&(*other).to_string());
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SET_DEFAULT_FLOOR, &args))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the draft's own setter drops a floor the picker would not offer, so answering Ran would \
         report a write that never happened",
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
fn a_floor_key_that_is_not_uuid_text_is_refused_and_the_draft_stands() -> TestResult {
    let (mut app, mut client) = theme_tab_app_and_client()?;
    let before = theme_draft(&app)?;

    let args = key_args(A_KEY_THAT_IS_NOT_UUID_TEXT);
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SET_DEFAULT_FLOOR, &args))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "text no UUID parser accepts names a terrain the editor cannot hold, and that is a \
         different refusal from a slab rule the picker enforces on a key it can read",
    );
    app.update();
    assert_eq!(
        theme_draft(&app)?,
        before,
        "a refused call writes nothing: the draft must hold every value it held before",
    );
    Ok(())
}
