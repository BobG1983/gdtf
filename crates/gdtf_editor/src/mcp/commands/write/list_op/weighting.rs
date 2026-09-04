//! The weighting table's three buckets: add a seeded row, remove one, and nothing else.

use gdtf_battle_sim::injuries::{InjuryRegistry, InjuryWeight, WeightedInjuryEntry};

use crate::{
    injury_form::WeightingDraft,
    mcp::{
        commands::write::{form_fault::FormWriteFault, weighting_rows},
        wire::{EditorListMemberNet, EditorListOpNet, WeightingBucketNet},
    },
};

const NO_TOGGLE: &str = "a weighting bucket is authored by adding, removing and rewriting rows, so it draws no tick \
     boxes and offers no toggle";

const NOT_THROUGH_THE_LIST: &str = "one weighting row is rewritten through the `Weighting(RowInjury(…))` and \
     `Weighting(RowWeight(…))` field arms, not through the list";

const NO_REORDER: &str = "a weighting bucket draws no reorder buttons. The Sprite form's animation frames and the two \
     on-death effect lists are the lists that reorder";

/// The rows the named bucket holds, as the reply reads them back.
pub(super) fn members(
    draft: &WeightingDraft,
    bucket: WeightingBucketNet,
) -> Vec<EditorListMemberNet> {
    weighting_rows::members(draft.weighting(), bucket)
}

fn add(
    draft: &mut WeightingDraft,
    registry: Option<&InjuryRegistry>,
    bucket: WeightingBucketNet,
) -> Result<(), FormWriteFault> {
    let seed = weighting_rows::seed_key(registry)?;
    weighting_rows::bucket_mut(draft.weighting_mut(), bucket)
        .push(WeightedInjuryEntry::new(seed, InjuryWeight::new(1)));
    Ok(())
}

fn remove(
    draft: &mut WeightingDraft,
    bucket: WeightingBucketNet,
    index: usize,
) -> Result<(), FormWriteFault> {
    let rows = weighting_rows::bucket_mut(draft.weighting_mut(), bucket);
    let held = rows.len();
    if index >= held {
        return Err(weighting_rows::past_the_end(bucket, index, held));
    }
    rows.remove(index);
    Ok(())
}

/// Apply one operation to a weighting bucket, matching what its row group draws.
pub(super) fn apply(
    draft: &mut WeightingDraft,
    registry: Option<&InjuryRegistry>,
    bucket: WeightingBucketNet,
    op: EditorListOpNet,
) -> Result<(), FormWriteFault> {
    match op {
        EditorListOpNet::Add => add(draft, registry, bucket),
        EditorListOpNet::Remove(index) => remove(draft, bucket, *index),
        EditorListOpNet::Toggle(_) => Err(FormWriteFault::bad(NO_TOGGLE.to_owned())),
        EditorListOpNet::SetAt(..) => Err(FormWriteFault::bad(NOT_THROUGH_THE_LIST.to_owned())),
        EditorListOpNet::MoveUp(_) | EditorListOpNet::MoveDown(_) => {
            Err(FormWriteFault::bad(NO_REORDER.to_owned()))
        }
    }
}
