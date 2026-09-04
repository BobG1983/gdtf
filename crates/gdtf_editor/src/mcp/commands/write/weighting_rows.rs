//! The weighting table's buckets and rows, shared by its field arm and its list arm.

use cobalt_mcp_protocol::command::RefusalNote;
use gdtf_battle_sim::injuries::{InjuryName, InjuryRegistry, InjuryWeighting, WeightedInjuryEntry};

use super::form_fault::FormWriteFault;
use crate::mcp::wire::{EditorListMemberNet, InjuryKeyNet, WeightingBucketNet, WeightingRowNet};

const NO_INJURY_REGISTRY: RefusalNote = RefusalNote::from_static(
    "the weighting rows pick their injury from the injury registry, and it is absent or empty, \
     so the form disables its own Add button",
);

/// The rows the named bucket holds, as the panel's own row group draws them.
pub(in crate::mcp::commands::write) fn bucket(
    weighting: &InjuryWeighting,
    bucket: WeightingBucketNet,
) -> &[WeightedInjuryEntry] {
    match bucket {
        WeightingBucketNet::Minor => &weighting.minor,
        WeightingBucketNet::Major => &weighting.major,
        WeightingBucketNet::Critical => &weighting.critical,
    }
}

/// The rows the named bucket holds, for a write that edits them.
pub(in crate::mcp::commands::write) const fn bucket_mut(
    weighting: &mut InjuryWeighting,
    bucket: WeightingBucketNet,
) -> &mut Vec<WeightedInjuryEntry> {
    match bucket {
        WeightingBucketNet::Minor => &mut weighting.minor,
        WeightingBucketNet::Major => &mut weighting.major,
        WeightingBucketNet::Critical => &mut weighting.critical,
    }
}

/// One bucket's rows, as a reply reads them back.
pub(in crate::mcp::commands::write) fn members(
    weighting: &InjuryWeighting,
    named: WeightingBucketNet,
) -> Vec<EditorListMemberNet> {
    bucket(weighting, named)
        .iter()
        .map(|entry| EditorListMemberNet::WeightingRow(WeightingRowNet::from_entry(entry)))
        .collect()
}

/// The fault an index beyond a bucket's own length answers.
pub(in crate::mcp::commands::write) fn past_the_end(
    named: WeightingBucketNet,
    index: usize,
    held: usize,
) -> FormWriteFault {
    FormWriteFault::bad(format!(
        "{index} is past the end of the {named:?} bucket, which holds {held}"
    ))
}

/// The first key the row combo offers, which is what the Add button seeds.
pub(in crate::mcp::commands::write) fn seed_key(
    registry: Option<&InjuryRegistry>,
) -> Result<InjuryName, FormWriteFault> {
    let mut keys: Vec<&InjuryName> =
        registry.map_or_else(Vec::new, |held| held.iter().map(|(key, _)| key).collect());
    keys.sort();
    match keys.first() {
        Some(first) => Ok((*first).clone()),
        None => Err(FormWriteFault::MissingModel(NO_INJURY_REGISTRY)),
    }
}

/// The key the row combo offers under this name, or the fault a name it lacks answers.
pub(in crate::mcp::commands::write) fn known_key(
    registry: Option<&InjuryRegistry>,
    named: &InjuryKeyNet,
) -> Result<InjuryName, FormWriteFault> {
    let Some(registry) = registry else {
        return Err(FormWriteFault::MissingModel(NO_INJURY_REGISTRY));
    };
    let wanted = InjuryName::new((**named).clone());
    if registry.contains(&wanted) {
        Ok(wanted)
    } else {
        Err(FormWriteFault::bad(format!(
            "`{}` is not an injury the registry holds, so the row combo offers no such key",
            **named
        )))
    }
}
