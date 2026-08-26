use bevy::app::App;
use gdtf_content_editor::{
    ArmorDraft, AttachmentDraft, EditorMode, GangDraft, InjuryDraft, MeleeWeaponDraft, SpriteDraft,
    TerrainDraft,
};
use gdtf_qa_protocol::message::QaResponse;

use crate::{
    harness::editing_app_and_client,
    names::{EDITOR_LIST_OP, EDITOR_SET_FIELD, EDITOR_SET_MODE},
    outcome::ran_body,
    rows::{ListOpReplyRow, SetFieldReplyRow},
    socket::{Client, run_editor},
    support::TestError,
};

/// The mode tab the world has open right now.
pub(crate) fn editor_mode(app: &App) -> Result<EditorMode, TestError> {
    let Some(mode) = app.world().get_resource::<EditorMode>() else {
        return Err("the mode tab is a resource the editor creates on entering Editing".into());
    };
    Ok(*mode)
}

/// An editing app with one form tab open, and no egui pass, so the draft is that form's default.
pub(crate) fn form_tab_app_and_client(mode: EditorMode) -> Result<(App, Client), TestError> {
    let (mut app, mut client) = editing_app_and_client()?;
    let arguments = format!("(mode: {mode:?})");
    client.exchange(&mut app, &run_editor(EDITOR_SET_MODE, &arguments))?;
    let open = editor_mode(&app)?;
    if open != mode {
        return Err(
            format!("the {mode:?} tab must be open before a case runs, got {open:?}").into(),
        );
    }
    Ok((app, client))
}

/// The Armor draft the world holds right now.
pub(crate) fn armor_draft(app: &App) -> Result<ArmorDraft, TestError> {
    let Some(draft) = app.world().get_resource::<ArmorDraft>() else {
        return Err("the Armor draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The Sprite draft the world holds right now.
pub(crate) fn sprite_draft(app: &App) -> Result<SpriteDraft, TestError> {
    let Some(draft) = app.world().get_resource::<SpriteDraft>() else {
        return Err("the Sprite draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The Attachment draft the world holds right now.
pub(crate) fn attachment_draft(app: &App) -> Result<AttachmentDraft, TestError> {
    let Some(draft) = app.world().get_resource::<AttachmentDraft>() else {
        return Err(
            "the Attachment draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.clone())
}

/// The Terrain draft the world holds right now.
pub(crate) fn terrain_draft(app: &App) -> Result<TerrainDraft, TestError> {
    let Some(draft) = app.world().get_resource::<TerrainDraft>() else {
        return Err(
            "the Terrain draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.clone())
}

/// The Injury draft the world holds right now.
pub(crate) fn injury_draft(app: &App) -> Result<InjuryDraft, TestError> {
    let Some(draft) = app.world().get_resource::<InjuryDraft>() else {
        return Err("the Injury draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The Melee Weapon draft the world holds right now.
pub(crate) fn melee_weapon_draft(app: &App) -> Result<MeleeWeaponDraft, TestError> {
    let Some(draft) = app.world().get_resource::<MeleeWeaponDraft>() else {
        return Err(
            "the Melee Weapon draft is a resource the editor creates on entering Editing".into(),
        );
    };
    Ok(draft.clone())
}

/// The Gang draft the world holds right now.
pub(crate) fn gang_draft(app: &App) -> Result<GangDraft, TestError> {
    let Some(draft) = app.world().get_resource::<GangDraft>() else {
        return Err("the Gang draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// Run `editor.set_field` and read the field the reply carries.
pub(crate) fn set_field(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<SetFieldReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SET_FIELD, arguments))?;
    ran_body(&reply, EDITOR_SET_FIELD)
}

/// Run `editor.set_field` and hand back whatever outcome it answered.
pub(crate) fn try_set_field(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<QaResponse, TestError> {
    client.exchange(app, &run_editor(EDITOR_SET_FIELD, arguments))
}

/// Run `editor.list_op` and read the list the reply carries.
pub(crate) fn list_op(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<ListOpReplyRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_LIST_OP, arguments))?;
    ran_body(&reply, EDITOR_LIST_OP)
}

/// Run `editor.list_op` and hand back whatever outcome it answered.
pub(crate) fn try_list_op(
    app: &mut App,
    client: &mut Client,
    arguments: &str,
) -> Result<QaResponse, TestError> {
    client.exchange(app, &run_editor(EDITOR_LIST_OP, arguments))
}
