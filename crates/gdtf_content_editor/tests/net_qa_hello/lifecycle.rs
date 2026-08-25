use bevy::app::App;
use gdtf_battle_sim::{armor::ArmorRegistry, weapon::WeaponRegistry};
use gdtf_content_editor::{ArmorDraft, EditorQaAssetsRoot, WeaponDraft};
use tempfile::TempDir;

use crate::{
    client::{EDITOR_LOAD, EDITOR_NEW, EDITOR_SAVE},
    harness::editing_app_and_client,
    outcome::ran_body,
    rows::{
        LoadOutcomeRow, LoadReplyRow, NewOutcomeRow, NewReplyRow, SaveOutcomeRow, SaveReplyRow,
    },
    socket::{Client, run_editor},
    support::TestError,
};

/// A key the live registry actually holds, so no test pins a content file name.
pub(crate) fn first_armor_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<ArmorRegistry>() else {
        return Err("the editor reached Editing, so its armor registry is loaded".into());
    };
    let mut keys: Vec<String> = registry.keys().map(|key| key.as_str().to_owned()).collect();
    keys.sort();
    let Some(first) = keys.first() else {
        return Err("the loaded armor registry is empty, so no key can be loaded".into());
    };
    Ok(first.clone())
}

/// A key the live weapon registry actually holds.
pub(crate) fn first_weapon_key(app: &App) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<WeaponRegistry>() else {
        return Err("the editor reached Editing, so its weapon registry is loaded".into());
    };
    let mut keys: Vec<String> = registry.keys().map(|key| key.as_str().to_owned()).collect();
    keys.sort();
    let Some(first) = keys.first() else {
        return Err("the loaded weapon registry is empty, so no key can be loaded".into());
    };
    Ok(first.clone())
}

/// The armor draft the world holds right now.
pub(crate) fn armor_draft(app: &App) -> Result<ArmorDraft, TestError> {
    let Some(draft) = app.world().get_resource::<ArmorDraft>() else {
        return Err("the armor draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// The weapon draft the world holds right now.
pub(crate) fn weapon_draft(app: &App) -> Result<WeaponDraft, TestError> {
    let Some(draft) = app.world().get_resource::<WeaponDraft>() else {
        return Err("the weapon draft is a resource the editor creates on entering Editing".into());
    };
    Ok(draft.clone())
}

/// Run `editor.new` for a mode and hand back what it answered.
pub(crate) fn new_mode(
    app: &mut App,
    client: &mut Client,
    mode: &str,
) -> Result<NewOutcomeRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_NEW, &format!("(mode: {mode})")))?;
    let body: NewReplyRow = ran_body(&reply, EDITOR_NEW)?;
    Ok(body.outcome)
}

/// Run `editor.load` and hand back what it answered.
pub(crate) fn load_by_key(
    app: &mut App,
    client: &mut Client,
    mode: &str,
    key: &str,
) -> Result<LoadOutcomeRow, TestError> {
    let arguments = format!("(mode: {mode}, key: \"{key}\")");
    let reply = client.exchange(app, &run_editor(EDITOR_LOAD, &arguments))?;
    let body: LoadReplyRow = ran_body(&reply, EDITOR_LOAD)?;
    Ok(body.outcome)
}

/// An editing app whose QA saves land in a temp root, with an armor draft loaded from content.
pub(crate) struct ArmorSaveCase {
    pub(crate) app:    App,
    pub(crate) client: Client,
    pub(crate) root:   TempDir,
}

/// Stand up the armor save case: load a real armor, then point QA saves at a temp root.
pub(crate) fn armor_save_case() -> Result<ArmorSaveCase, TestError> {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_armor_key(&app)?;
    let loaded = load_by_key(&mut app, &mut client, "Armor", &key)?;
    let LoadOutcomeRow::Loaded { .. } = loaded else {
        return Err(format!("`{key}` came from the live registry, got {loaded:?}").into());
    };
    let root = TempDir::new()?;
    app.world_mut()
        .insert_resource(EditorQaAssetsRoot::new(root.path().to_path_buf()));
    Ok(ArmorSaveCase { app, client, root })
}

/// Run `editor.save` with no name argument at all.
pub(crate) fn save_mode(
    app: &mut App,
    client: &mut Client,
    mode: &str,
) -> Result<SaveOutcomeRow, TestError> {
    let reply = client.exchange(app, &run_editor(EDITOR_SAVE, &format!("(mode: {mode})")))?;
    let body: SaveReplyRow = ran_body(&reply, EDITOR_SAVE)?;
    Ok(body.outcome)
}

/// The path a successful save reported, or why the reply was not a written file.
pub(crate) fn written_path(outcome: SaveOutcomeRow) -> Result<String, TestError> {
    match outcome {
        SaveOutcomeRow::Wrote { path } => Ok(path),
        other => Err(format!("expected a written file, got {other:?}").into()),
    }
}
