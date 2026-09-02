use gdtf_assets::ContentFolderHandle;
use gdtf_battle_sim::{effects::fields::FieldDefRegistry, terrain::def::TerrainDefRegistry};
use gdtf_content_families::FieldsFamily;

use crate::{
    client::{
        EDITOR_LOAD_ARMOR, EDITOR_LOAD_ATTACHMENT, EDITOR_LOAD_FIELD, EDITOR_LOAD_GANG,
        EDITOR_LOAD_INJURY, EDITOR_LOAD_MELEE_WEAPON, EDITOR_LOAD_SPRITE, EDITOR_LOAD_TERRAIN,
        EDITOR_LOAD_THEME, EDITOR_LOAD_WEAPON,
    },
    drafts::{
        attachment_draft, gang_draft, injury_draft, melee_weapon_draft, sprite_draft,
        terrain_draft, theme_draft,
    },
    harness::editing_app_and_client,
    keys::{
        first_attachment_key, first_gang_key, first_injury_key, first_melee_weapon_key,
        first_sprite_key, first_terrain_key, first_theme_key,
    },
    lifecycle::{
        armor_draft, field_draft, first_armor_key, first_field_key, first_weapon_key, load_by_key,
        open_tab, weapon_draft,
    },
    outcome::unavailable_code,
    rows::LoadOutcomeRow,
    socket::run_editor,
    support::{TestError, TestResult},
};

const ABSENT_KEY: &str = "no_such_weapon_in_any_content_pack";

const ABSENT_FIELD_KEY: &str = "no_such_field_in_any_content_pack";

const ABSENT_TERRAIN_KEY: &str = "no_such_terrain_in_any_content_pack";

// The key a load echoed back, or why the reply was not a load at all.
fn echoed_key(outcome: LoadOutcomeRow) -> Result<String, TestError> {
    match outcome {
        LoadOutcomeRow::Loaded { key } => Ok(key),
        LoadOutcomeRow::NoSuchKey { key, known } => Err(format!(
            "the key came from the live registry, so the load must answer Loaded. It answered a \
             miss on `{key}`, against the registry's own {known:?}"
        )
        .into()),
    }
}

#[test]
fn load_theme_fills_the_theme_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_theme_key(&app)?;
    open_tab(&mut app, &mut client, "Theme")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_THEME, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        (*theme_draft(&app)?.key()).to_string(),
        key,
        "the world's own theme draft carries the loaded theme, through the same `load_theme` the \
         form's own picker calls",
    );
    Ok(())
}

#[test]
fn load_gang_fills_the_gang_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_gang_key(&app)?;
    open_tab(&mut app, &mut client, "Gang")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_GANG, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        gang_draft(&app)?.name(),
        key,
        "the world's own gang draft carries the loaded gang, through the same `load_gang` method",
    );
    Ok(())
}

#[test]
fn load_armor_fills_the_armor_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_armor_key(&app)?;
    open_tab(&mut app, &mut client, "Armor")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_ARMOR, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        armor_draft(&app)?.name(),
        key,
        "the world's own armor draft carries the loaded armor, through the same `load_armor` \
         method",
    );
    Ok(())
}

#[test]
fn load_injury_fills_the_injury_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_injury_key(&app)?;
    open_tab(&mut app, &mut client, "Injury")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_INJURY, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        injury_draft(&app)?.key(),
        key,
        "the world's own injury draft carries the loaded injury, through the same `load_injury` \
         method",
    );
    Ok(())
}

#[test]
fn load_sprite_fills_the_sprite_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_sprite_key(&app)?;
    open_tab(&mut app, &mut client, "Sprite")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_SPRITE, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        sprite_draft(&app)?.name(),
        key,
        "the world's own sprite draft carries the loaded sprite, through the same `load_sprite` \
         method",
    );
    Ok(())
}

#[test]
fn load_attachment_fills_the_attachment_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_attachment_key(&app)?;
    open_tab(&mut app, &mut client, "Attachment")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_ATTACHMENT, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        attachment_draft(&app)?.name(),
        key,
        "the world's own attachment draft carries the loaded attachment, through the same \
         `load_attachment` method",
    );
    Ok(())
}

#[test]
fn load_weapon_fills_the_weapon_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_weapon_key(&app)?;
    open_tab(&mut app, &mut client, "Weapon")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_WEAPON, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        weapon_draft(&app)?.name(),
        key,
        "the world's own weapon draft carries the loaded weapon — this is the draft the form's \
         load combo fills, through the same `load_weapon` method",
    );
    Ok(())
}

#[test]
fn load_melee_weapon_fills_the_melee_weapon_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_melee_weapon_key(&app)?;
    open_tab(&mut app, &mut client, "MeleeWeapon")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_MELEE_WEAPON, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    assert_eq!(
        melee_weapon_draft(&app)?.name(),
        key,
        "the world's own melee weapon draft carries the loaded weapon, through the same \
         `load_melee_weapon` method",
    );
    Ok(())
}

