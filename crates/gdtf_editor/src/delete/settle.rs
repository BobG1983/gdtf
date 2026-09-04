//! Read the republished report, resolve every referrer, and settle the delete.

use bevy::{log::warn, prelude::World};
use gdtf_assets::{
    ContentChecksComplete, ContentFinding, ContentIntegrityReport, ContentMemberKey,
    ContentValidationDone, FindingFamily, ReferringRecord,
};

use super::{
    offer::{OfferResolution, ReplacementOffer},
    registry::{DeleteEntry, DeleteRegistry},
    request::{DeleteOutcome, DeleteRefusal},
    resolution::{DroppedReferences, ReferenceResolution},
    systems::{InFlight, Pending},
};
use crate::mcp::EditorQaAssetsRoot;

/// Read the republished report and either remove the record, ask, or rewrite.
pub(super) fn settle_delete(world: &mut World, flight: InFlight) -> Pending {
    world.resource_scope::<DeleteRegistry, _>(|world, registry| {
        let Some(entry) = registry.entry(&flight.family) else {
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::NoEntry));
            return Pending::Idle;
        };
        let referring = referring_records(world, &flight.family, &flight.key);
        if referring.is_empty() {
            finish_removal(world, entry, &flight.key);
            return Pending::Idle;
        }
        let families = referring_families(&referring);
        if !families
            .iter()
            .all(|family| entry.resolution(family).is_some())
        {
            refuse_in_use(world, entry, flight, referring);
            return Pending::Idle;
        }
        if families.iter().any(|family| {
            matches!(
                entry.resolution(family),
                Some(ReferenceResolution::Replace(_))
            )
        }) {
            world.insert_resource(ReplacementOffer::new(flight.candidates.clone()));
            return Pending::AwaitingOffer(InFlight {
                referring,
                ..flight
            });
        }
        let dropped = resolve_all(world, entry, &families, &flight.key, None);
        after_rewrite(world, entry, flight, referring, dropped)
    })
}

/// Wait on the author's answer, then cancel, refuse, or rewrite every referrer.
pub(super) fn settle_offer(world: &mut World, flight: InFlight) -> Pending {
    let Some(offer) = world.get_resource::<ReplacementOffer>() else {
        return Pending::AwaitingOffer(flight);
    };
    let Some(resolution) = offer.resolve else {
        return Pending::AwaitingOffer(flight);
    };
    let chosen = offer.choose.clone().filter(|key| offer.offers(key));
    world.remove_resource::<ReplacementOffer>();
    world.resource_scope::<DeleteRegistry, _>(|world, registry| {
        let Some(entry) = registry.entry(&flight.family) else {
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::NoEntry));
            return Pending::Idle;
        };
        if resolution == OfferResolution::Cancel {
            entry.restore(world, &flight.key, flight.record);
            world.insert_resource(DeleteOutcome::Cancelled);
            return Pending::Idle;
        }
        let Some(chosen) = chosen else {
            let referring = flight.referring.clone();
            refuse_in_use(world, entry, flight, referring);
            return Pending::Idle;
        };
        if let Some(missing) = entry.replacement_fault(world, &flight.key, &chosen) {
            entry.restore(world, &flight.key, flight.record);
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::ReplacementLacks(
                missing,
            )));
            return Pending::Idle;
        }
        let families = referring_families(&flight.referring);
        let dropped = resolve_all(world, entry, &families, &flight.key, Some(&chosen));
        let referring = flight.referring.clone();
        after_rewrite(world, entry, flight, referring, dropped)
    })
}

/// Read the report the rewrites re-armed, and remove the record only once it is clean.
pub(super) fn settle_after_drop(world: &mut World, flight: InFlight) -> Pending {
    world.resource_scope::<DeleteRegistry, _>(|world, registry| {
        let Some(entry) = registry.entry(&flight.family) else {
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::NoEntry));
            return Pending::Idle;
        };
        if referring_records(world, &flight.family, &flight.key).is_empty() {
            finish_removal(world, entry, &flight.key);
        } else {
            let referring = flight.referring.clone();
            refuse_in_use(world, entry, flight, referring);
        }
        Pending::Idle
    })
}

