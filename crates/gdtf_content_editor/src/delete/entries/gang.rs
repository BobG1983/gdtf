//! The gang delete: the situation names the replacement gang, member for member.

use bevy::prelude::World;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, FindingFamily};
use gdtf_battle_sim::ganger::{GangName, GangRegistry, GangRoster};
use gdtf_content_families::{GangsFamily, situation::LoadedSituation};

use super::{
    gangs::GANG_FAMILY,
    situations::{members_of, replace_situation_gang},
};
use crate::{
    delete::{
        offer::{ReplacementCandidate, ReplacementLabel, labelled_candidates},
        registry::{DeleteEntry, DeleteScreen},
        resolution::{DroppedReferences, delete_assets_root},
    },
    mode::EditorMode,
};

// The family label the loaded situation's own references are recorded under.
const SITUATION_REFERRER: &str = "LoadedSituation";

/// The delete for one gang roster, offered on the Gang tab.
pub(crate) fn gang_delete_entry() -> DeleteEntry {
    DeleteEntry::new(
        FindingFamily::new(GANG_FAMILY.to_owned()),
        DeleteScreen::new(EditorMode::Gang, None),
        Box::new(|world, key| {
            let mut registry = world.get_resource_mut::<GangRegistry>()?;
            let roster = registry.remove(&gang_name(key))?;
            Some(Box::new(roster))
        }),
        Box::new(|world, key, record| {
            let Ok(roster) = record.downcast::<GangRoster>() else {
                return;
            };
            if let Some(mut registry) = world.get_resource_mut::<GangRegistry>() {
                registry.insert(gang_name(key), *roster);
            }
        }),
        Box::new(|world, key| {
            let sources = world.get_resource::<ContentSourcePaths<GangsFamily>>()?;
            sources.path(key).cloned()
        }),
    )
    .with_candidates(Box::new(gang_candidates))
    .with_replacement_check(Box::new(missing_member))
    .replacing_in(
        FindingFamily::new(SITUATION_REFERRER.to_owned()),
        Box::new(|world, key, replacement| {
            let deleted = gang_name(key);
            let chosen = gang_name(replacement);
            delete_assets_root(world).map_or(DroppedReferences::Failed, |root| {
                replace_situation_gang(world, &root, &deleted, &chosen)
            })
        }),
    )
}

// The registry key the member key names.
fn gang_name(key: &ContentMemberKey) -> GangName {
    GangName::new((**key).clone())
}

// Every gang as a replacement row; a gang keys on its file stem, which is already unique.
fn gang_candidates(world: &World) -> Vec<ReplacementCandidate> {
    world
        .get_resource::<GangRegistry>()
        .map_or_else(Vec::new, |registry| {
            labelled_candidates(registry.keys().map(|name| {
                (
                    ContentMemberKey::new((**name).clone()),
                    ReplacementLabel::new((**name).clone()),
                )
            }))
        })
}

// The first member the situation asks the deleted gang for that the replacement lacks.
fn missing_member(
    world: &World,
    key: &ContentMemberKey,
    replacement: &ContentMemberKey,
) -> Option<ContentMemberKey> {
    let situation = world.get_resource::<LoadedSituation>()?;
    let gangs = world.get_resource::<GangRegistry>()?;
    let Some(roster) = gangs.roster(&gang_name(replacement)) else {
        return Some(replacement.clone());
    };
    members_of(situation, &gang_name(key))
        .into_iter()
        .find(|member| roster.member(member).is_none())
        .map(|member| ContentMemberKey::new((*member).clone()))
}
