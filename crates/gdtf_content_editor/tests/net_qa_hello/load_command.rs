use crate::{
    harness::editing_app_and_client,
    lifecycle::{first_weapon_key, load_by_key, weapon_draft},
    rows::{LoadOutcomeRow, RefusalRow},
    support::TestResult,
};

const ABSENT_KEY: &str = "no_such_weapon_in_any_content_pack";

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
