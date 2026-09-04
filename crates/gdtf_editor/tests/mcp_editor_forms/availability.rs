use bevy::prelude::*;
use cobalt_mcp_command::dispatch::{CommandInbox, QaCommandSystems};
use cobalt_mcp_protocol::{
    command::{CommandAvailability, CommandName, CommandOutcome, UnavailableCode},
    message::{QaRequest, QaResponse},
};
use gdtf_editor::{EditorMode, FieldDraft, GangDraft, InjuryDraft, SpriteDraft, WeaponDraft};

use crate::{
    names::{EDITOR_LIST_OP, EDITOR_SET_FIELD},
    outcome::unavailable_code,
    refusal::refusal_note,
    setup::{form_tab_app_and_client, try_list_op, try_set_field},
    socket::Client,
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

#[test]
fn the_catalogue_offers_both_writes_on_every_form_tab_that_draws_a_draft() -> TestResult {
    for mode in [
        EditorMode::Terrain,
        EditorMode::Injury,
        EditorMode::MeleeWeapon,
        EditorMode::Gang,
        EditorMode::Weapon,
        EditorMode::Field,
    ] {
        let (mut app, mut client) = form_tab_app_and_client(mode)?;
        let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;
        for command in [EDITOR_SET_FIELD, EDITOR_LIST_OP] {
            assert_eq!(
                availability_of(&catalogue, command)?,
                CommandAvailability::Available,
                "`{command}` writes the {mode:?} form's draft, so the catalogue offers it while \
                 that tab is open",
            );
        }
    }
    Ok(())
}

#[test]
fn the_gang_tab_with_no_draft_refuses_both_the_catalogue_row_and_the_write() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Gang)?;
    app.world_mut().remove_resource::<GangDraft>();

    let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;
    let CommandAvailability::Unavailable { code, note } =
        availability_of(&catalogue, EDITOR_SET_FIELD)?
    else {
        return Err(
            "with the Gang draft out of the world the write must publish as Unavailable"
                .to_owned()
                .into(),
        );
    };
    assert_eq!(
        code,
        UnavailableCode::WrongState,
        "the Gang tab is open but its draft is gone, which is what WrongState names",
    );
    assert!(
        !note.as_str().is_empty(),
        "the refusal carries the line that tells a client what is missing",
    );

    let reply = try_set_field(&mut app, &mut client, "(field: Gang(Name(\"Ash Ferals\")))")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "a missing draft is host state, so the handler names the code the catalogue names",
    );
    assert!(
        !refusal_note(&reply)?.is_empty(),
        "the write's refusal carries the line that tells a client what is missing",
    );
    Ok(())
}

#[test]
fn the_weapon_tab_with_no_draft_refuses_both_the_catalogue_row_and_the_write() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Weapon)?;
    app.world_mut().remove_resource::<WeaponDraft>();

    let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;
    for command in [EDITOR_SET_FIELD, EDITOR_LIST_OP] {
        let CommandAvailability::Unavailable { code, note } = availability_of(&catalogue, command)?
        else {
            return Err(format!(
                "with the Weapon draft out of the world `{command}` must publish as Unavailable"
            )
            .into());
        };
        assert_eq!(
            code,
            UnavailableCode::WrongState,
            "the Weapon tab is open but its draft is gone, which is what WrongState names",
        );
        assert!(
            !note.as_str().is_empty(),
            "the refusal carries the line that tells a client what is missing",
        );
    }

    let field = try_set_field(&mut app, &mut client, "(field: Weapon(Damage(9)))")?;
    assert_eq!(
        unavailable_code(&field)?,
        "WrongState",
        "a missing draft is host state, so the handler names the code the catalogue names, not \
         MissingModel",
    );
    assert!(!refusal_note(&field)?.is_empty());

    let list = try_list_op(&mut app, &mut client, "(list: WeaponFireModes, op: Add)")?;
    assert_eq!(unavailable_code(&list)?, "WrongState");
    Ok(())
}

#[test]
fn the_field_tab_with_no_draft_refuses_both_the_catalogue_row_and_the_write() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Field)?;
    app.world_mut().remove_resource::<FieldDraft>();

    let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;
    for command in [EDITOR_SET_FIELD, EDITOR_LIST_OP] {
        let CommandAvailability::Unavailable { code, note } = availability_of(&catalogue, command)?
        else {
            return Err(format!(
                "with the Field draft out of the world `{command}` must publish as Unavailable"
            )
            .into());
        };
        assert_eq!(
            code,
            UnavailableCode::WrongState,
            "the Field tab is open but its draft is gone, which is what WrongState names",
        );
        assert!(!note.as_str().is_empty());
    }

    let field = try_set_field(&mut app, &mut client, "(field: Field(Damage(6)))")?;
    assert_eq!(unavailable_code(&field)?, "WrongState");
    let list = try_list_op(
        &mut app,
        &mut client,
        "(list: FieldImmuneArmorTypes, op: Toggle(ImmuneArmorType(Flak)))",
    )?;
    assert_eq!(unavailable_code(&list)?, "WrongState");
    Ok(())
}

