use gdtf_battle_sim::{armor::InjuryCategory, injuries::DamageContext};
use gdtf_content_editor::WeightingDraft;

use crate::{
    rows::{BucketRow, CategoryRow, ContextRow},
    setup::{
        injury_tab_app_and_client, injury_tables, list_op, live_rows, select_table, weighting,
    },
    support::TestResult,
};

// The pair every case asks for, chosen so it is not the one the draft opens on.
const WANTED_CATEGORY: InjuryCategory = InjuryCategory::Leg;
const WANTED_CONTEXT: DamageContext = DamageContext::Melee;

#[test]
fn selecting_a_table_loads_that_categorys_own_rows_out_of_the_live_tables() -> TestResult {
    let opening = WeightingDraft::default();
    assert_ne!(
        (WANTED_CATEGORY, WANTED_CONTEXT),
        (opening.category(), opening.context()),
        "this case's app adds no `EguiPlugin`, so the draft opens on the pair \
         `WeightingDraft::default()` holds, and the case must ask for a different one to show \
         the select did anything",
    );
    let (mut app, mut client) = injury_tab_app_and_client()?;

    let selected = select_table(&mut app, &mut client, "(category: Leg, context: Melee)")?;
    assert_eq!(selected.category, CategoryRow::Leg);
    assert_eq!(selected.context, ContextRow::Melee);

    let read = weighting(&mut app, &mut client)?;
    assert_eq!(
        read, selected,
        "the read answers the same table the select left in the draft, key and all three buckets",
    );

    let tables = injury_tables(&app)?;
    for bucket in [BucketRow::Minor, BucketRow::Major, BucketRow::Critical] {
        assert_eq!(
            read.bucket(bucket),
            live_rows(
                &tables,
                WANTED_CATEGORY,
                WANTED_CONTEXT,
                bucket.to_severity()
            ),
            "the {bucket:?} bucket must be the live tables' own rows for that key, in their own \
             order",
        );
    }
    assert!(
        !(read.minor.is_empty() && read.major.is_empty() && read.critical.is_empty()),
        "the live tables hold rows for this key, so an all-empty reply means the select read \
         nothing rather than agreeing with the tables: {read:?}",
    );
    Ok(())
}

#[test]
fn selecting_a_second_table_replaces_every_bucket_rather_than_merging_them() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;

    let melee = select_table(&mut app, &mut client, "(category: Leg, context: Melee)")?;
    let ranged = select_table(&mut app, &mut client, "(category: Head, context: Ranged)")?;

    assert_eq!(ranged.category, CategoryRow::Head);
    assert_eq!(ranged.context, ContextRow::Ranged);
    let tables = injury_tables(&app)?;
    for bucket in [BucketRow::Minor, BucketRow::Major, BucketRow::Critical] {
        assert_eq!(
            ranged.bucket(bucket),
            live_rows(
                &tables,
                InjuryCategory::Head,
                DamageContext::Ranged,
                bucket.to_severity()
            ),
            "the second select replaces the {bucket:?} bucket from the tables, so nothing the \
             first one loaded may survive in it",
        );
    }
    assert_ne!(
        melee, ranged,
        "the two keys carry different rows, so a reply that did not change means the second \
         select never reached the draft",
    );
    Ok(())
}

#[test]
fn selecting_the_table_already_open_reloads_it_and_discards_an_unsaved_row_edit() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;

    let loaded = select_table(&mut app, &mut client, "(category: Leg, context: Melee)")?;
    let edited = list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Critical), op: Add)",
    )?;
    assert_eq!(
        edited.members.len(),
        loaded.critical.len() + 1,
        "the case only shows the second select discarded an edit if the edit reached the draft \
         first: {edited:?}",
    );

    let again = select_table(&mut app, &mut client, "(category: Leg, context: Melee)")?;

    let tables = injury_tables(&app)?;
    for bucket in [BucketRow::Minor, BucketRow::Major, BucketRow::Critical] {
        assert_eq!(
            again.bucket(bucket),
            live_rows(
                &tables,
                WANTED_CATEGORY,
                WANTED_CONTEXT,
                bucket.to_severity()
            ),
            "asking for the table already open replaces the {bucket:?} bucket from the tables \
             again, so the row the Add appended is gone rather than preserved",
        );
    }
    assert_eq!(
        again, loaded,
        "the re-select leaves the draft exactly as the first select left it",
    );
    Ok(())
}
