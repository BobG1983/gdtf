use std::path::{Path, PathBuf};

use bevy::app::App;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables},
    level::PrefabRegistry,
    severity::Severity,
    terrain::def::TerrainDefRegistry,
};
use gdtf_content_families::injuries::weighting_member_key;
use gdtf_editor::{EditorMcpAssetsRoot, weighting_save_path_in};

use crate::{
    bad_arguments::bad_arguments_detail,
    fixture_root::{
        DELETED_PIECE, FIXTURE_PREFAB, FLOOR_THEME, REPLACEMENT_PIECE, prefab_member_key,
        write_fixture_injury, write_fixture_prefab, write_fixture_weighting, write_terrain_def,
        write_theme_def,
    },
    mirror::ModeRow,
    names::{EDITOR_DELETE_RECORD, EDITOR_SELECT_INJURY_TAB, EDITOR_SET_MODE},
    outcome::{ran_body, unavailable_code},
    rows::{DeleteOutcomeRow, DeleteRefusalRow},
    setup::editing_app_and_client_on,
    socket::{Client, run_editor},
    support::{TestError, TestResult},
};

// The weighting table both weighting cases name.
const CATEGORY: InjuryCategory = InjuryCategory::Head;

// The damage context half of that table's key.
const CONTEXT: DamageContext = DamageContext::Ranged;

// The whole args body `editor.delete_record` takes.
fn delete_args(family: &str, key: &str) -> String {
    format!("(family: \"{family}\", key: \"{key}\")")
}

// The same call, naming the replacement every kept reference is pointed at.
fn replace_args(family: &str, key: &str, replacement: &str) -> String {
    format!("(family: \"{family}\", key: \"{key}\", replacement: Some(\"{replacement}\"))")
}

// Open a top-level tab, so a case drives the delete from the screen that offers it.
fn open_tab(app: &mut App, client: &mut Client, mode: ModeRow) -> Result<(), TestError> {
    let args = format!("(mode: {mode:?})");
    client.exchange(app, &run_editor(EDITOR_SET_MODE, &args))?;
    app.update();
    Ok(())
}

// A temp assets root every case in this file boots the editor on.
fn temp_root() -> Result<tempfile::TempDir, TestError> {
    Ok(tempfile::tempdir()?)
}

// The prefab registry, which the editor builds once its map folder resolves.
fn prefab_registry(app: &App) -> Result<&PrefabRegistry, TestError> {
    let Some(registry) = app.world().get_resource::<PrefabRegistry>() else {
        return Err("PrefabRegistry is a resource the editor builds during its load pass".into());
    };
    Ok(registry)
}

// The injury tables, which the editor builds from its injuries folder.
fn injury_tables(app: &App) -> Result<&InjuryTables, TestError> {
    let Some(tables) = app.world().get_resource::<InjuryTables>() else {
        return Err("InjuryTables is a resource the editor builds during its load pass".into());
    };
    Ok(tables)
}

// Point every QA-driven write and delete at the temp root the app booted on.
fn aim_saves_at(app: &mut App, root: &Path) {
    app.insert_resource(EditorMcpAssetsRoot::new(root.to_path_buf()));
}

#[test]
fn deleting_a_prefab_over_the_wire_takes_it_out_of_the_registry_and_removes_its_file() -> TestResult
{
    let root = temp_root()?;
    let Some(file): Option<PathBuf> = write_fixture_prefab(root.path()) else {
        return Err("the fixture prefab write must succeed under the temp assets root".into());
    };

    let (mut app, mut client) = editing_app_and_client_on(root.path())?;
    aim_saves_at(&mut app, root.path());
    open_tab(&mut app, &mut client, ModeRow::Prefab)?;
    if !prefab_registry(&app)?
        .iter()
        .any(|prefab| **prefab.name() == *FIXTURE_PREFAB)
    {
        return Err("the temp root's prefab must be in the registry before the delete runs".into());
    }

    let args = delete_args("PrefabRegistry", &prefab_member_key());
    let reply = client.exchange(&mut app, &run_editor(EDITOR_DELETE_RECORD, &args))?;
    let body: DeleteOutcomeRow = ran_body(&reply, EDITOR_DELETE_RECORD)?;
    app.update();

    assert_eq!(
        body,
        DeleteOutcomeRow::Removed,
        "nothing names a prefab, so the in-use check clears and the delete goes through",
    );
    assert!(
        !prefab_registry(&app)?
            .iter()
            .any(|prefab| **prefab.name() == *FIXTURE_PREFAB),
        "the deleted prefab must stay out of PrefabRegistry",
    );
    assert!(
        !file.exists(),
        "the deleted prefab's file must be gone — otherwise the next folder reload brings it \
         back",
    );
    Ok(())
}

