//! The weighting table's own row fields, written through the draft its bucket rows write.

use gdtf_battle_sim::injuries::{InjuryRegistry, WeightedInjuryEntry};

use crate::{
    injury_form::WeightingDraft,
    net_qa::{
        commands::write::{form_fault::FormWriteFault, weighting_rows},
        wire::{
            EditorFieldNet, EditorListIndexNet, InjuryKeyNet, InjuryWeightNet, WeightingBucketNet,
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
) -> Result<EditorFieldNet, FormWriteFault> {
    let key = weighting_rows::known_key(registry, &injury)?;
    let row = row_at(draft, bucket, *index)?;
    row.injury = key;
    Ok(EditorFieldNet::WeightingRowInjury {
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
) -> Result<EditorFieldNet, FormWriteFault> {
    let row = row_at(draft, bucket, *index)?;
    row.weight = weight.to_weight();
    Ok(EditorFieldNet::WeightingRowWeight {
        bucket,
        index,
        weight: InjuryWeightNet::from_weight(row.weight),
    })
}

/// Write one weighting row field, answering the field as the draft stores it.
pub(super) fn write(
    draft: &mut WeightingDraft,
    registry: Option<&InjuryRegistry>,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::WeightingRowInjury {
            bucket,
            index,
            injury,
        } => write_injury(draft, registry, bucket, index, injury),
        EditorFieldNet::WeightingRowWeight {
            bucket,
            index,
            weight,
        } => write_weight(draft, bucket, index, weight),
        _ => Err(FormWriteFault::ForeignArm),
    }
}
