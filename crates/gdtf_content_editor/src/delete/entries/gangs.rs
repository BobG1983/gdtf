//! Rewriting the gang members whose loadout names a record that is being deleted.

use bevy::prelude::World;
use gdtf_battle_sim::ganger::{GangMember, GangName, GangRegistry, GangRoster};

use crate::{
    delete::resolution::{DroppedReferences, delete_assets_root},
    gang_form::write_gang_in,
};

// Rewrite every member `edit` changes, each gang's file written before its registry entry.
pub(super) fn drop_from_gang_members(
    world: &mut World,
    edit: impl Fn(&mut GangMember) -> bool,
) -> DroppedReferences {
    let Some(root) = delete_assets_root(world) else {
        return DroppedReferences::Failed;
    };
    let Some(gangs) = world.get_resource::<GangRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<(GangName, GangRoster)> = gangs
        .iter()
        .filter_map(|(name, roster)| {
            rewritten_roster(roster, &edit).map(|next| (name.clone(), next))
        })
        .collect();
    if rewritten.is_empty() {
        return DroppedReferences::Nothing;
    }
    for (name, roster) in &rewritten {
        if write_gang_in(&root, name, roster).is_err() {
            return DroppedReferences::Failed;
        }
    }
    let Some(mut gangs) = world.get_resource_mut::<GangRegistry>() else {
        return DroppedReferences::Failed;
    };
    for (name, roster) in rewritten {
        gangs.insert(name, roster);
    }
    DroppedReferences::Rewritten
}

// The roster `edit` changed, or nothing when it left every member alone.
fn rewritten_roster(
    roster: &GangRoster,
    edit: &impl Fn(&mut GangMember) -> bool,
) -> Option<GangRoster> {
    let mut next = roster.clone();
    let mut changed = false;
    for member in &mut next.members {
        if edit(member) {
            changed = true;
        }
    }
    changed.then_some(next)
}
