use bevy::app::App;

use crate::{
    mcp_editor_weighting::{
        rows::{BucketRow, FieldRow, ListMemberRow, ListRow, WeightingFieldRow, WeightingRow},
        setup::{
            draft_weighting, injury_tab_app_and_client, list_op, set_field, sorted_injury_keys,
            table_of,
        },
    },
    mcp_shared::support::{TestError, TestResult},
};

// The rows the named bucket holds in the world right now, as a reply reads them back.
fn bucket_in_world(app: &App, bucket: BucketRow) -> Result<Vec<WeightingRow>, TestError> {
    Ok(table_of(&draft_weighting(app)?).bucket(bucket).to_vec())
}

// A row the reply carries, spelled the way the client reads it.
fn member(injury: &str, weight: u32) -> ListMemberRow {
    ListMemberRow::WeightingRow(WeightingRow {
        injury: injury.to_owned(),
        weight,
    })
}

#[test]
fn add_seeds_the_first_registry_key_at_weight_one_and_remove_takes_that_row_back_out() -> TestResult
{
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let keys = sorted_injury_keys(&app)?;
    let Some(seed) = keys.first() else {
        unreachable!("the live injury registry is loaded and holds keys: {keys:?}");
    };

    let added = list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Minor), op: Add)",
    )?;

    assert_eq!(added.list, ListRow::WeightingBucket(BucketRow::Minor));
    assert_eq!(
        added.members,
        vec![member(seed, 1)],
        "the Add button seeds the first key the row combo offers, at weight one",
    );
    assert_eq!(
        bucket_in_world(&app, BucketRow::Minor)?,
        vec![WeightingRow {
            injury: seed.clone(),
            weight: 1,
        }],
        "the reply reads back the draft the write reached, not a value the handler made up",
    );

    let removed = list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Minor), op: Remove(0))",
    )?;

    assert!(
        removed.members.is_empty(),
        "Remove takes the row out, as the row's own Remove button does: {removed:?}",
    );
    assert!(bucket_in_world(&app, BucketRow::Minor)?.is_empty());
    Ok(())
}

#[test]
fn each_bucket_adds_and_removes_its_own_rows_and_leaves_the_other_two_alone() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;

    let added = list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Major), op: Add)",
    )?;

    assert_eq!(added.list, ListRow::WeightingBucket(BucketRow::Major));
    assert_eq!(added.members.len(), 1);
    assert!(
        bucket_in_world(&app, BucketRow::Minor)?.is_empty(),
        "an Add on Major must not reach the Minor bucket",
    );
    assert!(
        bucket_in_world(&app, BucketRow::Critical)?.is_empty(),
        "an Add on Major must not reach the Critical bucket",
    );
    Ok(())
}

#[test]
fn a_rows_injury_and_its_weight_are_written_through_their_own_field_arms() -> TestResult {
    let (mut app, mut client) = injury_tab_app_and_client()?;
    let keys = sorted_injury_keys(&app)?;
    let (Some(seed), Some(other)) = (keys.first(), keys.last()) else {
        unreachable!("the live injury registry is loaded and holds keys: {keys:?}");
    };
    if seed == other {
        return Err("this case rewrites a row's key, so it needs a registry holding two".into());
    }
    list_op(
        &mut app,
        &mut client,
        "(list: WeightingBucket(Critical), op: Add)",
    )?;

    let picked = set_field(
        &mut app,
        &mut client,
        &format!("(field: Weighting(RowInjury(bucket: Critical, index: 0, injury: \"{other}\")))"),
    )?;
    assert_eq!(
        picked.field,
        FieldRow::Weighting(WeightingFieldRow::RowInjury {
            bucket: BucketRow::Critical,
            index:  0,
            injury: other.clone(),
        }),
        "the reply reads the key back as the draft stores it",
    );

    let weighed = set_field(
        &mut app,
        &mut client,
        "(field: Weighting(RowWeight(bucket: Critical, index: 0, weight: 9)))",
    )?;
    assert_eq!(
        weighed.field,
        FieldRow::Weighting(WeightingFieldRow::RowWeight {
            bucket: BucketRow::Critical,
            index:  0,
            weight: 9,
        }),
    );
    assert_eq!(
        bucket_in_world(&app, BucketRow::Critical)?,
        vec![WeightingRow {
            injury: other.clone(),
            weight: 9,
        }],
        "both field arms wrote the one row the bucket holds, key and weight together",
    );
    Ok(())
}
