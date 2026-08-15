use super::fixtures::{asked_key, theme_def, themes};
use crate::{
    net_qa::commands::write::load::{families::KeyLookup, theme::load_theme},
    session::MapEditorSession,
    theme_form::ThemeDraft,
};

#[test]
fn a_loaded_theme_leaves_the_draft_and_the_session_on_the_same_key() {
    let def = theme_def();
    let registry = themes(&def);
    let mut draft = ThemeDraft::default();
    let mut session = MapEditorSession::default();

    let found = load_theme(
        &mut draft,
        &mut session,
        &registry,
        &asked_key(&(*def.key).to_string()),
    );

    assert_eq!(found, KeyLookup::Loaded, "the fixture holds that theme");
    assert_eq!(
        draft.key(),
        def.key,
        "the draft carries the loaded theme's own key",
    );
    assert_eq!(
        session.theme(),
        def.key,
        "the session theme is the loaded key too — the theme form's sync reloads the session \
         theme's def over any draft whose key differs, so a draft-only load is undone on the \
         next egui frame",
    );
}

#[test]
fn a_theme_key_that_does_not_parse_writes_nothing() {
    let def = theme_def();
    let registry = themes(&def);
    let before_draft = ThemeDraft::default();
    let mut draft = before_draft.clone();
    let before_session = MapEditorSession::default();
    let mut session = before_session.clone();

    let found = load_theme(
        &mut draft,
        &mut session,
        &registry,
        &asked_key("not-a-uuid"),
    );

    let KeyLookup::NoSuchKey(known) = found else {
        unreachable!("`not-a-uuid` names no theme, got {found:?}");
    };
    assert!(
        known.iter().any(|key| **key == (*def.key).to_string()),
        "a miss lists every theme key the registry does hold: {known:?}",
    );
    assert_eq!(draft, before_draft, "a miss leaves the draft alone");
    assert_eq!(
        session, before_session,
        "a miss leaves the session theme alone as well as the draft",
    );
}
