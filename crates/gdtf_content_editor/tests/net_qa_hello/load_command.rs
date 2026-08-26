use gdtf_assets::ContentFolderHandle;
use gdtf_battle_sim::effects::fields::FieldDefRegistry;
use gdtf_content_families::FieldsFamily;

use crate::{
    client::EDITOR_LOAD,
    harness::editing_app_and_client,
    lifecycle::{field_draft, first_field_key, first_weapon_key, load_by_key, weapon_draft},
    outcome::unavailable_code,
    rows::{LoadOutcomeRow, RefusalRow},
    socket::run_editor,
    support::TestResult,
};

const ABSENT_KEY: &str = "no_such_weapon_in_any_content_pack";

const ABSENT_FIELD_KEY: &str = "no_such_field_in_any_content_pack";

/// The two modes this build loads no draft for.
const NO_LOAD_BY_KEY: [&str; 2] = ["Terrain", "Prefab"];

#[test]
fn load_fills_the_weapon_draft_from_the_live_registry() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_weapon_key(&app)?;

    let outcome = load_by_key(&mut app, &mut client, "Weapon", &key)?;

    let LoadOutcomeRow::Loaded { key: echoed } = outcome else {
        unreachable!("`{key}` came from the live registry, got {outcome:?}");
    };
    assert_eq!(echoed, key, "the reply echoes the key that was loaded");
    assert_eq!(
        weapon_draft(&app)?.name(),
        key,
        "the world's own weapon draft carries the loaded weapon — this is the draft the form's \
         load combo fills, through the same `load_weapon` method",
    );
    Ok(())
}

#[test]
fn load_fills_the_field_draft_from_the_live_registry_and_a_miss_lists_the_keys() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let key = first_field_key(&app)?;

    let outcome = load_by_key(&mut app, &mut client, "Field", &key)?;
    let LoadOutcomeRow::Loaded { key: echoed } = outcome else {
        unreachable!("`{key}` came from the live registry, got {outcome:?}");
    };
    assert_eq!(echoed, key, "the reply echoes the key that was loaded");

    let draft = field_draft(&app)?;
    assert_eq!(
        draft.key(),
        key,
        "the world's own field draft carries the loaded field. This is the draft the form's \
         load combo fills, through the same `load_field` method",
    );
    assert!(
        !draft.autoload_pending(),
        "a loaded draft is settled, so the form's own sync cannot seed over it",
    );

    let before = field_draft(&app)?;
    let missed = load_by_key(&mut app, &mut client, "Field", ABSENT_FIELD_KEY)?;
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

#[test]
fn load_refuses_terrain_and_prefab_rather_than_reporting_a_missing_model() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    for mode in NO_LOAD_BY_KEY {
        let outcome = load_by_key(&mut app, &mut client, mode, ABSENT_KEY)?;
        assert_eq!(
            outcome,
            LoadOutcomeRow::Refused(RefusalRow::NoLoadAction),
            "{mode} loads no draft by key in this build, and that is a typed outcome inside a Ran \
             reply — Unavailable is reserved for host state, so a client reading this cannot \
             mistake it for a draft the editor lost",
        );
    }
    Ok(())
}

#[test]
fn a_key_the_registry_does_not_hold_leaves_the_draft_alone() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let before = weapon_draft(&app)?;

    let outcome = load_by_key(&mut app, &mut client, "Weapon", ABSENT_KEY)?;

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
    let before = field_draft(&app)?;
    // The folder handle goes too, or the family's resolve system rebuilds the registry next frame.
    app.world_mut()
        .remove_resource::<ContentFolderHandle<FieldsFamily>>();
    app.world_mut().remove_resource::<FieldDefRegistry>();

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_LOAD, &format!("(mode: Field, key: \"{key}\")")),
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
