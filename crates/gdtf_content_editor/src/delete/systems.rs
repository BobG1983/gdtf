//! Drive a pending delete across the frames its in-use check needs.

use bevy::{
    ecs::system::Local,
    prelude::{App, Update, World},
};
use gdtf_assets::{ContentMemberKey, ContentValidationDone, FindingFamily, ReferringRecord};

use super::{
    entries::{
        armor_delete_entry, attachment_delete_entry, field_delete_entry, gang_delete_entry,
        injury_delete_entry, melee_weapon_delete_entry, prefab_delete_entry, terrain_delete_entry,
        theme_delete_entry, weapon_delete_entry, weighting_delete_entry,
    },
    offer::ReplacementCandidate,
    registry::{DeleteRegistry, TakenRecord},
    request::{DeleteOutcome, DeleteRefusal, DeleteRequest},
    settle::{settle_after_drop, settle_delete, settle_offer},
};

// Where a delete has got to. The record waits here while validation republishes.
#[derive(Default)]
pub(super) enum Pending {
    #[default]
    Idle,
    AwaitingClear(InFlight),
    AwaitingPublish(InFlight),
    AwaitingOffer(InFlight),
    AwaitingDropCheck(InFlight),
}

// The delete in progress, holding the record it took out.
pub(super) struct InFlight {
    pub(super) family:     FindingFamily,
    pub(super) key:        ContentMemberKey,
    pub(super) record:     TakenRecord,
    pub(super) referring:  Vec<ReferringRecord>,
    pub(super) candidates: Vec<ReplacementCandidate>,
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
    registry.add(terrain_delete_entry());
    registry.add(theme_delete_entry());
    registry.add(gang_delete_entry());
    registry.add(weapon_delete_entry());
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
        Pending::AwaitingOffer(flight) => *pending = settle_offer(world, flight),
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
        // The offer names what the registry held before the record came out of it.
        let candidates = entry.candidates(world, request.key());
        let Some(record) = entry.take(world, request.key()) else {
            world.insert_resource(DeleteOutcome::Refused(DeleteRefusal::NoRecord));
            return Pending::Idle;
        };
        Pending::AwaitingClear(InFlight {
            family: request.family().clone(),
            key: request.key().clone(),
            record,
            referring: Vec::new(),
            candidates,
        })
    })
}