// Wait on a fresh report when something was rewritten, and refuse when nothing was.
fn after_rewrite(
    world: &mut World,
    entry: &DeleteEntry,
    flight: InFlight,
    referring: Vec<ReferringRecord>,
    dropped: DroppedReferences,
) -> Pending {
    if dropped == DroppedReferences::Rewritten {
        rearm_validation(world);
        return Pending::AwaitingDropCheck(InFlight {
            referring,
            ..flight
        });
    }
    refuse_in_use(world, entry, flight, referring);
    Pending::Idle
}

// Run each referring family's own resolution, worst answer winning.
fn resolve_all(
    world: &mut World,
    entry: &DeleteEntry,
    families: &[FindingFamily],
    key: &ContentMemberKey,
    replacement: Option<&ContentMemberKey>,
) -> DroppedReferences {
    let mut answer = DroppedReferences::Nothing;
    for family in families {
        let part = entry
            .resolve_references(world, family, key, replacement)
            .unwrap_or(DroppedReferences::Failed);
        answer = combined(answer, part);
    }
    answer
}

// A failed part loses to nothing else, and one rewrite is enough to re-check.
const fn combined(left: DroppedReferences, right: DroppedReferences) -> DroppedReferences {
    match (left, right) {
        (DroppedReferences::Failed, _) | (_, DroppedReferences::Failed) => {
            DroppedReferences::Failed
        }
        (DroppedReferences::Rewritten, _) | (_, DroppedReferences::Rewritten) => {
            DroppedReferences::Rewritten
        }
        _ => DroppedReferences::Nothing,
    }
}

// Each family holding a reference, once, in the order the report raised them.
fn referring_families(referring: &[ReferringRecord]) -> Vec<FindingFamily> {
    let mut families: Vec<FindingFamily> = Vec::new();
    for record in referring {
        if !families.iter().any(|held| **held == *record.family) {
            families.push(record.family.clone());
        }
    }
    families
}

// The record stays out, and its own file goes.
fn finish_removal(world: &mut World, entry: &DeleteEntry, key: &ContentMemberKey) {
    remove_record_file(world, entry, key);
    world.insert_resource(DeleteOutcome::Removed);
}

// The record goes back, and the refusal names the records that still refer to it.
fn refuse_in_use(
    world: &mut World,
    entry: &DeleteEntry,
    flight: InFlight,
    referring: Vec<ReferringRecord>,
) {
    entry.restore(world, &flight.key, flight.record);
    world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::InUse(referring)));
}

// The rewritten referrers need a report of their own, so ask for one pass more.
fn rearm_validation(world: &mut World) {
    world.insert_resource(ContentIntegrityReport::default());
    world.remove_resource::<ContentChecksComplete>();
    world.remove_resource::<ContentValidationDone>();
}

// Every referring record the republished report raises against the dropped key.
fn referring_records(
    world: &World,
    family: &FindingFamily,
    key: &ContentMemberKey,
) -> Vec<ReferringRecord> {
    let Some(report) = world.get_resource::<ContentIntegrityReport>() else {
        return Vec::new();
    };
    report
        .findings()
        .iter()
        .filter_map(|finding| match finding {
            ContentFinding::DanglingRef {
                referring_record,
                target,
                family: found_family,
                ..
            } if **target == **key && **found_family == **family => Some(referring_record.clone()),
            _ => None,
        })
        .collect()
}

// A file that will not delete leaves the registry entry gone and says so loudly.
fn remove_record_file(world: &World, entry: &DeleteEntry, key: &ContentMemberKey) {
    let Some(root) = world.get_resource::<EditorQaAssetsRoot>().cloned() else {
        warn!(
            "delete removed `{}` from {} but no assets root says where its file lives",
            **key,
            **entry.family(),
        );
        return;
    };
    let Some(relative) = entry.relative_path(world, key) else {
        warn!(
            "delete removed `{}` from {} but its entry knows no file for that key",
            **key,
            **entry.family(),
        );
        return;
    };
    if let Err(error) = std::fs::remove_file(root.join(&*relative)) {
        warn!(
            "delete removed `{}` from {} but could not remove its file: {error}",
            **key,
            **entry.family(),
        );
    }
}
