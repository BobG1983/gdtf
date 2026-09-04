//! The weighting table's own row fields, written through the draft its bucket rows write.

use gdtf_battle_sim::injuries::{InjuryRegistry, WeightedInjuryEntry};

use crate::{
    injury_form::WeightingDraft,
    mcp::{
        commands::write::{form_fault::FormWriteFault, weighting_rows},
        wire::{
            EditorListIndexNet, InjuryKeyNet, InjuryWeightNet, WeightingBucketNet,
            WeightingFieldNet,
        },
    },
};

// The row the bucket holds at this position, or the fault a past-the-end index answers.
fn row_at(
    draft: &mut WeightingDraft,
    bucket: WeightingBucketNet,
    index: usize,
) -> Result<&mut WeightedInjuryEntry, FormWriteFault> {
    let rows = weighting_rows::bucket_mut(draft.weighting_mut(), bucket);
    let held = rows.len();
    rows.get_mut(index)
        .ok_or_else(|| weighting_rows::past_the_end(bucket, index, held))
}

fn write_injury(
    draft: &mut WeightingDraft,
    registry: Option<&InjuryRegistry>,
    bucket: WeightingBucketNet,
    index: EditorListIndexNet,
    injury: InjuryKeyNet,
) -> Result<WeightingFieldNet, FormWriteFault> {
    let key = weighting_rows::known_key(registry, &injury)?;
    let row = row_at(draft, bucket, *index)?;
    row.injury = key;
    Ok(WeightingFieldNet::RowInjury {
        bucket,
        index,
        injury: InjuryKeyNet::new(row.injury.as_str()),
    })
}

fn write_weight(
    draft: &mut WeightingDraft,
    bucket: WeightingBucketNet,
    index: EditorListIndexNet,
    weight: InjuryWeightNet,
) -> Result<WeightingFieldNet, FormWriteFault> {
    let row = row_at(draft, bucket, *index)?;
    row.weight = weight.to_weight();
    Ok(WeightingFieldNet::RowWeight {
        bucket,
        index,
        weight: InjuryWeightNet::from_weight(row.weight),
    })
}

/// Write one weighting row field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut WeightingDraft,
    registry: Option<&InjuryRegistry>,
    field: WeightingFieldNet,
) -> Result<WeightingFieldNet, FormWriteFault> {
    match field {
        WeightingFieldNet::RowInjury {
            bucket,
            index,
            injury,
        } => write_injury(draft, registry, bucket, index, injury),
        WeightingFieldNet::RowWeight {
            bucket,
            index,
            weight,
        } => write_weight(draft, bucket, index, weight),
    }
}
