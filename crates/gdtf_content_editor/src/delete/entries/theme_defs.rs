//! Rewriting the theme defs that name a terrain piece a delete is removing.

use std::path::Path;

use bevy::prelude::World;
use gdtf_assets::{ContentMemberKey, ContentSourcePaths, write_ron_pretty};
use gdtf_battle_sim::{
    level::{ThemeUuid, UuidThemeDef, UuidThemeRegistry},
    terrain::def::TerrainUuid,
};
use gdtf_content_families::ThemeDefsFamily;

use crate::delete::{
    offer::{ReplacementCandidate, ReplacementLabel, labelled_candidates},
    resolution::DroppedReferences,
};

/// The key a theme def's source path and every finding against it are recorded under.
pub(super) fn theme_member_key(key: ThemeUuid) -> ContentMemberKey {
    ContentMemberKey::new((*key).to_string())
}

/// The theme UUID a member key names, if it is one.
pub(super) fn theme_uuid(key: &ContentMemberKey) -> Option<ThemeUuid> {
    bevy::asset::uuid::Uuid::parse_str(key)
        .ok()
        .map(ThemeUuid::new)
}

/// Every theme def as a replacement row, its display name disambiguated by key.
pub(super) fn theme_candidates(world: &World) -> Vec<ReplacementCandidate> {
    world
        .get_resource::<UuidThemeRegistry>()
        .map_or_else(Vec::new, |registry| {
            labelled_candidates(registry.defs().map(|(key, def)| {
                (
                    theme_member_key(*key),
                    ReplacementLabel::new((*def.display_name).clone()),
                )
            }))
        })
}

/// Point every theme's default floor and terrain palette at `replacement` instead.
///
/// A palette already holding the replacement keeps one entry, never two.
pub(super) fn replace_theme_piece(
    world: &mut World,
    root: &Path,
    deleted: TerrainUuid,
    replacement: TerrainUuid,
) -> DroppedReferences {
    let Some(registry) = world.get_resource::<UuidThemeRegistry>() else {
        return DroppedReferences::Nothing;
    };
    let rewritten: Vec<(ThemeUuid, UuidThemeDef)> = registry
        .defs()
        .filter_map(|(key, def)| {
            let mut next = def.clone();
            repointed(&mut next, deleted, replacement).then_some((*key, next))
        })
        .collect();
    if rewritten.is_empty() {
        return DroppedReferences::Nothing;
    }
    let Some(sources) = world.get_resource::<ContentSourcePaths<ThemeDefsFamily>>() else {
        return DroppedReferences::Failed;
    };
    let written = rewritten.iter().all(|(key, def)| {
        sources
            .path(&theme_member_key(*key))
            .is_some_and(|relative| write_ron_pretty(&root.join(&**relative), def).is_ok())
    });
    if !written {
        return DroppedReferences::Failed;
    }
    let Some(mut registry) = world.get_resource_mut::<UuidThemeRegistry>() else {
        return DroppedReferences::Failed;
    };
    for (key, def) in rewritten {
        registry.insert(key, def);
    }
    DroppedReferences::Rewritten
}

// Repoint one theme's floor and palette, answering whether it named the deleted piece.
fn repointed(def: &mut UuidThemeDef, deleted: TerrainUuid, replacement: TerrainUuid) -> bool {
    let mut changed = false;
    if def.default_floor == deleted {
        def.default_floor = replacement;
        changed = true;
    }
    if def.terrain.contains(&deleted) {
        changed = true;
        let mut palette: Vec<TerrainUuid> = Vec::with_capacity(def.terrain.len());
        for piece in &def.terrain {
            let next = if *piece == deleted {
                replacement
            } else {
                *piece
            };
            if !palette.contains(&next) {
                palette.push(next);
            }
        }
        def.terrain = palette;
    }
    changed
}