#[test]
fn load_field_fills_the_field_draft_from_the_live_registry_and_a_miss_lists_the_keys() -> TestResult
{
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_field_key(&app)?;
    open_tab(&mut app, &mut client, "Field")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_FIELD, &key)?;
    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");

    let draft = field_draft(&app)?;
    assert_eq!(
        draft.key(),
        key,
        "the world's own field draft carries the loaded field. This is the draft the form's load \
         combo fills, through the same `load_field` method",
    );
    assert!(
        !draft.autoload_pending(),
        "a loaded draft is settled, so the form's own sync cannot seed over it",
    );

    let before = field_draft(&app)?;
    let missed = load_by_key(&mut app, &mut client, EDITOR_LOAD_FIELD, ABSENT_FIELD_KEY)?;
    let LoadOutcomeRow::NoSuchKey {
        key: asked,
        mut known,
    } = missed
    else {
        unreachable!("`{ABSENT_FIELD_KEY}` is in no content pack, got {missed:?}");
    };
    assert_eq!(asked, ABSENT_FIELD_KEY);
    assert!(
        !known.is_empty() && !known.contains(&ABSENT_FIELD_KEY.to_owned()),
        "the miss lists the registry's own keys, and the asked-for key is not one: {known:?}",
    );
    let listed = known.clone();
    known.sort();
    assert_eq!(
        listed, known,
        "the known list comes back sorted, so a client reading a miss sees the same order every \
         time",
    );
    assert_eq!(
        field_draft(&app)?,
        before,
        "a miss writes nothing, so the draft still equals the one taken before the call",
    );
    Ok(())
}

// The display name the live registry holds under one rendered key.
fn terrain_display_name(app: &bevy::app::App, key: &str) -> Result<String, TestError> {
    let Some(registry) = app.world().get_resource::<TerrainDefRegistry>() else {
        return Err("the editor reached Editing, so its terrain registry is loaded".into());
    };
    let found = registry
        .defs()
        .find_map(|(uuid, def)| ((**uuid).to_string() == key).then(|| (*def.display_name).clone()));
    let Some(name) = found else {
        return Err(format!("`{key}` came from this registry, so it still holds that def").into());
    };
    Ok(name)
}

#[test]
fn load_terrain_fills_the_terrain_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_terrain_key(&app)?;
    let name = terrain_display_name(&app, &key)?;
    open_tab(&mut app, &mut client, "Terrain")?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_TERRAIN, &key)?;

    assert_eq!(echoed_key(outcome)?, key, "the reply echoes the loaded key");
    let draft = terrain_draft(&app)?;
    assert_eq!(
        draft.uuid().map(|uuid| (*uuid).to_string()),
        Some(key.clone()),
        "the world's own terrain draft carries the loaded def's authored key, so the next save \
         writes that record instead of minting a new one",
    );
    assert_eq!(
        draft.display_name(),
        name,
        "the draft carries the loaded def's display name, through the same `load_from_def` the \
         form's own picker calls",
    );
    Ok(())
}

#[test]
fn a_terrain_key_that_is_not_uuid_text_lists_the_keys_and_leaves_the_draft_alone() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, "Terrain")?;
    let before = terrain_draft(&app)?;

    let missed = load_by_key(
        &mut app,
        &mut client,
        EDITOR_LOAD_TERRAIN,
        ABSENT_TERRAIN_KEY,
    )?;

    let LoadOutcomeRow::NoSuchKey {
        key: asked,
        mut known,
    } = missed
    else {
        unreachable!(
            "`{ABSENT_TERRAIN_KEY}` is not UUID text, so it misses at the parse: {missed:?}"
        );
    };
    assert_eq!(asked, ABSENT_TERRAIN_KEY);
    assert!(
        !known.is_empty() && !known.contains(&ABSENT_TERRAIN_KEY.to_owned()),
        "the miss lists the registry's own keys, and the asked-for key is not one: {known:?}",
    );
    let listed = known.clone();
    known.sort();
    assert_eq!(
        listed, known,
        "the known list comes back sorted, so a client reading a miss sees the same order every \
         time",
    );
    assert_eq!(
        terrain_draft(&app)?,
        before,
        "a miss writes nothing, so the draft still equals the one taken before the call",
    );
    Ok(())
}

#[test]
fn a_key_the_registry_does_not_hold_leaves_the_draft_alone() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    open_tab(&mut app, &mut client, "Weapon")?;
    let before = weapon_draft(&app)?;

    let outcome = load_by_key(&mut app, &mut client, EDITOR_LOAD_WEAPON, ABSENT_KEY)?;

    let LoadOutcomeRow::NoSuchKey { key, known } = outcome else {
        unreachable!("`{ABSENT_KEY}` is in no content pack, got {outcome:?}");
    };
    assert_eq!(key, ABSENT_KEY, "the miss names the key that was asked for");
    assert!(
        !known.is_empty(),
        "the miss lists every key the registry does hold, so one round trip fixes the call",
    );
    assert!(
        !known.contains(&ABSENT_KEY.to_owned()),
        "the known list is the registry's own keys, and the asked-for key is not one of them: \
         {known:?}",
    );
    assert_eq!(
        weapon_draft(&app)?,
        before,
        "a miss writes nothing, so the draft still equals the one taken before the call",
    );
    Ok(())
}

#[test]
fn a_field_load_with_no_registry_in_the_world_reports_a_missing_model() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_field_key(&app)?;
    open_tab(&mut app, &mut client, "Field")?;
    let before = field_draft(&app)?;
    // The folder handle goes too, or the family's resolve system rebuilds the registry next frame.
    app.world_mut()
        .remove_resource::<ContentFolderHandle<FieldsFamily>>();
    app.world_mut().remove_resource::<FieldDefRegistry>();

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_LOAD_FIELD, &format!("(key: \"{key}\")")),
    )?;

    assert_eq!(
        unavailable_code(&reply)?,
        "MissingModel",
        "the field registry the load reads is gone, so the host says its model is missing. A \
         `NoSuchKey` answer here would tell a client the key was wrong when the registry was \
         never consulted",
    );
    assert_eq!(
        field_draft(&app)?,
        before,
        "a refused load writes nothing, so the draft still equals the one taken before the call",
    );
    Ok(())
}
