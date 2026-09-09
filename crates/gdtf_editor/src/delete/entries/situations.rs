//! Rewriting the loaded situation where it names a record a delete is removing.

use std::path::Path;

use bevy::prelude::World;
use gdtf_battle_sim::{
    ganger::{GangName, GangerName},
    level::ThemeUuid,
    situation::Situation,
    terrain::def::TerrainUuid,
};
use gdtf_content_families::situation::LoadedSituation;

use crate::{delete::resolution::DroppedReferences, situation::write_situation_in};

/// Point every piece key and the default floor at `replacement` instead.
pub(super) fn replace_situation_piece(
    world: &mut World,
    root: &Path,
    deleted: TerrainUuid,
    replacement: TerrainUuid,
) -> DroppedReferences {
    rewrite_situation(world, root, |situation| {
        let mut changed = false;
        for piece in situation_pieces(situation) {
            if *piece == deleted {
                *piece = replacement;
                changed = true;
            }
        }
        if situation.map.default_floor == deleted {
            situation.map.default_floor = replacement;
            changed = true;
        }
        changed
    })
}

/// Point the situation's own theme at `replacement` instead.
pub(super) fn replace_situation_theme(
    world: &mut World,
    root: &Path,
    deleted: ThemeUuid,
    replacement: ThemeUuid,
) -> DroppedReferences {
    rewrite_situation(world, root, |situation| {
        let names_it = situation.map.theme == deleted;
        if names_it {
            situation.map.theme = replacement;
        }
        names_it
    })
}

/// Point every roster entry at the `replacement` gang instead.
pub(super) fn replace_situation_gang(
    world: &mut World,
    root: &Path,
    deleted: &GangName,
    replacement: &GangName,
) -> DroppedReferences {
    rewrite_situation(world, root, |situation| {
        let mut changed = false;
        for gang in situation_gangs(situation) {
            if gang == deleted {
                *gang = replacement.clone();
                changed = true;
            }
        }
        changed
    })
}

/// Every member name the situation's rosters ask `gang` for.
pub(super) fn members_of(situation: &Situation, gang: &GangName) -> Vec<GangerName> {
    situation
        .combatants
        .rosters
        .iter()
        .filter(|member| member.gang == *gang)
        .map(|member| member.member.clone())
        .collect()
}

// Every terrain key the situation places, across all four piece lists.
fn situation_pieces(situation: &mut Situation) -> impl Iterator<Item = &mut TerrainUuid> {
    let map = &mut situation.map;
    map.walls
        .iter_mut()
        .map(|spawn| &mut spawn.piece)
        .chain(map.scatter.iter_mut().map(|spawn| &mut spawn.piece))
        .chain(map.slabs.iter_mut().map(|spawn| &mut spawn.piece))
        .chain(map.floors.iter_mut().map(|spawn| &mut spawn.piece))
}

// Every gang key the situation's rosters name.
fn situation_gangs(situation: &mut Situation) -> impl Iterator<Item = &mut GangName> {
    situation
        .combatants
        .rosters
        .iter_mut()
        .map(|member| &mut member.gang)
}

// Rewrite the situation `edit` changes, its file written before the loaded resource.
fn rewrite_situation(
    world: &mut World,
    root: &Path,
    edit: impl Fn(&mut Situation) -> bool,
) -> DroppedReferences {
    let Some(loaded) = world.get_resource::<LoadedSituation>() else {
        return DroppedReferences::Nothing;
    };
    let mut situation = (**loaded).clone();
    if !edit(&mut situation) {
        return DroppedReferences::Nothing;
    }
    if write_situation_in(root, &situation).is_err() {
        return DroppedReferences::Failed;
    }
    let Some(mut loaded) = world.get_resource_mut::<LoadedSituation>() else {
        return DroppedReferences::Failed;
    };
    *loaded.situation_mut() = situation;
    DroppedReferences::Rewritten
}
