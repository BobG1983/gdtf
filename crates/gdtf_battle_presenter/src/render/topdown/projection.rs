//! The px/coordinate projection: [`CELL_PX`], the within-storey draw [`Layer`]s, and
//! the sim→world mappings ([`cell_to_world`] / [`sim_pos_to_world`] /
//! [`cell_to_world_layered`]).

use bevy::prelude::*;
use gdtf_battle_sim::prelude::{Cell, Level, SimPos};

/// On-screen size of one cell, in world units.
///
/// The ONE presenter source of truth for cell size. A `const`, NOT a domain newtype
/// — the framework-plumbing carve-out (`.claude/rules/no-bare-types.md` clause 4):
/// a scalar fed straight to a [`Transform`] / `custom_size`, not a domain quantity,
/// the same reasoning the landed `WORLD_RENDER_LAYER`-class consts use. 16.0 because
/// the source tiles are 16×16 px, so one source tile maps to a 16-world-unit cell.
pub const CELL_PX: f32 = 16.0;

/// Per-level world-space draw-z spacing, in world units.
///
/// A small monotonic gap between storeys so sprites on different levels do not
/// z-fight. Full multi-level z-stacking is GTW-49 / GTW-10; this slice only needs a
/// stable per-level z for the projection.
const Z_PER_LEVEL: f32 = 1.0;

/// The within-storey draw-z bias that lifts a ganger sprite ABOVE its own floor tile.
///
/// A `const`, NOT a domain newtype — the `CELL_PX`-class framework-plumbing carve-out
/// (`.claude/rules/no-bare-types.md` clause 4): a scalar fed straight to a
/// [`Transform`]'s `z`, not a domain quantity. Strictly `< Z_PER_LEVEL` (`0.1 < 1.0`)
/// so a ganger's lifted z never sorts into the NEXT storey's band — it stays within its
/// own storey, just in front of the same-cell terrain (which draws at the bare level z,
/// [`Layer::Terrain`] = `0.0`). This fixes the GTW-283 occlusion: at storey 0 the ganger
/// and its floor both projected to `z = 0.0`, and Bevy 0.18's non-deterministic same-z 2D
/// sort let the opaque floor draw over the ganger. `0.1` is the smallest legible lift.
pub const GANGER_Z_BIAS: f32 = 0.1;

