//! The **tag-derived vision-blocking surface** (GTW-502, child 482b) — the
//! [`VisionBlocking`] `(cell, level) → band` map the [`OccupancyGrid`] holds, plus the
//! [`project_vision_blocking`] system that keeps it in sync with the
//! [`BlocksVision`](crate::terrain::entity::BlocksVision) components.
//!
//! ## Why a separate vision surface (GTW-502, the GTW-501 D1 mirror)
//!
//! GTW-501 split PATH-blocking out of the kind-based occupancy so the pathfinder reads a
//! tag-derived surface. GTW-502 is the VISION mirror: a tag-derived, **height-aware**
//! occluder surface the LoS/FoV march ([`impact_at`](crate::march)) reads ALONGSIDE the
//! existing occupant + cover occlusion. It is VISION-ONLY and INDEPENDENT of the path
//! surface — a path-only blocker never occludes sight, and a vision-only occluder never
//! blocks a path (GTW-502 C7).
//!
//! ## Zero-regression + no-double-count reconciliation (GTW-502 D-CRITICAL)
//!
//! A `Wall`/`Cover` ALREADY occludes vision today: it is seeded into the
//! [`CoverLedger`](crate::cover::CoverLedger) at setup carrying its `height_band`, and
//! [`impact_at`](crate::march) stops sight on that ledger entry (height-aware, destroyed-cover
//! excluded). The [`BlocksVision`](crate::terrain::entity::BlocksVision) component
//! ([`derives_vision_occlusion`](crate::terrain::def::derives_vision_occlusion)) derives a
//! `Cover`/`Emplacement`'s ledger band exactly, and a `Wall`'s `Full` → `High` band, which
//! equals the ledger band for every shipped wall (all author `height_band: High`); so this
//! surface reproduces their existing occlusion EXACTLY for shipped content — and because
//! `impact_at` runs the cover-ledger clause FIRST, an intact `Wall`/`Cover` already stopped the
//! round before the vision-surface clause is reached, so the surface is a harmless idempotent
//! re-block, never a double-count. The NET-NEW behaviour
//! is the explicit-`BlocksVision`-tag opt-in for a `Slab` (which the cover ledger never held)
//! — a desirable gap-closer.
//!
//! ## Source of truth vs. projected snapshot (GTW-502 C2 / C4)
//!
//! The [`BlocksVision`](crate::terrain::entity::BlocksVision) components on terrain entities
//! are the SOURCE OF TRUTH (derived from the def at spawn, GTW-502 C1). This
//! [`VisionBlocking`] map is the PROJECTED snapshot [`impact_at`](crate::march) reads — kept
//! in sync by [`project_vision_blocking`] via `Added<BlocksVision>` /
//! `Changed<BlocksVision>` / `RemovedComponents<BlocksVision>` change detection, so adding the
//! occluder re-blocks a cell, RE-TUNING its band updates the cell, and removing it re-opens
//! the sightline — with no full rebuild.

use bevy::{
    ecs::{entity::Entity, system::SystemParam},
    platform::collections::HashMap,
    prelude::{Added, Changed, Deref, Local, Or, Query, RemovedComponents, ResMut},
};

use super::OccupancyGrid;
use crate::{
    cover::HeightBand,
    metric::CellLevel,
    terrain::entity::{BlocksVision, TerrainCell},
};

/// The change-detection FILTER a `BlocksVision` add-or-retune query observes — a terrain
/// entity that GAINED (`Added`) OR RETUNED (`Changed`) its
/// [`BlocksVision`](crate::terrain::entity::BlocksVision) component this tick (GTW-502 C4 /
/// C6). `Added` implies `Changed`, so the `Or` reads as "added-or-retuned"; BOTH halves are
/// named because the height-aware band can change in place (a re-tune must re-project, unlike
/// the GTW-501 path SET which only tracks presence). Shared by the projection
/// ([`project_vision_blocking`]) and the squad-fog trigger
/// ([`should_recompute_visibility`](crate::visibility::should_recompute_visibility)) so both
/// read the SAME filter (and so the query type stays under clippy's `type_complexity` gate).
pub type VisionOccluderChanged = Or<(Added<BlocksVision>, Changed<BlocksVision>)>;

/// The occupancy grid's **tag-derived vision-blocking** `(cell, level) → `[`HeightBand`] map
/// — the projected snapshot of the [`BlocksVision`](crate::terrain::entity::BlocksVision)
/// components that the LoS/FoV march ([`impact_at`](crate::march)) reads (GTW-502 C3).
///
/// A named newtype over the `(cell, level) → band` map (no-bare-types: the vision-occluder
/// surface is a domain value, not a bare `HashMap`). Private inner + derived read-only
/// [`Deref`] to the map for `get` / `contains_key` / `len` / `iter`; mutation goes through
/// the named [`insert`](VisionBlocking::insert) / [`remove`](VisionBlocking::remove) methods
/// (so the invariant — only the projection writes it — is concentrated here).
///
/// Unlike the GTW-501 [`PathBlocking`](super::PathBlocking) `(cell, level)` SET, this is a
/// MAP: an occluder is **height-aware**, so the surface stores not just THAT a cell occludes
/// but at WHICH [`HeightBand`] (the band a sightline must fly strictly higher than to clear
/// it). Like the path surface it is two-way (a component can be added, retuned, OR removed at
/// runtime, GTW-502 C4).
#[derive(Deref, Debug, Clone, Default, PartialEq, Eq)]
pub struct VisionBlocking(HashMap<CellLevel, HeightBand>);

