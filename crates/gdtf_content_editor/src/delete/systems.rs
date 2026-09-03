//! Drive a pending delete across the frames its in-use check needs.

use bevy::{
    ecs::system::Local,
    log::warn,
    prelude::{App, Update, World},
};
use gdtf_assets::{
    ContentChecksComplete, ContentFinding, ContentIntegrityReport, ContentMemberKey,
    ContentValidationDone, FindingFamily, ReferringRecord,
};

use super::{
    entries::{
        armor_delete_entry, attachment_delete_entry, field_delete_entry, injury_delete_entry,
        melee_weapon_delete_entry, prefab_delete_entry, weighting_delete_entry,
    },
    registry::{DeleteEntry, DeleteRegistry, TakenRecord},
    request::{DeleteOutcome, DeleteRefusal, DeleteRequest},
    resolution::DroppedReferences,
};
use crate::net_qa::EditorQaAssetsRoot;

// Where a delete has got to. The record waits here while validation republishes.
#[derive(Default)]
enum Pending {
    #[default]
    Idle,
    AwaitingClear(InFlight),
    AwaitingPublish(InFlight),
    AwaitingDropCheck(InFlight),
}

// The delete in progress, holding the record it took out.
struct InFlight {
    family:    FindingFamily,
    key:       ContentMemberKey,
    record:    TakenRecord,
    referring: Vec<ReferringRecord>,
}

/// Register the delete driver.
pub(crate) fn register_delete(app: &mut App) {
    app.init_resource::<DeleteRegistry>();
    let mut registry = app.world_mut().resource_mut::<DeleteRegistry>();
    registry.add(prefab_delete_entry());
    registry.add(weighting_delete_entry());
    registry.add(armor_delete_entry());
    registry.add(melee_weapon_delete_entry());
    registry.add(attachment_delete_entry());
    registry.add(injury_delete_entry());
    registry.add(field_delete_entry());
    app.add_systems(Update, run_pending_delete);
}

// Start, wait on, and settle the pending delete.
fn run_pending_delete(world: &mut World, mut pending: Local<Pending>) {
    match core::mem::take(&mut *pending) {
        Pending::Idle => *pending = start_delete(world),
        Pending::AwaitingClear(flight) => {
            *pending = if world.get_resource::<ContentValidationDone>().is_none() {
                Pending::AwaitingPublish(flight)
            } else {
                Pending::AwaitingClear(flight)
            };
        }
        Pending::AwaitingPublish(flight) => {
            *pending = if world.get_resource::<ContentValidationDone>().is_some() {
                settle_delete(world, flight)
            } else {
                Pending::AwaitingPublish(flight)
            };
        }
        Pending::AwaitingDropCheck(flight) => {
            *pending = if world.get_resource::<ContentValidationDone>().is_some() {
                settle_after_drop(world, flight)
            } else {
                Pending::AwaitingDropCheck(flight)
            };
        }
    }
}

// Take the record out, or refuse on the frame the request is seen.
fn start_delete(world: &mut World) -> Pending {
    let Some(request) = world.get_resource::<DeleteRequest>().cloned() else {
        return Pending::Idle;
    };
    // The report the delete is about to invalidate has to have settled first.
    if world.get_resource::<ContentValidationDone>().is_none()
        && world.resource::<DeleteRegistry>().handles(request.family())
    {
        return Pending::Idle;
    }
    world.remove_resource::<DeleteRequest>();
    world.resource_scope::<DeleteRegistry, _>(|world, registry| {
        let Some(entry) = registry.entry(request.family()) else {
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::NoEntry));
            return Pending::Idle;
        };
        let Some(record) = entry.take(world, request.key()) else {
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::NoRecord));
            return Pending::Idle;
        };
        Pending::AwaitingClear(InFlight {
            family: request.family().clone(),
            key: request.key().clone(),
            record,
            referring: Vec::new(),
        })
    })
}

// Read the republished report, and rewrite every referrer the entry can drop.
fn settle_delete(world: &mut World, flight: InFlight) -> Pending {
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
        if entry.drop_references(world, &flight.key) == Some(DroppedReferences::Rewritten) {
            rearm_validation(world);
            return Pending::AwaitingDropCheck(InFlight {
                referring,
                ..flight
            });
        }
        refuse_in_use(world, entry, flight, referring);
        Pending::Idle
    })
}

// Read the report the rewrites re-armed, and remove the record only once it is clean.
fn settle_after_drop(world: &mut World, flight: InFlight) -> Pending {
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