/// A presenter draw layer within a single storey — the ONE place the
/// terrain &lt; field &lt; vertical-link &lt; fire-target &lt; actor &lt; cross-level-signal &lt;
/// highlight &lt; reachable-range &lt; path-preview stacking order lives.
///
/// A domain value (a real named type, not a bare z magnitude), per
/// `.claude/rules/no-bare-types.md`. Each layer's [`z_bias`](Layer::z_bias) is added on
/// top of the per-storey level z by [`cell_to_world_layered`] so a sprite draws in front
/// of the lower layers at its own cell without crossing into the next storey's band (every
/// bias is strictly `< Z_PER_LEVEL`). This slice wires [`Terrain`](Layer::Terrain) (the
/// bare level z, via [`cell_to_world`]) and [`Actor`](Layer::Actor) (the
/// [`GANGER_Z_BIAS`] lift); [`Highlight`](Layer::Highlight) is defined so the documented
/// order is complete, but routing the hover/selection highlight through it is the
/// in-engine-adjustable later tweak the GTW-283 contract flags (it currently still draws
/// at the bare level z). [`PathPreview`](Layer::PathPreview) is the GTW-358 route-preview
/// band — the topmost within-storey band, drawn ABOVE the highlight so the previewed route
/// (and its target-cell TU-cost label, GTW-368) reads over the terrain, actors, and reticle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Floor / wall / cover terrain — the ground plane, drawn at the bare per-storey z.
    Terrain,
    /// The GTW-545 area-damage-field wash — a translucent hazard tile drawn OVER the terrain
    /// floor of a fielded cell (a toxic pool / electrified floor / burning ground), so the
    /// danger zone reads over its own floor tile. Drawn just ABOVE the terrain ground plane but
    /// strictly BELOW the [`VerticalLink`](Layer::VerticalLink) / [`Actor`](Layer::Actor) bands
    /// so a ganger (or a stair) standing on the field draws OVER the wash — it is a ground-plane
    /// hazard the unit stands in, not something that occludes the unit. Strictly `< GANGER_Z_BIAS
    /// < Z_PER_LEVEL`, so it never sorts into the actor band or the next storey's band.
    Field,
    /// A GTW-359 vertical-link (stair / ladder) endpoint tile — drawn just ABOVE the
    /// terrain ground plane (so the stair / ladder reads over its own floor tile) but
    /// strictly BELOW the [`Actor`](Layer::Actor) band so a ganger standing on the link
    /// cell draws over it. Strictly `< GANGER_Z_BIAS < Z_PER_LEVEL`, so it never sorts
    /// into the actor band or the next storey's band.
    VerticalLink,
    /// The GTW-371 fire-target highlight — the RED tile drawn UNDER the enemy a fireable
    /// hover would shoot. Drawn ABOVE the terrain + vertical-link bands but strictly BELOW
    /// the [`Actor`](Layer::Actor) band (`GANGER_Z_BIAS * 0.75 = 0.075 < GANGER_Z_BIAS`),
    /// so it renders UNDER the enemy's own sprite — the contract's "under the actor"
    /// treatment, exactly the way the vertical-link tile sits under a ganger standing on it.
    FireTarget,
    /// A ganger (actor) — drawn just in front of its own terrain by [`GANGER_Z_BIAS`].
    Actor,
    /// The GTW-596 cross-level tactical badges — compact corner signals (threats
    /// above/below, hole/ledge drop depth, stair/ladder connector deltas) drawn ON the
    /// active storey. Strictly ABOVE the actor band (so a badge reads over a ganger
    /// standing at that cell) but strictly BELOW the [`Highlight`](Layer::Highlight) band
    /// (so it never competes with the hover/selection highlight).
    CrossLevelSignal,
    /// The hover / selection highlight — drawn in front of the actor so it tints the unit
    /// (documented order; wiring deferred, see the type doc).
    Highlight,
    /// The GTW-387 reachable-range overlay — the cells the selected ganger can reach
    /// within its remaining TU. Drawn ABOVE the highlight so the range tint reads over the
    /// terrain and actors, but strictly BELOW the [`PathPreview`](Layer::PathPreview) so
    /// the move-route still reads over the range highlight when both are shown.
    ReachableRange,
    /// The GTW-358 route-preview highlight — the previewed `find_path` route from the
    /// selected ganger to the target cell (and the GTW-368 target-cell TU-cost label), drawn
    /// strictly ABOVE the highlight so the route + its cost label read over the terrain,
    /// actors, and reticle. The topmost within-storey band — still strictly `< Z_PER_LEVEL`
    /// so it never sorts into the next storey's band.
    PathPreview,
}

impl Layer {
    /// This layer's within-storey draw-z bias, added on top of the per-storey level z.
    ///
    /// Strictly increasing terrain &lt; field &lt; vertical-link &lt; fire-target &lt; actor &lt;
    /// cross-level-signal &lt; highlight &lt; reachable-range &lt; path-preview, and every value
    /// is strictly `< Z_PER_LEVEL` so a biased sprite never sorts into the next storey's band.
    #[must_use]
    const fn z_bias(self) -> f32 {
        match self {
            Self::Terrain => 0.0,
            // GTW-545: the field hazard wash — just above the terrain floor (reads over its own
            // floor tile), strictly below every other band so the ganger / stair standing IN the
            // field draws over the wash (`0.025 < GANGER_Z_BIAS`).
            Self::Field => GANGER_Z_BIAS * 0.25,
            // Above the terrain ground plane, strictly below the actor band so a ganger
            // standing on the stair / ladder draws over it (`0.05 < GANGER_Z_BIAS`).
            Self::VerticalLink => GANGER_Z_BIAS * 0.5,
            // GTW-371: above the terrain / vertical-link bands, strictly below the actor band
            // so the red fire-target tile renders UNDER the enemy's sprite
            // (`0.075 < GANGER_Z_BIAS`).
            Self::FireTarget => GANGER_Z_BIAS * 0.75,
            Self::Actor => GANGER_Z_BIAS,
            // GTW-596: strictly above the actor, strictly below the highlight
            // (`0.15` sits between `Actor`'s `GANGER_Z_BIAS` (`0.1`) and `Highlight`'s `0.2`).
            Self::CrossLevelSignal => GANGER_Z_BIAS * 1.5,
            // Strictly above the actor, still within the storey band (`< Z_PER_LEVEL`).
            Self::Highlight => GANGER_Z_BIAS * 2.0,
            // GTW-387: above the highlight, below the route-preview so the move-route reads
            // over the range highlight when both are shown.
            Self::ReachableRange => GANGER_Z_BIAS * 2.5,
            // Topmost within-storey band — above the highlight + range overlay (the route +
            // its cost label read over the reticle and range tint), still `< Z_PER_LEVEL`.
            Self::PathPreview => GANGER_Z_BIAS * 3.0,
        }
    }
}

