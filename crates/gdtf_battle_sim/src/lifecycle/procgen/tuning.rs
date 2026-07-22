//! The **procgen fill tuning** — the three hot-reloadable knobs the GTW-427 random
//! same-theme fill pass reads (OQ-6).
//!
//! Per OQ-6 (RULED default), the fill pass's three magnitudes live in a
//! `assets/core_tuning/procgen.tuning.ron` asset rather than as scattered Rust consts, so
//! the fill density / large-prefab threshold / dead-rect scatter cap tune WITHOUT a
//! rebuild — the same hot-reloadable `RonAsset<T>` convention the combat / pan / fx tuning
//! tables use (`docs/`-locked tuning convention; memory: *hot-reload-runtime-data*).
//!
//! Every magnitude here is a named newtype (no bare `f32`/`u32`/`u8` field, no-bare-types
//! rule), each with a private inner + a derived [`Deref`](bevy::prelude::Deref) and
//! `#[serde(transparent)]` so it round-trips as a bare RON scalar. The values are
//! **tunable** balance data — tests assert only parse / round-trip / consistency, NEVER a
//! shipped magnitude (the loader-test rule, memory: *ron-serde-loading-conventions*).

use bevy::{prelude::Resource, reflect::TypePath};
use serde::Deserialize;

use super::geometry::CellCount;

/// The per-rect **scatter count** as a loop bound — [`DeadRectScatterCount`] widened to
/// `usize` for the bounded `0..k` scatter loop the fill pass runs over a dead rect.
///
/// A named newtype over `usize` (no-bare-types: the loop bound is a domain value, not a
/// bare index) — the `usize` companion to the `u8`-inner [`DeadRectScatterCount`].
/// Private inner + derived [`Deref`](bevy::prelude::Deref).
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScatterCount(usize);

impl ScatterCount {
    /// Build a scatter loop count from its value.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// The **minimum density floor** — the cell-coverage FRACTION (placed-prefab cells over
/// total board cells) the fill pass aims to reach before it considers the level "full
/// enough" and stops drawing more fill prefabs (OQ-6).
///
/// A dimensionless fraction in `0.0..=1.0`: `0.0` would place no fill at all, `1.0` would
/// pack until literally nothing more fits. The fill loop stops EITHER when coverage reaches
/// this floor OR when no further prefab fits (C1/C2) — whichever comes first — so this is a
/// SOFT target, never a guarantee (a board may not admit enough prefabs to reach it).
///
/// A distinct domain concept ⇒ its own newtype (no-bare-types rule 3): a coverage fraction,
/// never a bare `f32`. Private inner + derived [`Deref`](bevy::prelude::Deref);
/// `#[serde(transparent)]` parses a bare RON scalar. **Tunable** — tests assert only its
/// relation / parse, never the magnitude.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MinDensityFloor(f32);

impl MinDensityFloor {
    /// Build a density floor from its coverage fraction (tunable balance data).
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

impl Default for MinDensityFloor {
    fn default() -> Self {
        // 0.45 — a defensible STARTING POINT: enough fill that the connective interior
        // reads as a built level (not two lonely deployment pads on bare floor), while
        // leaving ample open `default_floor` lanes for movement / LOS. Tunable balance
        // data; value-agnostic tests only, never a pinned magnitude.
        Self(0.45)
    }
}

/// The **maximum coverage cap** — the cell-coverage FRACTION (placed-prefab cells over total
/// board cells) at or above which the fill pass STOPS drawing more prefabs, even if more would
/// fit and the density floor has not yet been reached (GTW-767).
///
/// The upper-bound COMPANION to [`MinDensityFloor`]: the floor is the coverage the fill AIMS
/// to reach; this cap is the coverage the fill must NOT exceed. The fill loop stops as soon as
/// EITHER coverage reaches the floor OR coverage reaches this cap OR no further prefab fits —
/// whichever comes first — so the level always keeps some open floor even where the space would
/// admit still more fill. A dimensionless fraction in `0.0..=1.0`: keep it ABOVE the floor (a
/// cap at or below the floor makes the floor dead, since the cap always wins first) and below
/// `1.0` (a cap of `1.0` never binds, so the fill runs to exhaustion exactly as it did before
/// this knob existed).
///
/// A distinct domain concept ⇒ its own newtype (no-bare-types rule 3): a coverage-cap fraction,
/// never a bare `f32`, and distinct from the [`MinDensityFloor`] it sits above. Private inner +
/// derived [`Deref`](bevy::prelude::Deref); `#[serde(transparent)]` parses a bare RON scalar.
/// **Tunable** — tests assert only its relation / parse, never the magnitude.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct MaxCoverageCap(f32);

impl MaxCoverageCap {
    /// Build a coverage cap from its coverage fraction (tunable balance data).
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction)
    }
}

impl Default for MaxCoverageCap {
    fn default() -> Self {
        // 0.85 — a defensible STARTING POINT set ABOVE the 0.45 density floor: 0.40 of real
        // headroom for the fill to build well past the floor target on a spacious board before
        // the cap bites, yet low enough that a densely packable board still keeps roughly the
        // top ~15% of its cells as open `default_floor` lanes for movement / LOS rather than
        // packing wall-to-wall. Tunable balance data; value-agnostic tests only, never a pinned
        // magnitude.
        Self(0.85)
    }
}