#[test]
fn the_catalogue_refuses_both_writes_on_the_prefab_tab() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Prefab)?;
    let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;

    for command in [EDITOR_SET_FIELD, EDITOR_LIST_OP] {
        let CommandAvailability::Unavailable { code, note } = availability_of(&catalogue, command)?
        else {
            return Err(format!("`{command}` holds no draft on the Prefab tab").into());
        };
        assert_eq!(code, UnavailableCode::WrongState);
        assert!(
            note.as_str().contains("Prefab"),
            "the note names the tab that is open, got `{}`",
            note.as_str(),
        );
    }
    Ok(())
}

#[test]
fn a_write_with_no_draft_names_the_same_code_the_catalogue_does() -> TestResult {
    let (mut app, mut client) = form_tab_app_and_client(EditorMode::Injury)?;
    app.world_mut().remove_resource::<InjuryDraft>();

    let catalogue = client.exchange(&mut app, &QaRequest::Catalogue)?;
    for command in [EDITOR_SET_FIELD, EDITOR_LIST_OP] {
        let CommandAvailability::Unavailable { code, .. } = availability_of(&catalogue, command)?
        else {
            return Err(format!("`{command}` published as available with its draft gone").into());
        };
        assert_eq!(code, UnavailableCode::WrongState);
    }

    let field = try_set_field(&mut app, &mut client, "(field: Injury(Severity(Major)))")?;
    assert_eq!(
        unavailable_code(&field)?,
        "WrongState",
        "a missing draft is host state, so the handler names the code the catalogue names",
    );
    let list = try_list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    assert_eq!(unavailable_code(&list)?, "WrongState");
    Ok(())
}

// The code and the note one refusal carries, so a case can say which refusal answered it.
fn unavailable_parts(reply: &QaResponse) -> Result<(String, String), TestError> {
    let QaResponse::Outcome(CommandOutcome::Unavailable { code, note }) = reply else {
        return Err(format!("expected an Unavailable outcome, got {reply:?}").into());
    };
    Ok((format!("{code:?}"), note.as_str().to_owned()))
}

// Take the Injury draft out of the world in the frame that already admitted the call, so the
// handler meets a missing draft and the availability check cannot answer in its place.
fn drop_the_injury_draft_after_admission(world: &mut World) {
    let admitted = world
        .get_resource::<CommandInbox>()
        .is_some_and(|inbox| !inbox.is_empty());
    if admitted {
        world.remove_resource::<InjuryDraft>();
    }
}

// An Injury app whose draft leaves the world between the router and the handler.
fn injury_app_losing_its_draft_mid_frame() -> Result<(App, Client), TestError> {
    let (mut app, client) = form_tab_app_and_client(EditorMode::Injury)?;
    app.add_systems(
        Update,
        drop_the_injury_draft_after_admission
            .after(QaCommandSystems::Route)
            .before(QaCommandSystems::Claim),
    );
    Ok((app, client))
}

// The handler's own refusal names the mode, which the catalogue's note does not, so a case can
// tell the two apart.
fn assert_the_handler_answered(reply: &QaResponse) -> TestResult {
    let (code, note) = unavailable_parts(reply)?;
    assert!(
        note.contains("Injury"),
        "the handler's own refusal names the open mode, so a note without it came from the \
         availability check instead: `{note}`",
    );
    assert_eq!(
        code, "WrongState",
        "a draft that left the world is host state, so the handler names the code the catalogue \
         names, not MissingModel",
    );
    Ok(())
}

#[test]
fn a_set_field_whose_draft_goes_mid_frame_is_refused_by_the_handler() -> TestResult {
    let (mut app, mut client) = injury_app_losing_its_draft_mid_frame()?;

    let reply = try_set_field(&mut app, &mut client, "(field: Injury(Severity(Major)))")?;
    assert_the_handler_answered(&reply)?;
    assert!(
        app.world().get_resource::<InjuryDraft>().is_none(),
        "the case only proves the handler's arm if the draft really left the world",
    );
    Ok(())
}

#[test]
fn a_list_op_whose_draft_goes_mid_frame_is_refused_by_the_handler() -> TestResult {
    let (mut app, mut client) = injury_app_losing_its_draft_mid_frame()?;

    let reply = try_list_op(&mut app, &mut client, "(list: InjuryEffects, op: Add)")?;
    assert_the_handler_answered(&reply)?;
    assert!(
        app.world().get_resource::<InjuryDraft>().is_none(),
        "the case only proves the handler's arm if the draft really left the world",
    );
    Ok(())
}
