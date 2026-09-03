use std::path::PathBuf;

use super::fixtures::{THEME_SOURCE_FILE, asked_key, theme_def, theme_sources, themes};
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
        None,
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
        None,
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

#[test]
fn a_loaded_theme_carries_the_file_it_was_read_from() {
    let def = theme_def();
    let registry = themes(&def);
    let sources = theme_sources(&def);
    let mut draft = ThemeDraft::default();
    let mut session = MapEditorSession::default();

    let key = asked_key(&(*def.key).to_string());
    let found = load_theme(&mut draft, &mut session, &registry, Some(&sources), &key);

    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {key:?}");
    assert_eq!(
        draft.source().map(|path| (**path).clone()),
        Some(PathBuf::from(THEME_SOURCE_FILE)),
        "the load records the file the def was read from; a load that fills the draft and drops \
         the path leaves the next save rebuilding one from the display name",
    );
}

#[test]
fn a_theme_loaded_with_no_path_table_carries_no_file() {
    let def = theme_def();
    let registry = themes(&def);
    let mut draft = ThemeDraft::default();
    let mut session = MapEditorSession::default();

    let key = asked_key(&(*def.key).to_string());
    let found = load_theme(&mut draft, &mut session, &registry, None, &key);

    assert_eq!(found, KeyLookup::Loaded, "the fixture holds {key:?}");
    assert_eq!(
        draft.source(),
        None,
        "with no path table in the world the load still fills the draft, and the save falls \
         back to the display name rather than writing an invented path",
    );
}
