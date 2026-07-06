//! The sim-ganger → presenter-sprite map resource and the per-sprite marker.

use bevy::{platform::collections::HashMap, prelude::*};

/// The presenter-owned map linking each sim ganger [`Entity`] to its presenter sprite
/// [`Entity`].
///
/// A NAMED newtype [`Resource`] over a [`HashMap`] (no-bare-types: a domain map, not a
/// bare collection field; the inner [`Entity`] keys/values are the framework
/// carve-out). `init_resource`-d by [`TopDownRendererPlugin`](crate::TopDownRendererPlugin)
/// so it is present for the whole battle span — the spawn system records into it; the
/// move / reframe / death / removal systems and the GTW-627 visibility resolver
/// (`resolve_ganger_visibility`) look up through it.
#[derive(Resource, Default, Debug)]
pub struct GangerSprites {
    /// Each `sim ganger Entity -> presenter sprite Entity` link. The framework
    /// `Entity` keys/values are the no-bare-types carve-out (a Bevy-owned identity).
    map: HashMap<Entity, Entity>,
}

impl GangerSprites {
    /// Record (or overwrite) the presenter sprite mirroring `sim` ganger.
    pub(super) fn insert(&mut self, sim: Entity, sprite: Entity) {
        self.map.insert(sim, sprite);
    }

    /// The presenter sprite mirroring `sim` ganger, if one is mapped.
    #[must_use]
    pub fn sprite_for(&self, sim: Entity) -> Option<Entity> {
        self.map.get(&sim).copied()
    }

    /// Drop the mapping for `sim` ganger, returning the presenter sprite it had (if
    /// any) so the caller can despawn it.
    pub(super) fn remove(&mut self, sim: Entity) -> Option<Entity> {
        self.map.remove(&sim)
    }

    /// Whether a presenter sprite is currently mapped for `sim` ganger.
    #[must_use]
    pub fn contains(&self, sim: Entity) -> bool {
        self.map.contains_key(&sim)
    }
}

/// Marker tagging every ganger sprite this slice spawns, carrying the sim [`Entity`] it
/// mirrors.
///
/// So a redraw / reframe / despawn finds exactly the ganger sprites — and ONLY them,
/// never the S4 [`TerrainSprite`](crate::TerrainSprite), never the S2
/// [`WorldCamera`](crate::WorldCamera). The inner [`Entity`] is the framework carve-out
/// (a Bevy-owned identity, not a domain value); the marker tags only the presenter
/// sprites this slice spawns.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GangerSprite {
    /// The sim ganger [`Entity`] this presenter sprite mirrors.
    pub entity: Entity,
}
