//! Drive a pending delete across the frames its in-use check needs.

use bevy::{
    ecs::system::Local,
    log::warn,
    prelude::{App, Update, World},
};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, ContentValidationDone, FindingFamily,
    ReferringRecord,
};

use super::{
    entries::{prefab_delete_entry, weighting_delete_entry},
    registry::{DeleteEntry, DeleteRegistry, TakenRecord},
    request::{DeleteOutcome, DeleteRefusal, DeleteRequest},
};
use crate::net_qa::EditorQaAssetsRoot;

// Where a delete has got to. The record waits here while validation republishes.
#[derive(Default)]
enum Pending {
    #[default]
    Idle,
    AwaitingClear(InFlight),
    AwaitingPublish(InFlight),
}

// The delete in progress, holding the record it took out.
struct InFlight {
    family: FindingFamily,
    key:    ContentMemberKey,
    record: TakenRecord,
}

/// Register the delete driver.
pub(crate) fn register_delete(app: &mut App) {
    app.init_resource::<DeleteRegistry>();
    let mut registry = app.world_mut().resource_mut::<DeleteRegistry>();
    registry.add(prefab_delete_entry());
    registry.add(weighting_delete_entry());
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
                settle_delete(world, flight);
                Pending::Idle
            } else {
                Pending::AwaitingPublish(flight)
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
        })
    })
}

// Read the republished report and either put the record back or drop its file.
fn settle_delete(world: &mut World, flight: InFlight) {
    world.resource_scope::<DeleteRegistry, _>(|world, registry| {
        let Some(entry) = registry.entry(&flight.family) else {
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::NoEntry));
            return;
        };
        let referring = referring_records(world, &flight.family, &flight.key);
        if referring.is_empty() {
            remove_record_file(world, entry, &flight.key);
            world.insert_resource(DeleteOutcome::Removed);
        } else {
            entry.restore(world, &flight.key, flight.record);
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::InUse(referring)));
        }
    });
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
