use gdtf_content_editor::{EditorMode, SpriteDraft};
use gdtf_qa_protocol::{
    command::{CommandAvailability, CommandName, UnavailableCode},
    message::{QaRequest, QaResponse},
};

use crate::{
    names::{EDITOR_LIST_OP, EDITOR_SET_FIELD},
    setup::form_tab_app_and_client,
    support::{TestError, TestResult},
};

// What the catalogue publishes for one command right now.
fn availability_of(
    reply: &QaResponse,
    command: &'static str,
) -> Result<CommandAvailability, TestError> {
    let QaResponse::Catalogue(catalogue) = reply else {
        return Err(format!("expected a Catalogue reply, got {reply:?}").into());
    };
    let Some(entry) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(command))
    else {
        return Err(format!("the catalogue carries no row for `{command}`: {catalogue:?}").into());
    };
    Ok(entry.availability.clone())
}

// The summary the catalogue publishes for one command right now.
fn summary_of(reply: &QaResponse, command: &'static str) -> Result<String, TestError> {
    let QaResponse::Catalogue(catalogue) = reply else {
        return Err(format!("expected a Catalogue reply, got {reply:?}").into());
    };
    let Some(entry) = catalogue
        .entries
        .iter()
        .find(|entry| entry.command == CommandName::from_static(command))
    else {
        return Err(format!("the catalogue carries no row for `{command}`: {catalogue:?}").into());
    };
    Ok(entry.summary.as_str().to_owned())
}

#[test]
fn a_form_tab_whose_draft_is_gone_publishes_the_write_as_unavailable() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;

    let open = client.exchange(&mut app, &QaRequest::Catalogue)?;
    assert_eq!(
        availability_of(&open, EDITOR_SET_FIELD)?,
        CommandAvailability::Available,
        "the Sprite tab is open and its draft is in the world, so the write answers",
    );

    app.world_mut().remove_resource::<SpriteDraft>();
    let gone = client.exchange(&mut app, &QaRequest::Catalogue)?;
    let CommandAvailability::Unavailable { code, note } = availability_of(&gone, EDITOR_SET_FIELD)?
    else {
        return Err(
            "with the open tab's draft out of the world the write must publish as Unavailable"
                .to_owned()
                .into(),
        );
    };
    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "the tab is open but its model is gone, which is what WrongState names",
    );
    assert!(
        !note.as_str().is_empty(),
        "the refusal carries the line that tells a client what is missing",
    );
    Ok(())
}

#[test]
fn neither_write_commands_summary_names_a_single_form() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Sprite)?;
    let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;

    for command in [EDITOR_SET_FIELD, EDITOR_LIST_OP] {
        let summary = summary_of(&catalogue, command)?;
        assert!(
            !summary.is_empty(),
            "`{command}` carries the one line a client reads to learn what it does",
        );
        assert!(
            !summary.contains("Terrain"),
            "`{command}` writes every form's draft, so its summary must not name one form: \
             `{summary}`",
        );
    }
    Ok(())
}
