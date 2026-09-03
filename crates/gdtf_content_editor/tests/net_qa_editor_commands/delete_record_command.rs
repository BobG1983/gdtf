use std::path::{Path, PathBuf};

use bevy::app::App;
use gdtf_battle_sim::{
    armor::InjuryCategory,
    injuries::{DamageContext, InjuryTables},
    level::PrefabRegistry,
    severity::Severity,
};
use gdtf_content_editor::{EditorQaAssetsRoot, weighting_save_path_in};
use gdtf_content_families::injuries::weighting_member_key;

use crate::{
    fixture_root::{
        FIXTURE_PREFAB, prefab_member_key, write_fixture_injury, write_fixture_prefab,
        write_fixture_weighting,
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
    app.insert_resource(EditorQaAssetsRoot::new(root.to_path_buf()));
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

    let args = delete_args("WeaponRegistry", "fixture_gun");
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