impl VisionBlocking {
    /// Build an empty vision-blocking map (no cell occludes sight yet).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark `cell_level` vision-occluding at `band` — the insert the projection makes for an
    /// `Added`/`Changed`<`BlocksVision`> component. Re-inserting OVERWRITES the band (so a
    /// retuned occluder updates the cell in place — map semantics, the height-aware
    /// difference from the path SET).
    pub fn insert(&mut self, cell_level: CellLevel, band: HeightBand) {
        self.0.insert(cell_level, band);
    }

    /// Clear `cell_level`'s vision-occlusion — the removal the projection makes for a
    /// `RemovedComponents<BlocksVision>` component. Removing an absent cell is a harmless
    /// no-op (map semantics).
    pub fn remove(&mut self, cell_level: &CellLevel) {
        self.0.remove(cell_level);
    }
}

/// The terrain-entity reads the [`project_vision_blocking`] system needs, bundled as one
/// [`SystemParam`] so the system signature reads cleanly (GTW-502 C4).
///
/// One query plus the [`RemovedComponents`] reader (a special `SystemParam` that must be
/// drained every run, `bevy-traps.md` #4 — it is NOT a `Query`):
///
/// - `changed` — every terrain entity that JUST gained OR retuned a
///   [`BlocksVision`](crate::terrain::entity::BlocksVision) component this tick. The filter is
///   `Or<(Added<BlocksVision>, Changed<BlocksVision>)>`: `Added` fires on the setup-spawn
///   insert and a later runtime add, `Changed` ALSO fires when the component's band is
///   mutated in place — BOTH are needed here (unlike the GTW-501 path SET, the height-aware
///   band can change, so a re-tune must re-project). Note `Added` IMPLIES `Changed`, so the
///   `Or` is belt-and-braces, intentionally read as "added-or-retuned". Each row carries the
///   [`Entity`] + [`TerrainCell`] + the [`BlocksVision`] band so the projection knows which
///   cell to occlude, at which band, and can record the entity→cell mapping for a later
///   removal.
/// - `removed` — the `Entity`s whose `BlocksVision` component was removed this tick. A
///   `RemovedComponents` yields ONLY the `Entity` — and on a DESPAWN the entity is already
///   gone, so its [`TerrainCell`] can no longer be queried; that is why the system keeps the
///   entity→cell map in a `Local` (see [`project_vision_blocking`]).
#[derive(SystemParam)]
pub struct VisionBlockingChanges<'w, 's> {
    /// Terrain entities that gained OR retuned the component this tick (incl. the setup-spawn
    /// insert), paired with the entity + cell + band so the projection can occlude the cell
    /// at the right band and record the entity→cell mapping.
    changed:
        Query<'w, 's, (Entity, &'static TerrainCell, &'static BlocksVision), VisionOccluderChanged>,
    /// The entities whose `BlocksVision` component was removed this tick (component-remove OR
    /// despawn). Only the `Entity` is available — the cell is recovered from the `Local`.
    removed: RemovedComponents<'w, 's, BlocksVision>,
}

/// Project the [`BlocksVision`](crate::terrain::entity::BlocksVision) components into the
/// [`OccupancyGrid`]'s [`VisionBlocking`] surface, keeping it in sync via change detection
/// (GTW-502 C4).
///
/// Two halves, both reconciled against the entity→cell map the system keeps in a `Local`:
///
/// - **Removed** — every `RemovedComponents<BlocksVision>` clears the cell it last occluded,
///   re-opening the sightline (C4 / C6). `RemovedComponents` is a special `SystemParam` that
///   MUST be drained each run (`bevy-traps.md` #4) — the loop reads it every tick, never under
///   a `run_if` that could skip it. It yields only the `Entity`, and on a DESPAWN the entity
///   is already gone (so its `TerrainCell` can no longer be queried); the `tracked` `Local`
///   map holds the cell so removal works for BOTH a component-remove and a despawn. Drained
///   FIRST so a cell that lost AND regained the component in the same tick ends OCCLUDED (the
///   add below wins, matching the component's final state).
/// - **Added/Changed** — every `Added`/`Changed`<`BlocksVision`> (which fires for the
///   setup-spawn insert, a later runtime add, AND a band re-tune) occludes its cell at the
///   component's band on the grid AND records the entity→cell mapping. This is how the surface
///   is populated at setup, re-occluded when an occluder is added at runtime, and updated when
///   its band is retuned (C4 / C6).
///
/// `tracked` is a system-private [`Local`] entity→cell map: an occluder's cell is recorded on
/// add and forgotten on removal. It is the projection's OWN reverse index, NOT a domain
/// resource — system-private scratch state, so it stays a plain `HashMap` (`no-bare-types`
/// rule 4: framework plumbing the system owns).
///
/// ORDERING (C4): chained LAST in the occupancy-sync maintenance chain (move → die → cover →
/// path → VISION) and `.before` the squad-fog recompute, so a tick's component change is
/// visible to the same tick's LoS/FoV recompute — the projection writes the grid directly,
/// and a same-tick `Res<OccupancyGrid>` read sees it (`bevy-traps.md` #3). It mutates the grid
/// IN PLACE; it NEVER rebuilds it.
///
/// `bevy-traps.md` #7: a normal system over `Query` / `ResMut` / `Local` — no `&mut World`.
pub fn project_vision_blocking(
    mut grid: ResMut<OccupancyGrid>,
    mut changes: VisionBlockingChanges,
    mut tracked: Local<HashMap<Entity, CellLevel>>,
) {
    // Removed first: a cell that lost AND regained the component in the same tick ends
    // OCCLUDED — the add below wins, matching the component's final state.
    for entity in changes.removed.read() {
        if let Some(cell) = tracked.remove(&entity) {
            grid.clear_vision_blocking(cell);
        }
    }
    for (entity, cell, blocks) in &changes.changed {
        grid.set_vision_blocking(**cell, **blocks);
        tracked.insert(entity, **cell);
    }
}