/// The **large-prefab area threshold** — the footprint AREA (in cells, width × height)
/// at or above which a fill prefab is treated as "large" for the `FillLarge` pass that runs
/// before the smaller-prefab fill (OQ-6).
///
/// The fill pass places large fill prefabs FIRST (they need the biggest contiguous free
/// rectangles, which exist early before the space fragments), then smaller ones, then
/// scatters micro-pieces into the leftover dead rects. This threshold is the area cut-off
/// between the two prefab passes.
///
/// A distinct domain concept ⇒ its own newtype (no-bare-types rule): a cell-area cut-off,
/// never a bare `u32`. Private inner + derived [`Deref`](bevy::prelude::Deref);
/// `#[serde(transparent)]` parses a bare RON scalar. **Tunable** — tests assert only its
/// relation / parse, never the magnitude.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LargePrefabAreaThreshold(u32);

impl LargePrefabAreaThreshold {
    /// Build a large-prefab area threshold from its cell-area cut-off (tunable balance
    /// data).
    #[must_use]
    pub const fn new(area: u32) -> Self {
        Self(area)
    }

    /// This threshold as an `i64` cell area — for the integer area comparison the fill
    /// pass does against a footprint's `width * height`.
    #[must_use]
    pub const fn area(self) -> CellCount {
        CellCount::new(self.0 as i64)
    }
}

impl Default for LargePrefabAreaThreshold {
    fn default() -> Self {
        // 64 cells (an ~8x8 footprint) — a defensible STARTING POINT for "large enough to
        // want the big early free rectangles". Tunable balance data; value-agnostic tests
        // only, never a pinned magnitude.
        Self(64)
    }
}

/// The **dead-rect scatter count** `k` — the MAXIMUM number of micro-piece scatter
/// prefabs the fill pass may drop into any one leftover "dead" free rectangle of `>= 4x4`
/// cells (OQ-6, `dead_rect_scatter_count_k`).
///
/// After the large + smaller fill passes, sizeable free rectangles can remain. Rather than
/// leave them as one big empty hall, the pass scatters up to `k` small fill micro-pieces
/// into each `>= 4x4` dead rect (drawn from the same `(theme, Fill)` bucket via
/// [`ProcgenRng`](crate::rng::ProcgenRng)) to break up the open space — the rest stays
/// open `default_floor` lanes (the no-fit fallback never shrinks the playable area).
///
/// A distinct domain concept ⇒ its own newtype (no-bare-types rule): a per-rect scatter
/// cap, never a bare `u8`. Private inner + derived [`Deref`](bevy::prelude::Deref);
/// `#[serde(transparent)]` parses a bare RON scalar. **Tunable** — tests assert only its
/// relation / parse, never the magnitude.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct DeadRectScatterCount(u8);

impl DeadRectScatterCount {
    /// Build a dead-rect scatter cap from its per-rect micro-piece maximum (tunable
    /// balance data).
    #[must_use]
    pub const fn new(k: u8) -> Self {
        Self(k)
    }

    /// This cap as a `usize` — for the bounded scatter loop the fill pass runs over a
    /// dead rect.
    #[must_use]
    pub const fn count(self) -> ScatterCount {
        ScatterCount::new(self.0 as usize)
    }
}

impl Default for DeadRectScatterCount {
    fn default() -> Self {
        // 3 micro-pieces per dead rect — a defensible STARTING POINT: enough to break up a
        // big empty hall without clogging it. Tunable balance data; value-agnostic tests
        // only, never a pinned magnitude.
        Self(3)
    }
}

/// The **procgen fill tuning** resource — the three OQ-6 hot-reloadable knobs the GTW-427
/// random same-theme fill pass reads.
///
/// A named [`Resource`] (no-bare-types: a tuning store is a domain value) deserializable
/// from `assets/core_tuning/procgen.tuning.ron` (the [`CombatTuning`](crate::tuning::CombatTuning)
/// precedent — it is BOTH the `Deserialize` payload AND the runtime `Resource`). Derives
/// [`TypePath`] because the `RonAsset<ProcgenTuning>` loader's payload bound requires it
/// (the same bound `CombatTuning` / the theme spec carry). `#[serde(default)]` so a file
/// may omit any field and fall back to the shipped [`Default`] — tune one number without
/// restating the rest.
///
/// The magnitudes are **tunable** balance data; tests assert only parse / round-trip /
/// consistency, never a shipped magnitude (the loader-test rule).
#[derive(Debug, Clone, Copy, PartialEq, Default, Resource, Deserialize, TypePath)]
#[serde(default)]
pub struct ProcgenTuning {
    /// The minimum cell-coverage fraction the fill pass aims to reach (C1 termination
    /// target).
    pub min_density_floor:           MinDensityFloor,
    /// The maximum cell-coverage fraction the fill pass may reach before it stops drawing
    /// more prefabs (the GTW-767 upper bound; the fill stops at the floor, this cap, or when
    /// nothing more fits — whichever comes first).
    pub max_coverage_cap:            MaxCoverageCap,
    /// The footprint-area cut-off above which a fill prefab is "large" (the `FillLarge`
    /// pass).
    pub large_prefab_area_threshold: LargePrefabAreaThreshold,
    /// The maximum micro-piece scatter prefabs dropped into any one `>= 4x4` dead rect.
    pub dead_rect_scatter_count_k:   DeadRectScatterCount,
}