#[test]
fn a_family_the_delete_registry_holds_no_entry_for_is_refused_with_no_entry() -> TestResult {
    let root = temp_root()?;
    let (mut app, mut client) = editing_app_and_client_on(root.path())?;
    aim_saves_at(&mut app, root.path());

    let args = delete_args("SpriteDefRegistry", "fixture_sprite");
    let reply = client.exchange(&mut app, &run_editor(EDITOR_DELETE_RECORD, &args))?;
    let body: DeleteOutcomeRow = ran_body(&reply, EDITOR_DELETE_RECORD)?;

    assert_eq!(
        body,
        DeleteOutcomeRow::Refused(DeleteRefusalRow::NoEntry),
        "a family this build cannot delete is answered by the delete driver itself, not by an \
         availability refusal and not by BadArguments",
    );
    Ok(())
}

#[test]
fn the_weighting_delete_is_refused_from_the_injury_def_sub_tab_and_touches_nothing() -> TestResult {
    let root = temp_root()?;
    if !write_fixture_injury(root.path(), CATEGORY) {
        return Err("the fixture injury def write must succeed".into());
    }
    if !write_fixture_weighting(root.path(), CATEGORY, CONTEXT) {
        return Err("the fixture weighting write must succeed".into());
    }
    let file = weighting_save_path_in(root.path(), CATEGORY, CONTEXT);

    let (mut app, mut client) = editing_app_and_client_on(root.path())?;
    aim_saves_at(&mut app, root.path());
    open_tab(&mut app, &mut client, ModeRow::Injury)?;
    client.exchange(
        &mut app,
        &run_editor(EDITOR_SELECT_INJURY_TAB, "(tab: Def)"),
    )?;
    app.update();

    let key = weighting_member_key(CATEGORY, CONTEXT);
    let args = delete_args("InjuryTables", &key);
    let reply = client.exchange(&mut app, &run_editor(EDITOR_DELETE_RECORD, &args))?;

    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "a record is deleted from the screen that shows it, and the weighting table's screen \
         is the Injury tab's Tables sub-tab",
    );
    app.update();
    assert!(
        injury_tables(&app)?
            .table_for_category(CATEGORY, CONTEXT, Severity::Minor)
            .is_some(),
        "a refused delete must leave the table in InjuryTables",
    );
    assert!(
        file.exists(),
        "a refused delete must leave the table's file on disk",
    );
    Ok(())
}

// A temp root holding two terrain defs and a theme whose default floor names the first.
fn terrain_root() -> Result<(tempfile::TempDir, PathBuf, PathBuf), TestError> {
    let root = temp_root()?;
    let Some(deleted) = write_terrain_def(root.path(), "deleted_piece", DELETED_PIECE) else {
        return Err("the deleted terrain def write must succeed".into());
    };
    if write_terrain_def(root.path(), "replacement_piece", REPLACEMENT_PIECE).is_none() {
        return Err("the replacement terrain def write must succeed".into());
    }
    let Some(theme) = write_theme_def(
        root.path(),
        "floor_theme",
        FLOOR_THEME,
        DELETED_PIECE,
        REPLACEMENT_PIECE,
    ) else {
        return Err("the theme def write must succeed".into());
    };
    Ok((root, deleted, theme))
}

// The terrain registry, which the editor builds from its terrain folder.
fn terrain_registry(app: &App) -> Result<&TerrainDefRegistry, TestError> {
    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        return Err(
            "TerrainDefRegistry is a resource the editor builds during its load pass".into(),
        );
    };
    Ok(registry)
}

// Whether the terrain registry still holds the piece the delete cases name.
fn holds_deleted_piece(app: &App) -> Result<bool, TestError> {
    let held = terrain_registry(app)?
        .defs()
        .any(|(key, _def)| (**key).to_string() == DELETED_PIECE);
    Ok(held)
}

