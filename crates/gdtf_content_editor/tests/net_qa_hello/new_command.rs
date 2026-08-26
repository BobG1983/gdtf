use crate::{
    client::EDITOR_NEW,
    harness::editing_app_and_client,
    lifecycle::{
        armor_draft, field_draft, first_armor_key, first_field_key, load_by_key, new_mode,
    },
    outcome::ran_body,
    rows::{LoadOutcomeRow, NewOutcomeRow, NewReplyRow, RefusalRow},
    socket::run_editor,
    support::TestResult,
};

/// The two modes whose form draws no New button.
const NO_NEW_BUTTON: [&str; 2] = ["Terrain", "Prefab"];

#[test]
fn new_blanks_a_loaded_armor_draft_and_settles_its_autoload() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_armor_key(&app)?;
    let loaded = load_by_key(&mut app, &mut client, "Armor", &key)?;
    let LoadOutcomeRow::Loaded { .. } = loaded else {
        unreachable!("`{key}` came from the live registry, got {loaded:?}");
    };
    assert_eq!(
        armor_draft(&app)?.name(),
        key,
        "the draft starts this case holding the armor that was loaded into it",
    );

    let reply = client.exchange(&mut app, &run_editor(EDITOR_NEW, "(mode: Armor)"))?;
    let body: NewReplyRow = ran_body(&reply, EDITOR_NEW)?;

    assert_eq!(
        body.outcome,
        NewOutcomeRow::Blanked,
        "the draft was blanked"
    );
    let draft = armor_draft(&app)?;
    assert!(
        draft.name().is_empty(),
        "the blanked draft carries no name, so the armor loaded a moment ago is gone: {draft:?}",
    );
    assert!(
        !draft.autoload_pending(),
        "the blanked draft's autoload flag is settled — a draft built with `default()` instead \
         of `ArmorDraft::new_armor()` would leave it Pending, and the form's own sync would seed \
         the first sorted registry entry over it on the next egui frame",
    );
    Ok(())
}

#[test]
fn new_blanks_a_loaded_field_draft_and_settles_its_autoload() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_field_key(&app)?;
    let loaded = load_by_key(&mut app, &mut client, "Field", &key)?;
    let LoadOutcomeRow::Loaded { .. } = loaded else {
        unreachable!("`{key}` came from the live registry, got {loaded:?}");
    };
    assert_eq!(field_draft(&app)?.key(), key);

    let reply = client.exchange(&mut app, &run_editor(EDITOR_NEW, "(mode: Field)"))?;
    let body: NewReplyRow = ran_body(&reply, EDITOR_NEW)?;
    assert_eq!(body.outcome, NewOutcomeRow::Blanked);

    let draft = field_draft(&app)?;
    assert!(
        draft.key().is_empty(),
        "the blanked draft carries no stem, so the field loaded a moment ago is gone: {draft:?}",
    );
    assert!(
        draft.immune_armor_types().is_empty(),
        "the blank Field draft's immune list is empty, which is the legal empty list",
    );
    assert!(
        !draft.autoload_pending(),
        "the blanked draft's autoload flag is settled. A draft built with `default()` instead \
         of `FieldDraft::new_field()` would leave it Pending, and the form's own sync would seed \
         the first registry entry over it on the next egui frame",
    );
    Ok(())
}

#[test]
fn new_refuses_terrain_and_prefab_because_neither_form_draws_a_new_button() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    for mode in NO_NEW_BUTTON {
        let outcome = new_mode(&mut app, &mut client, mode)?;
        assert_eq!(
            outcome,
            NewOutcomeRow::Refused(RefusalRow::NoNewAction),
            "{mode} draws no New button, so there is no constructor the form calls and blanking \
             it would be inventing editor behaviour — the refusal names that reason inside a Ran \
             reply, never Unavailable and never Theme's sync reason",
        );
    }
    Ok(())
}

#[test]
fn new_refuses_theme_because_the_forms_sync_would_undo_it() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_NEW, "(mode: Theme)"))?;
    let body: NewReplyRow = ran_body(&reply, EDITOR_NEW)?;
    assert_eq!(
        body.outcome,
        NewOutcomeRow::Refused(RefusalRow::ThemeNewIsUndoneBySync),
        "Theme has a blank-draft constructor; what stops it is the theme form's own sync \
         reloading the session theme over any draft whose key differs, so the refusal must name \
         that and not the missing-New-action reason Terrain and Prefab carry",
    );
    Ok(())
}