/// Projects a sim cell + level into the top-down renderer's world-space position.
///
/// Row 0 sits at the TOP: Bevy's +Y is up, so a larger `cell.y` (further down the
/// grid) yields a smaller world `y`. `x` grows right by exactly [`CELL_PX`] per cell.
/// The `z` is a stable per-level draw-z ([`z_for`]) so sprites on different storeys
/// do not z-fight; it is NOT full multi-level stacking (GTW-49 / GTW-10).
///
/// `cell.x` / `cell.y` read through [`Cell`]'s `Deref<Target = IVec2>`; the level
/// index reads through [`Level`]'s `Deref<Target = u8>` inside [`z_for`].
#[must_use]
pub fn cell_to_world(cell: Cell, level: Level) -> Vec3 {
    Vec3::new(
        cell.x as f32 * CELL_PX,
        -(cell.y as f32) * CELL_PX,
        z_for(level),
    )
}

/// Projects a CONTINUOUS sim-unit position ([`SimPos`]) into the top-down renderer's
/// world-space — the [`cell_to_world`] projection generalised to a fractional point.
///
/// Same mapping as [`cell_to_world`] (one sim unit = one [`CELL_PX`] cell; +Y is up, so a
/// larger `pos.y` yields a smaller world `y`), but for a continuous point rather than a
/// discrete `(cell, level)`: the GTW-290 muzzle origin is a [`SimPos`] (a fractional 3D
/// fire point), so the muzzle flash + tracer origin map through this. The `z` scales the
/// fractional storey by [`Z_PER_LEVEL`] (matching [`z_for`]'s discrete `*level *
/// Z_PER_LEVEL`) so a muzzle on storey *n* draws in that storey's band.
///
/// `pos.x` / `pos.y` / `pos.z` read through [`SimPos`]'s `Deref<Target = Vec3>`.
#[must_use]
pub fn sim_pos_to_world(pos: SimPos) -> Vec3 {
    Vec3::new(pos.x * CELL_PX, -pos.y * CELL_PX, pos.z * Z_PER_LEVEL)
}

/// Projects a sim cell + level into world-space, lifted by `layer`'s within-storey
/// draw-z bias — the [`cell_to_world`] position with [`Layer::z_bias`] added to `z`.
///
/// The ONE place a presenter draws a sprite "in front of" the lower layers at the same
/// cell: [`Terrain`](Layer::Terrain) sits at the bare per-storey z (equivalent to
/// [`cell_to_world`]), [`Actor`](Layer::Actor) is lifted by [`GANGER_Z_BIAS`] so a ganger
/// draws over its own floor tile (GTW-283), and the lift never crosses into the next
/// storey (every bias is strictly `< Z_PER_LEVEL`). `x` / `y` are unchanged from
/// [`cell_to_world`].
#[must_use]
pub fn cell_to_world_layered(cell: Cell, level: Level, layer: Layer) -> Vec3 {
    let mut world = cell_to_world(cell, level);
    world.z += layer.z_bias();
    world
}

/// The world-space draw-z for a storey `level`.
///
/// Monotonic in the storey index so higher storeys draw in front: `*level` (read
/// through [`Level`]'s `Deref<Target = u8>`) scaled by [`Z_PER_LEVEL`]. Kept private
/// — callers use [`cell_to_world`].
pub(super) fn z_for(level: Level) -> f32 {
    f32::from(*level) * Z_PER_LEVEL
}