#[test]
fn naming_a_replacement_rewrites_the_referrer_and_removes_the_record() -> TestResult {
    let (root, deleted, theme) = terrain_root()?;
    let (mut app, mut client) = editing_app_and_client_on(root.path())?;
    aim_saves_at(&mut app, root.path());
    open_tab(&mut app, &mut client, ModeRow::Terrain)?;

    let args = replace_args("TerrainDefRegistry", DELETED_PIECE, REPLACEMENT_PIECE);
    let reply = client.exchange(&mut app, &run_editor(EDITOR_DELETE_RECORD, &args))?;
    let body: DeleteOutcomeRow = ran_body(&reply, EDITOR_DELETE_RECORD)?;
    app.update();

    assert_eq!(
        body,
        DeleteOutcomeRow::Removed,
        "naming a replacement confirms the delete, so every kept reference is rewritten and the \
         record goes",
    );
    let rewritten = std::fs::read_to_string(&theme).unwrap_or_default();
    assert!(
        rewritten.contains(REPLACEMENT_PIECE) && !rewritten.contains(DELETED_PIECE),
        "the theme's default floor must name the replacement: {rewritten}",
    );
    assert!(!deleted.exists(), "the deleted def's own file must be gone");
    Ok(())
}

#[test]
fn omitting_the_replacement_answers_the_refusal_with_the_referring_record() -> TestResult {
    let (root, deleted, theme) = terrain_root()?;
    let before = std::fs::read_to_string(&theme).unwrap_or_default();
    let (mut app, mut client) = editing_app_and_client_on(root.path())?;
    aim_saves_at(&mut app, root.path());
    open_tab(&mut app, &mut client, ModeRow::Terrain)?;

    let args = delete_args("TerrainDefRegistry", DELETED_PIECE);
    let reply = client.exchange(&mut app, &run_editor(EDITOR_DELETE_RECORD, &args))?;
    let body: DeleteOutcomeRow = ran_body(&reply, EDITOR_DELETE_RECORD)?;
    app.update();

    let named = match body {
        DeleteOutcomeRow::Refused(DeleteRefusalRow::InUse(ref records)) => records.clone(),
        _ => Vec::new(),
    };
    assert!(
        named.iter().any(|record| record.key == FLOOR_THEME),
        "a record with a required referrer and no replacement named comes back refused, naming \
         the record that still holds the reference; got {body:?}",
    );
    assert_eq!(
        std::fs::read_to_string(&theme).unwrap_or_default(),
        before,
        "a refused delete writes no referrer",
    );
    assert!(
        deleted.exists(),
        "a refused delete leaves the record's own file on disk",
    );
    Ok(())
}

#[test]
fn cancel_answers_cancelled_with_nothing_written_and_nothing_removed() -> TestResult {
    let (root, deleted, theme) = terrain_root()?;
    let before = std::fs::read(&theme).unwrap_or_default();
    let (mut app, mut client) = editing_app_and_client_on(root.path())?;
    aim_saves_at(&mut app, root.path());
    open_tab(&mut app, &mut client, ModeRow::Terrain)?;

    let args = format!("(family: \"TerrainDefRegistry\", key: \"{DELETED_PIECE}\", cancel: true)");
    let reply = client.exchange(&mut app, &run_editor(EDITOR_DELETE_RECORD, &args))?;
    let body: DeleteOutcomeRow = ran_body(&reply, EDITOR_DELETE_RECORD)?;
    app.update();

    assert_eq!(
        body,
        DeleteOutcomeRow::Cancelled,
        "cancel calls the delete off, and that is a different answer from a refusal",
    );
    assert_eq!(
        std::fs::read(&theme).unwrap_or_default(),
        before,
        "a cancelled delete leaves the referring file byte-unchanged",
    );
    assert!(
        deleted.exists(),
        "a cancelled delete leaves the record's own file on disk",
    );
    assert!(
        holds_deleted_piece(&app)?,
        "a cancelled delete leaves the record in its registry",
    );
    Ok(())
}

#[test]
fn cancel_together_with_a_replacement_key_is_refused_as_bad_arguments() -> TestResult {
    let (root, _deleted, _theme) = terrain_root()?;
    let (mut app, mut client) = editing_app_and_client_on(root.path())?;
    aim_saves_at(&mut app, root.path());
    open_tab(&mut app, &mut client, ModeRow::Terrain)?;

    let args = format!(
        "(family: \"TerrainDefRegistry\", key: \"{DELETED_PIECE}\", replacement: \
         Some(\"{REPLACEMENT_PIECE}\"), cancel: true)"
    );
    let reply = client.exchange(&mut app, &run_editor(EDITOR_DELETE_RECORD, &args))?;
    let detail = bad_arguments_detail(&reply)?;

    assert!(
        detail.contains("replacement") && detail.contains("cancel"),
        "one call carries one meaning, so the refusal names both arguments: {detail}",
    );
    Ok(())
}
