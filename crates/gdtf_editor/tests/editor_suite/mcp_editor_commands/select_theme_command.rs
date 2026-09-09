use crate::{
    mcp_editor_commands::{
        keys::{
            A_KEY_THAT_IS_NOT_UUID_TEXT, a_key_no_theme_holds, a_theme_off_the_session, key_args,
        },
        names::EDITOR_SELECT_THEME,
        rows::SelectThemeReplyRow,
    },
    mcp_shared::{
        harness::editing_app_and_client,
        outcome::{ran_body, unavailable_code},
        socket::run_editor,
        support::TestResult,
        world::{session, theme_registry},
    },
};

#[test]
fn select_theme_moves_the_session_onto_that_theme_and_its_default_floor() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let before = session(&app)?;
    let key = a_theme_off_the_session(&mut app, &mut client)?;

    let reply = client.exchange(&mut app, &run_editor(EDITOR_SELECT_THEME, &key_args(&key)))?;
    let body: SelectThemeReplyRow = ran_body(&reply, EDITOR_SELECT_THEME)?;
    app.update();

    let after = session(&app)?;
    assert_ne!(
        after.theme(),
        before.theme(),
        "the case picked a theme the session was not on, so a handler that wrote nothing leaves \
         the session where it was and this must catch it",
    );
    assert_eq!(
        (*after.theme()).to_string(),
        key,
        "the session holds the theme the call named, read out of the world the answering frame \
         left",
    );
    assert_eq!(
        body.theme, key,
        "the reply names the theme the session now holds, so a client needs no second read",
    );

    let themes = theme_registry(&app)?;
    let registry_floor = themes
        .default_floor(&after.theme())
        .map(|floor| (*floor).to_string());
    assert_eq!(
        body.default_floor, registry_floor,
        "picking a theme takes that theme's own default floor with it, the way the top bar's \
         picker does",
    );
    assert_eq!(
        after.default_floor().map(|floor| (*floor).to_string()),
        registry_floor,
        "the session took the floor as well as the theme, not the theme alone",
    );
    Ok(())
}

#[test]
fn a_theme_key_no_registry_holds_is_refused_and_the_session_stands() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let before = session(&app)?;
    let key = a_key_no_theme_holds(&app)?;

    let reply = client.exchange(&mut app, &run_editor(EDITOR_SELECT_THEME, &key_args(&key)))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "a key no theme answers to names a model the editor does not have, and the call is \
         refused rather than moving the session onto a theme with no def",
    );
    app.update();
    assert_eq!(
        session(&app)?,
        before,
        "a refused call writes nothing: the session must hold every value it held before",
    );
    Ok(())
}

#[test]
fn a_theme_key_that_is_not_uuid_text_is_refused_and_the_session_stands() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let before = session(&app)?;

    let args = key_args(A_KEY_THAT_IS_NOT_UUID_TEXT);
    let reply = client.exchange(&mut app, &run_editor(EDITOR_SELECT_THEME, &args))?;
    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "text no UUID parser accepts names a theme the editor cannot hold, so it is refused the \
         way a key the registry misses is",
    );
    app.update();
    assert_eq!(
        session(&app)?,
        before,
        "a refused call writes nothing: the session must hold every value it held before",
    );
    Ok(())
}
