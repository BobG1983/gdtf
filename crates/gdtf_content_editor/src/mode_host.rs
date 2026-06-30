//! Finding a region's per-mode content container (GTW-474).
//!
//! The Workbench shell spawns, under each of the four regions, a per-mode content container
//! ([`PrefabModeContent`](crate::mode::PrefabModeContent) /
//! [`TerrainModeContent`](crate::mode::TerrainModeContent)) so a mode switch toggles
//! [`Visibility`](bevy::prelude::Visibility) rather than despawning. The existing painter's
//! content and the GTW-474 terrain form each parent into the matching container, so they inherit
//! its visibility. This module owns the ONE descendant-search helper every parenting deferred
//! command uses to locate that container.

use bevy::prelude::*;

/// Find the per-mode content container marked `Host` that lives under the region marked
/// `Region` — the parenting target the existing painter / the terrain form hangs its content
/// on (GTW-474).
///
/// Walks the region's descendant subtree breadth-first for the FIRST entity carrying the
/// `Host` marker (each region holds exactly one container per mode). Returns [`None`] if the
/// region (or the container) is not present yet — the caller's deferred command then no-ops and
/// is retried (the command is queued each spawn, so a one-frame ordering miss self-heals).
///
/// Exclusive `&mut World` is the deferred-command idiom this is called from (a
/// `commands.queue` closure builds a `QueryState`, which needs `&mut World`), where the region /
/// child lookups read the world directly — NOT a registered system (bevy-traps #7 carve-out:
/// this is a helper invoked inside an already-`&mut World` deferred command, the same idiom the
/// shell's `parent_scroll_root_under` and every existing parenting closure use).
#[must_use]
pub(crate) fn mode_host_under_region<Region: Component, Host: Component>(
    world: &mut World,
) -> Option<Entity> {
    let region = world
        .query_filtered::<Entity, With<Region>>()
        .iter(world)
        .next()?;
    let mut stack = vec![region];
    while let Some(entity) = stack.pop() {
        if entity != region && world.get::<Host>(entity).is_some() {
            return Some(entity);
        }
        if let Some(children) = world.get::<Children>(entity) {
            stack.extend(children.iter());
        }
    }
    None
}
