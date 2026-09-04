use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    terrain::def::{
        LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
        TerrainSimKind, TerrainUuid, TerrainViews,
    },
};

use crate::{
    bad_arguments::bad_arguments_detail,
    harness::editing_app_and_client,
    load_case::reply_answered_during_load,
    names::EDITOR_FAMILIES,
    outcome::{ran_body, unavailable_code},
    rows::{FamiliesReplyRow, FamilyEntryRow, FamilyRow, FamilyRowEntries},
    socket::run_editor,
    support::TestResult,
};

// Every family the wire enum names, so the count is the enum's own and not a content count.
const EVERY_FAMILY: [FamilyRow; 10] = [
    FamilyRow::Terrain,
    FamilyRow::Theme,
    FamilyRow::Gang,
    FamilyRow::Armor,
    FamilyRow::Injury,
    FamilyRow::Sprite,
    FamilyRow::Attachment,
    FamilyRow::Weapon,
    FamilyRow::MeleeWeapon,
    FamilyRow::Field,
];

fn keys(row: &FamilyRowEntries) -> Vec<String> {
    row.entries
        .iter()
        .map(|entry: &FamilyEntryRow| entry.key.clone())
        .collect()
}

#[test]
fn a_read_without_a_filter_answers_every_family_exactly_once() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    let answered: Vec<FamilyRow> = body.families.iter().map(|row| row.family).collect();
    assert_eq!(
        answered,
        EVERY_FAMILY.to_vec(),
        "an unfiltered read answers every family the wire enum names, once each and in its own \
         order, so a client can walk the reply without checking for gaps",
    );
    Ok(())
}

#[test]
fn a_filter_narrows_the_reply_to_that_family_alone_and_answers_the_same_entries() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let filtered_reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_FAMILIES, "(family: Some(Armor))"),
    )?;
    let filtered: FamiliesReplyRow = ran_body(&filtered_reply, EDITOR_FAMILIES)?;
    let whole_reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let whole: FamiliesReplyRow = ran_body(&whole_reply, EDITOR_FAMILIES)?;

    let answered: Vec<FamilyRow> = filtered.families.iter().map(|row| row.family).collect();
    assert_eq!(
        answered,
        vec![FamilyRow::Armor],
        "a filtered read answers the named family and nothing else. A client asking for one \
         family must not have to sift eight others out",
    );
    let unfiltered_armor: Vec<&FamilyRowEntries> = whole
        .families
        .iter()
        .filter(|row| row.family == FamilyRow::Armor)
        .collect();
    assert_eq!(
        filtered.families.iter().collect::<Vec<_>>(),
        unfiltered_armor,
        "the filter chooses which rows answer and nothing else, so the Armor row reads the same \
         either way",
    );
    assert!(
        filtered.families.iter().any(|row| !row.entries.is_empty()),
        "the Armor registry answered no keys, so this case could not tell a filter that keeps \
         the entries from one that drops them: {filtered:?}",
    );
    Ok(())
}

#[test]
fn the_prefab_tab_is_refused_because_it_carries_no_content_family() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_FAMILIES, "(family: Some(Prefab))"),
    )?;

    let detail = bad_arguments_detail(&reply)?;
    assert!(
        detail.contains("Prefab"),
        "the mode vocabulary spells Prefab, so the refusal names that tab and says it carries no \
         content family, got `{detail}`",
    );
    Ok(())
}

#[test]
fn every_family_comes_back_sorted_by_key() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    let mut compared = 0_usize;
    for row in &body.families {
        let answered = keys(row);
        let mut expected = answered.clone();
        expected.sort();
        assert_eq!(
            answered, expected,
            "{:?} came back out of key order, and every registry is a HashMap underneath, so an \
             unsorted reply differs between two runs of the same editor",
            row.family,
        );
        compared += usize::from(answered.len() > 1);
    }
    assert!(
        compared > 0,
        "no family answered more than one key, so nothing here could tell a sorted reply from \
         an unsorted one. The loaded content is what makes this case discriminate: {body:?}",
    );
    Ok(())
}

#[test]
fn every_entry_carries_a_key_and_a_label() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    let reply = client.exchange(&mut app, &run_editor(EDITOR_FAMILIES, "()"))?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    for row in &body.families {
        for entry in &row.entries {
            assert!(
                !entry.key.is_empty() && !entry.label.is_empty(),
                "{:?} answered an entry a picker cannot show or a load cannot use: {entry:?}",
                row.family,
            );
        }
    }
    Ok(())
}

// The display name both fixture defs hold, so a label built from the name alone reads twice.
const SHARED_NAME: &str = "Waste Drum";

fn cover_def(low: u128) -> TerrainDef {
    TerrainDef {
        key:            TerrainUuid::new(Uuid::from_u128(low)),
        display_name:   TerrainDisplayName::new(SHARED_NAME.to_owned()),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(3),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover,
        views:          TerrainViews::new(Vec::new()),
        tags:           Vec::new(),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

#[test]
fn two_terrain_defs_under_one_display_name_answer_labels_that_differ() -> TestResult {
    let (mut app, mut client) = editing_app_and_client()?;
    // `resolve_content_family` is gated on the registry being absent, so this replacement sticks.
    app.world_mut().insert_resource(TerrainDefRegistry::new(
        [0x0184_0bcd_8001, 0x0184_0bcd_8002].into_iter().map(|low| {
            let def = cover_def(low);
            (def.key, def)
        }),
    ));

    let reply = client.exchange(
        &mut app,
        &run_editor(EDITOR_FAMILIES, "(family: Some(Terrain))"),
    )?;
    let body: FamiliesReplyRow = ran_body(&reply, EDITOR_FAMILIES)?;

    let Some(row) = body.families.first() else {
        unreachable!("a filtered read answers the Terrain row: {body:?}");
    };
    let [first, second] = row.entries.as_slice() else {
        unreachable!("the fixture registry holds two defs: {row:?}");
    };
    assert_ne!(
        first.label, second.label,
        "two defs sharing a display name must answer two labels an author can tell apart, or the \
         Terrain picker draws one row twice: {row:?}",
    );
    Ok(())
}

#[test]
fn families_is_refused_while_the_editor_is_still_loading() -> TestResult {
    let reply =
        reply_answered_during_load(run_editor(EDITOR_FAMILIES, "()"), "the editor.families run")?;
    assert_eq!(
        unavailable_code(&reply)?,
        "WrongState",
        "the editor only holds every registry once it enters Editing, so a Load-pass call is \
         refused rather than answering a half-loaded list",
    );
    Ok(())
}
