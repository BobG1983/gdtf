//! The §2 projectile travel — the **true 3-axis voxel DDA** (`march_vector`) that
//! flies one 3D shot ray through the 60×60×8 cubic-voxel grid and reports the first
//! thing the round fails to clear (`docs/combat/resolution.md` §2 + §3;
//! `docs/combat/battle-space.md` §"The shot is one 3D Vec3, ray-marched by voxel
//! DDA").
//!
//! This is the E2.7 slice. The shot is **ONE 3D `Vec3`** — a continuous muzzle
//! [`SimPos`] plus a unit-`Vec3` direction — marched as an **Amanatides–Woo** voxel
//! DDA in **sim units** (one cell on x = one cell on y = one level on z = 1.0):
//! [`march_vector`] walks the grid cell-by-cell and level-by-level, never a
//! pixel-stepped ray, and at each thing it crosses it applies the E2.6 clearance
//! rule (`docs/combat/resolution.md` §2: **strictly-higher SAILS OVER /
//! equal-or-lower IMPACTS**). **Zero pixels.**
//!
//! What the march tests, in order, as it walks the ray.
//!
//! **Occupied `(cell, level)`** — a ganger occupant ([`OccupancyGrid::occupant`]
//! plus its silhouette band from [`OccupancyGrid::occupant_band`]) OR a piece of
//! standing cover ([`crate::cover::CoverEntry`] read from [`CoverLedger`], excluded
//! when destroyed via [`OccupancyGrid::is_blocked`] / [`OccupancyGrid::is_cover_destroyed`]).
//! The round's band at the crossing is [`round_band_for_cell`] (its continuous z
//! within the crossed level), and [`round_clears_occupant`] decides: a strictly
//! higher round sails over and the march continues, an equal-or-lower round impacts
//! and the march stops. **Any** actor in the path impacts — **including the
//! shooter's own gang** (true friendly fire): the rule is band-vs-band, with no
//! exemption list.
//!
//! **Z-boundary crossings** — when the DDA steps across a storey boundary, the
//! floor/roof [`SurfaceGrid`] slab is tested (one slab, both faces): a
//! [`SlabState::Present`] slab stops the round; [`SlabState::Destroyed`] /
//! [`SlabState::Absent`] passes.
//!
//! **Grid exits** — leaving the **top** is a clean sky [`MarchKind::Miss`]; leaving
//! the **bottom** strikes the [`MarchKind::Ground`] (damaged, never destroyed);
//! leaving **laterally** is a [`MarchKind::Miss`].
//!
//! There is **NO target stop** — the round flies past the aim cell into whatever is
//! behind it; a "miss" is just a shot whose deviation carried it past everything.
//! The **only** hard exception is that the **shooter's own cell never blocks its own
//! shot** (`docs/combat/resolution.md` §2). Degenerate marches (a direction that
//! leaves the grid immediately, a zero direction) are graceful — a [`MarchKind::Miss`],
//! never a panic.

use bevy::{math::Vec3, prelude::Entity};

use crate::{
    clearance::{Clearance, round_band_for_cell, round_clears_occupant},
    cover::{CoverEntry, CoverLedger, HeightBand},
    metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, pos_to_cell},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    tuning::CombatTuning,
};

/// What the round **failed to clear** — the kind of thing the march stopped on, with
/// the payload that identifies it (`docs/combat/resolution.md` §2: "the first thing
/// the round fails to clear: (ganger | cover | floor/roof slab | ground) — or a
/// clean miss off the grid").
///
/// A named domain enum (no-bare-types: the march verdict is a domain value, not a
/// bare tag) carrying the struck object itself where there is one — a ganger's
/// [`Entity`] handle (NEVER a numeric id — GTW-10 / GTW-12) and the cover's
/// [`CoverEntry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarchKind {
    /// The round impacted a **ganger** occupant — carries its [`Entity`] handle (the
    /// struck actor; never a numeric id).
    Ganger(Entity),
    /// The round impacted a piece of **cover** — carries the [`CoverEntry`] read from
    /// the [`CoverLedger`] for the struck `(cell, level)`.
    Cover(CoverEntry),
    /// The round was stopped by an intact floor/roof **slab** at a z-boundary.
    Slab,
    /// The round left the **bottom** of the grid and struck the ground (damaged,
    /// never destroyed — crater FX later).
    Ground,
    /// A clean **miss** — the round left the grid off the top or laterally without
    /// failing to clear anything.
    Miss,
}

/// The outcome of a [`march_vector`] — the first thing the round fails to clear, the
/// `(cell, level)` it happened at, the crossed-cell [`HeightBand`] of the round, and
/// the impact point (`docs/combat/resolution.md` §2).
///
/// Every field is a named domain value (no-bare-types): the [`MarchKind`] verdict
/// (with its struck-object payload), the [`CellLevel`] grid key, the round's
/// [`HeightBand`] at the crossing, and the [`SimPos`] impact point — a continuous
/// sim-unit position built via [`SimPos::new`] (AC #7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarchResult {
    /// What the round failed to clear (with the struck ganger / cover payload).
    pub kind:   MarchKind,
    /// The `(cell, level)` the round failed to clear at (the exit cell for a
    /// ground / lateral / top result).
    pub at:     CellLevel,
    /// The round's clearance band at the crossed cell ([`round_band_for_cell`]).
    pub band:   HeightBand,
    /// The impact point in sim units — where along the ray the round stopped (or
    /// exited), built via [`SimPos::new`].
    pub impact: SimPos,
}

/// A generous per-march iteration cap — far above the longest possible voxel walk
/// (the grid's `60 + 60 + 8` voxel diameter, plus slack), so a degenerate ray can
/// never spin forever while a legitimate corner-to-corner shot always completes.
const MAX_STEPS: u32 = (GRID_WIDTH + GRID_HEIGHT + MAX_LEVELS as usize) as u32 * 4;

/// Per-axis DDA traversal state for one ground/level axis (Amanatides–Woo).
#[derive(Debug, Clone, Copy)]
struct AxisDda {
    /// The integer voxel index on this axis (a cell coord on x/y, a storey on z).
    index:   i32,
    /// `+1` / `-1` step direction, or `0` when the ray does not move on this axis.
    step:    i32,
    /// The ray parameter `t` at which the ray next crosses a voxel boundary on this
    /// axis; `f32::INFINITY` when the ray does not move on this axis.
    t_max:   f32,
    /// The `t` increment to cross one whole voxel on this axis; `f32::INFINITY` when
    /// the ray does not move on this axis.
    t_delta: f32,
}

impl AxisDda {
    /// Initialise the DDA for one axis from the ray's origin and direction
    /// components on that axis. `origin` is the continuous sim-unit start, `dir` the
    /// direction component, and `index` the integer voxel the origin floors into.
    fn new(origin: f32, dir: f32, index: i32) -> Self {
        if dir == 0.0 {
            return Self {
                index,
                step: 0,
                t_max: f32::INFINITY,
                t_delta: f32::INFINITY,
            };
        }
        let step = if dir > 0.0 { 1 } else { -1 };
        let t_delta = (1.0 / dir).abs();
        // Distance (in t) from the origin to the first voxel boundary in the step
        // direction: the next integer boundary above (step +) or below (step −).
        #[expect(
            clippy::cast_precision_loss,
            reason = "voxel indices are tiny (0..60 / 0..8); the f32 conversion is exact for this range"
        )]
        let next_boundary = if step > 0 {
            (index as f32 + 1.0) - origin
        } else {
            origin - index as f32
        };
        let t_max = next_boundary.abs() / dir.abs();
        Self {
            index,
            step,
            t_max,
            t_delta,
        }
    }
}

/// The voxel axis the DDA stepped on (the smallest `t_max`) — drives whether a step
/// crosses a z-boundary (slab test) and which grid face an exit leaves by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SteppedAxis {
    /// Stepped on the x ground axis.
    X,
    /// Stepped on the y ground axis.
    Y,
    /// Stepped on the z (storey) axis — a z-boundary crossing (slab test).
    Z,
}

/// Whether `(x, y)` is inside the ground-plane grid extent.
fn xy_in_grid(x: i32, y: i32) -> bool {
    let Ok(ux) = usize::try_from(x) else {
        return false;
    };
    let Ok(uy) = usize::try_from(y) else {
        return false;
    };
    ux < GRID_WIDTH && uy < GRID_HEIGHT
}

/// Whether a storey index `z` is a valid level (`0..MAX_LEVELS`).
fn z_in_grid(z: i32) -> bool {
    z >= 0 && z < i32::from(MAX_LEVELS)
}

/// Build the `(cell, level)` key for integer voxel coords — `z` is known in-range by
/// the caller (it is checked before this is called).
fn key_of(x: i32, y: i32, z: i32) -> CellLevel {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is checked in 0..MAX_LEVELS before this is called, so the u8 cast cannot truncate or wrap"
    )]
    let level = Level::new(z as u8);
    CellLevel::new(Cell::new(x, y), level)
}

/// The sim-unit point along the ray at parameter `t` — `muzzle + t × dir`, built via
/// [`SimPos::new`] (AC #7: the impact point is a `SimPos` from `SimPos::new`).
fn point_at(muzzle: SimPos, dir: Vec3, t: f32) -> SimPos {
    let p = *muzzle + dir * t;
    SimPos::new(p.x, p.y, p.z)
}

/// March one 3D shot ray through the cubic-voxel grid and report the first thing the
/// round fails to clear (`docs/combat/resolution.md` §2 + §3) — the true 3-axis
/// voxel DDA.
///
/// The ray starts at the 3D `muzzle` [`SimPos`] and flies along `dir` (a unit-`Vec3`
/// shot direction; a non-unit `dir` is handled — only its direction matters). It
/// walks the 60×60×8 grid cell-by-cell / level-by-level in sim units (Amanatides–Woo).
///
/// At each voxel an **occupant** is banded: a ganger ([`OccupancyGrid::occupant`]
/// plus [`OccupancyGrid::occupant_band`]) or standing cover ([`CoverLedger`] entry,
/// excluded when destroyed via [`OccupancyGrid::is_blocked`] /
/// [`OccupancyGrid::is_cover_destroyed`]) is compared by [`round_clears_occupant`]
/// against the round's [`round_band_for_cell`] band — an equal-or-lower round impacts
/// ([`MarchKind::Ganger`] / [`MarchKind::Cover`]), a strictly higher round sails
/// over. **Any** actor impacts — including the shooter's own gang (true friendly
/// fire) — by band alone, no exemption list. Each **z-boundary** step tests the
/// [`SurfaceGrid`] slab (one slab, both faces): [`SlabState::Present`] stops the
/// round ([`MarchKind::Slab`]); `Destroyed` / `Absent` passes. A **grid exit** is
/// top → clean sky [`MarchKind::Miss`]; bottom → [`MarchKind::Ground`]; lateral →
/// [`MarchKind::Miss`].
///
/// **No target stop** — the round flies past the aim cell into whatever is behind it.
/// The **only** exception: the `shooter_cell` never blocks its own shot (its
/// occupant / cover is skipped). Degenerate marches (a `dir` that leaves the grid
/// immediately, a zero `dir`, an out-of-grid `muzzle`) return a [`MarchKind::Miss`]
/// gracefully — never a panic (AC #7). Every band read comes from `tuning` /
/// `cover` / `occupancy` / `surface` (no hardcoded grid or band constant beyond the
/// structural [`GRID_WIDTH`] / [`GRID_HEIGHT`] / [`MAX_LEVELS`]).
#[must_use]
pub fn march_vector(
    muzzle: SimPos,
    dir: Vec3,
    occupancy: &OccupancyGrid,
    surface: &SurfaceGrid,
    cover: &CoverLedger,
    tuning: &CombatTuning,
    shooter_cell: CellLevel,
) -> MarchResult {
    let (start_cell, start_level) = pos_to_cell(muzzle);
    let mut state = MarchState::new(
        start_cell.x,
        start_cell.y,
        i32::from(*start_level),
        muzzle,
        dir,
    );

    // Degenerate / out-of-grid start, or a zero direction: a graceful Miss at the
    // muzzle, no panic (AC #7).
    if dir == Vec3::ZERO || !xy_in_grid(state.vx, state.vy) || !z_in_grid(state.vz) {
        return MarchResult {
            kind:   MarchKind::Miss,
            at:     key_of_clamped(state.vx, state.vy, state.vz),
            band:   round_band_for_cell(muzzle, tuning),
            impact: muzzle,
        };
    }

    for _ in 0..MAX_STEPS {
        let here = key_of(state.vx, state.vy, state.vz);
        // The round's position as it sits in this voxel (its entry point) bands it at
        // this crossing (AC #1, #2). The shooter's own cell never blocks its own shot
        // (the one hard exception, AC #6) — skip the occupant / cover checks there.
        let here_point = point_at(muzzle, dir, state.entry_t);
        let round_band = round_band_for_cell(here_point, tuning);
        if here != shooter_cell
            && let Some(result) = impact_at(here, here_point, round_band, occupancy, cover)
        {
            return result;
        }

        // Step to the next voxel; an exit / intact slab ends the march here.
        match state.advance(muzzle, dir, surface, tuning, round_band) {
            Step::Continue => {}
            Step::Stopped(result) => return result,
        }
    }

    // The iteration cap was hit (only reachable for a pathological ray) — degrade to a
    // graceful Miss rather than spin (AC #7).
    let here_point = point_at(muzzle, dir, state.entry_t);
    MarchResult {
        kind:   MarchKind::Miss,
        at:     key_of_clamped(state.vx, state.vy, state.vz),
        band:   round_band_for_cell(here_point, tuning),
        impact: here_point,
    }
}

/// The occupant impact at one crossed voxel, or `None` if the round clears
/// everything there — a ganger ([`MarchKind::Ganger`]) checked first, then standing
/// cover ([`MarchKind::Cover`], destroyed cover excluded). The clearance is
/// band-vs-band ([`round_clears_occupant`]): equal-or-lower impacts, strictly-higher
/// clears. **Any** actor impacts, including the shooter's own gang (true friendly
/// fire — AC #2, #6). `here_point` is the round's position in the voxel; `round_band`
/// is its band there.
fn impact_at(
    here: CellLevel,
    here_point: SimPos,
    round_band: HeightBand,
    occupancy: &OccupancyGrid,
    cover: &CoverLedger,
) -> Option<MarchResult> {
    // 1. A ganger occupant, banded by its silhouette band off the grid.
    if let (Some(entity), Some(occ_band)) =
        (occupancy.occupant(&here), occupancy.occupant_band(&here))
        && round_clears_occupant(round_band, occ_band) == Clearance::Impacts
    {
        return Some(MarchResult {
            kind:   MarchKind::Ganger(entity),
            at:     here,
            band:   round_band,
            impact: here_point,
        });
    }
    // 2. Standing cover, banded by its CoverEntry band, excluded when destroyed (AC #3).
    if !occupancy.is_cover_destroyed(&here)
        && let Some(entry) = cover.peek(&here)
        && !*entry.destroyed
        && round_clears_occupant(round_band, entry.height_band) == Clearance::Impacts
    {
        return Some(MarchResult {
            kind:   MarchKind::Cover(*entry),
            at:     here,
            band:   round_band,
            impact: here_point,
        });
    }
    None
}

/// The result of advancing the DDA one voxel — continue marching, or stop on a
/// terminal [`MarchResult`] (an intact slab, or a grid exit).
enum Step {
    /// The ray stepped into the next in-grid voxel; keep marching.
    Continue,
    /// The march ended on this step (slab / top / bottom / lateral exit).
    Stopped(MarchResult),
}

/// The mutable voxel-walk state of a march — the current integer voxel, the three
/// per-axis DDA traversals, and the `t` the ray entered the current voxel at.
struct MarchState {
    /// Current voxel x (cell coord).
    vx:      i32,
    /// Current voxel y (cell coord).
    vy:      i32,
    /// Current voxel z (storey index).
    vz:      i32,
    /// The x-axis DDA traversal.
    ax:      AxisDda,
    /// The y-axis DDA traversal.
    ay:      AxisDda,
    /// The z-axis DDA traversal.
    az:      AxisDda,
    /// The ray parameter `t` at which the ray entered the current voxel (0 at the
    /// muzzle's own voxel).
    entry_t: f32,
}

impl MarchState {
    /// Initialise the walk at integer voxel `(vx, vy, vz)` for the ray `muzzle + t × dir`.
    fn new(vx: i32, vy: i32, vz: i32, muzzle: SimPos, dir: Vec3) -> Self {
        Self {
            vx,
            vy,
            vz,
            ax: AxisDda::new(muzzle.x, dir.x, vx),
            ay: AxisDda::new(muzzle.y, dir.y, vy),
            az: AxisDda::new(muzzle.z, dir.z, vz),
            entry_t: 0.0,
        }
    }

    /// Step to the next voxel along the smallest-`t_max` axis (Amanatides–Woo),
    /// testing the slab on a z-crossing and the grid faces on an exit. Returns
    /// [`Step::Stopped`] on an intact slab or a grid exit, else [`Step::Continue`]
    /// after advancing into the next in-grid voxel.
    fn advance(
        &mut self,
        muzzle: SimPos,
        dir: Vec3,
        surface: &SurfaceGrid,
        tuning: &CombatTuning,
        round_band: HeightBand,
    ) -> Step {
        let stepped = step_axis(&self.ax, &self.ay, &self.az);
        let step_t = match stepped {
            SteppedAxis::X => self.ax.t_max,
            SteppedAxis::Y => self.ay.t_max,
            SteppedAxis::Z => self.az.t_max,
        };
        let crossing = point_at(muzzle, dir, step_t);
        self.entry_t = step_t;

        match stepped {
            SteppedAxis::X => {
                self.vx += self.ax.step;
                self.ax.index = self.vx;
                self.ax.t_max += self.ax.t_delta;
                if xy_in_grid(self.vx, self.vy) {
                    Step::Continue
                } else {
                    Step::Stopped(lateral_miss(
                        crossing, self.vx, self.vy, self.vz, round_band,
                    ))
                }
            }
            SteppedAxis::Y => {
                self.vy += self.ay.step;
                self.ay.index = self.vy;
                self.ay.t_max += self.ay.t_delta;
                if xy_in_grid(self.vx, self.vy) {
                    Step::Continue
                } else {
                    Step::Stopped(lateral_miss(
                        crossing, self.vx, self.vy, self.vz, round_band,
                    ))
                }
            }
            SteppedAxis::Z => self.advance_z(crossing, surface, tuning),
        }
    }

    /// The z-axis half of [`advance`](MarchState::advance): test the floor/roof slab
    /// at the boundary, then step the storey and handle the top / bottom exits.
    fn advance_z(
        &mut self,
        crossing: SimPos,
        surface: &SurfaceGrid,
        tuning: &CombatTuning,
    ) -> Step {
        // The slab between the two storeys (one slab, both faces) is keyed at the
        // UPPER of the two levels — the floor of the upper storey / roof of the lower.
        let upper_level = self.vz.max(self.vz + self.az.step);
        if z_in_grid(upper_level) {
            let slab_key = key_of(self.vx, self.vy, upper_level);
            if surface.slab_state(&slab_key) == SlabState::Present {
                // An intact slab stops the round at the boundary (AC #4).
                return Step::Stopped(MarchResult {
                    kind:   MarchKind::Slab,
                    at:     slab_key,
                    band:   round_band_for_cell(crossing, tuning),
                    impact: crossing,
                });
            }
        }
        self.vz += self.az.step;
        self.az.index = self.vz;
        self.az.t_max += self.az.t_delta;
        if self.vz >= i32::from(MAX_LEVELS) {
            // Left the top — a clean sky Miss (AC #5).
            Step::Stopped(MarchResult {
                kind:   MarchKind::Miss,
                at:     key_of_clamped(self.vx, self.vy, self.vz),
                band:   round_band_for_cell(crossing, tuning),
                impact: crossing,
            })
        } else if self.vz < 0 {
            // Left the bottom — strikes the ground in the exit cell (AC #5).
            Step::Stopped(MarchResult {
                kind:   MarchKind::Ground,
                at:     key_of_clamped(self.vx, self.vy, 0),
                band:   HeightBand::Low,
                impact: crossing,
            })
        } else {
            Step::Continue
        }
    }
}

/// A lateral grid-exit [`MarchKind::Miss`] result at the crossing point.
fn lateral_miss(impact: SimPos, x: i32, y: i32, z: i32, band: HeightBand) -> MarchResult {
    MarchResult {
        kind: MarchKind::Miss,
        at: key_of_clamped(x, y, z),
        band,
        impact,
    }
}

/// Build a `(cell, level)` key, clamping the storey into `0..MAX_LEVELS` so an
/// out-of-grid `z` (an exit cell) still yields a representable key — the cell x/y is
/// recorded as-is (the key is a pure record here, never an index).
fn key_of_clamped(x: i32, y: i32, z: i32) -> CellLevel {
    let clamped = z.clamp(0, i32::from(MAX_LEVELS) - 1);
    key_of(x, y, clamped)
}

/// Choose the axis with the smallest `t_max` — the next voxel boundary the ray
/// reaches (Amanatides–Woo). Ties resolve x, then y, then z (a fixed, deterministic
/// order), so a diagonal ray walks a stable, hand-computable cell sequence.
fn step_axis(ax: &AxisDda, ay: &AxisDda, az: &AxisDda) -> SteppedAxis {
    if ax.t_max <= ay.t_max && ax.t_max <= az.t_max {
        SteppedAxis::X
    } else if ay.t_max <= az.t_max {
        SteppedAxis::Y
    } else {
        SteppedAxis::Z
    }
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::world::World, math::Vec3};

    use super::*;
    use crate::{
        armor::{ArmorHardness, ArmorProtection},
        cover::{CoverHp, HeightBand},
        metric::{Cell, CellLevel, Level, MAX_LEVELS, SimPos, cell_center},
        occupancy::{
            OccupancyGrid, OccupancyInput, OccupantPlacement, TerrainKind, TerrainPlacement,
        },
        surface::{SlabState, SurfaceGrid},
        tuning::{BandEdge, CombatTuning},
    };

    /// A `(cell, level)` key from raw coords.
    fn key(x: i32, y: i32, level: u8) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(level))
    }

    /// The continuous sim-unit center of `(x, y)` on storey `level`.
    fn center(x: i32, y: i32, level: u8) -> SimPos {
        cell_center(Cell::new(x, y), Level::new(level))
    }

    /// A `SimPos` at the center of `(x, y, level)` raised to `above_floor` within
    /// that storey — the round's z is `level + above_floor`.
    fn at_height(x: i32, y: i32, level: u8, above_floor: f32) -> SimPos {
        SimPos::new(
            x as f32 + 0.5,
            y as f32 + 0.5,
            f32::from(level) + above_floor,
        )
    }

    /// An arbitrary cover entry at `band` (NOT shipped magnitudes — the band is what
    /// the clearance test reads; the HP/armor are arbitrary).
    fn cover_entry(band: HeightBand) -> CoverEntry {
        CoverEntry::seeded(
            CoverHp::new(50),
            band,
            ArmorProtection::new(5),
            ArmorHardness::new(2),
        )
    }

    /// An above-floor fraction that classifies HIGH under the default band edges (a
    /// HIGH round) — derived from the tuning, never a literal.
    fn high_above_floor(tuning: &CombatTuning) -> f32 {
        f32::midpoint(*tuning.projectile_band_edges.mid_high, 1.0)
    }

    /// An above-floor fraction that classifies LOW under the default band edges.
    fn low_above_floor(tuning: &CombatTuning) -> f32 {
        *tuning.projectile_band_edges.low_mid * 0.5
    }

    /// A spawned `Entity` from a throwaway `World` — a real Bevy handle, never a
    /// numeric id (GTW-10 / GTW-12).
    fn spawn_entity() -> Entity {
        let mut world = World::new();
        world.spawn_empty().id()
    }

    /// A cell far off the grid so it never coincides with any real cell under test —
    /// the "no shooter cell exception in play" sentinel.
    fn far_shooter() -> CellLevel {
        key(59, 59, 7)
    }

    // --- AC #1: hand-computed cell walks, asserted cell-by-cell. ---

    /// A flat axis-aligned ray (East along +x) walks the EXACT cell sequence
    /// `(2,2,0) → (3,2,0) → (4,2,0) → …` until it leaves the grid laterally. The
    /// walk is captured by standing a HIGH wall-cover in each successive cell and a
    /// LOW round, asserting the round impacts the NEXT cell each time (since a HIGH
    /// cover stops a LOW round) — so the impact cell IS the next cell in the DDA
    /// sequence.
    #[test]
    fn flat_axis_aligned_ray_walks_expected_cell_sequence() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let muzzle = center(2, 2, 0); // (2.5, 2.5, 0.5)
        let dir = Vec3::new(1.0, 0.0, 0.0); // due East

        // For each expected next cell, stand a HIGH cover there alone and assert the
        // LOW round (the flat ray sits LOW at z = 0.5? no — z=0.5 is HIGH under
        // default edges). Use a HIGH cover so any round impacts; the flat ray's band
        // does not matter for the WALK, only that it stops at that cell.
        for next_x in 3..=8 {
            let mut cover = CoverLedger::new();
            cover.insert(key(next_x, 2, 0), cover_entry(HeightBand::High));
            let grid = OccupancyGrid::new();

            let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
            assert_eq!(
                result.at,
                key(next_x, 2, 0),
                "the flat East ray must cross cell ({next_x}, 2, 0) in sequence",
            );
            assert!(
                matches!(result.kind, MarchKind::Cover(_)),
                "the HIGH cover at ({next_x},2,0) must stop the round",
            );
            // The y / level never change on a flat East ray.
            assert_eq!(result.at.y, 2);
            assert_eq!(result.at.z, 0);
        }
    }

    /// A diagonal + climbing ray (dir = (1,1,1) from the center of (0,0,0)) walks the
    /// EXACT Amanatides–Woo staircase `(0,0,0) (1,0,0) (1,1,0) (1,1,1) (2,1,1)
    /// (2,2,1) (2,2,2) (3,2,2) (3,3,2) (3,3,3) …`. Each expected cell is probed by
    /// standing a HIGH cover ALONE in it (no other cover) and asserting the round
    /// stops there — proving that cell is on the walk and in this order.
    #[test]
    fn diagonal_climbing_ray_walks_expected_staircase() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new(); // no slabs — climbing is free
        let muzzle = center(0, 0, 0); // (0.5, 0.5, 0.5)
        let dir = Vec3::new(1.0, 1.0, 1.0);

        // The hand-computed DDA sequence (ties resolve X, then Y, then Z).
        let expected = [
            key(1, 0, 0),
            key(1, 1, 0),
            key(1, 1, 1),
            key(2, 1, 1),
            key(2, 2, 1),
            key(2, 2, 2),
            key(3, 2, 2),
            key(3, 3, 2),
            key(3, 3, 3),
        ];

        for cell in expected {
            let mut cover = CoverLedger::new();
            cover.insert(cell, cover_entry(HeightBand::High));
            let grid = OccupancyGrid::new();

            let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
            assert_eq!(
                result.at, cell,
                "the diagonal climbing ray must cross {cell:?} in DDA order",
            );
            assert!(
                matches!(result.kind, MarchKind::Cover(_)),
                "the HIGH cover at {cell:?} must stop the round",
            );
        }
    }

    // --- AC #2: ganger banding — equal-or-lower impacts, strictly-higher sails. ---

    /// A ganger occupant at an EQUAL-or-lower band impacts: a LOW round vs a LOW
    /// ganger returns `Ganger(entity)` at that cell.
    #[test]
    fn ganger_at_equal_band_impacts_returning_entity() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let entity = spawn_entity();

        let target = key(5, 2, 0);
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(target, Some(entity));
        grid.set_occupant_band(target, Some(HeightBand::Low));
        let cover = CoverLedger::new();

        // A round flying at LOW (z just above the floor) through the target cell.
        let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0); // flat East at constant low z

        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Ganger(entity),
            "a LOW round vs a LOW ganger must impact and carry the Entity",
        );
        assert_eq!(result.at, target, "the impact cell is the ganger's cell");
        assert_eq!(
            result.band,
            HeightBand::Low,
            "the round's band at the crossing"
        );
    }

    /// A strictly-higher round sails over a ganger and continues: a HIGH round vs a
    /// LOW ganger does NOT impact the ganger — it flies past (here off the grid → a
    /// Miss, since nothing is behind it).
    #[test]
    fn strictly_higher_round_sails_over_ganger() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let entity = spawn_entity();

        let target = key(5, 2, 0);
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(target, Some(entity));
        grid.set_occupant_band(target, Some(HeightBand::Low));
        let cover = CoverLedger::new();

        // A HIGH round (z high within the storey) flat East through the target cell.
        let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);

        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert!(
            !matches!(result.kind, MarchKind::Ganger(_)),
            "a HIGH round must sail over a LOW ganger, not impact it",
        );
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "with nothing behind, the round flies off the grid laterally → Miss",
        );
    }

    // --- AC #3: cover stops a non-strictly-higher round; destroyed cover passes. ---

    /// Cover in the path stops a round not flying strictly higher (a LOW round vs a
    /// MID cover impacts — `Cover` result carrying the `CoverEntry`).
    #[test]
    fn cover_stops_non_strictly_higher_round() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let grid = OccupancyGrid::new();

        let at = key(5, 2, 0);
        let entry = cover_entry(HeightBand::Mid);
        let mut cover = CoverLedger::new();
        cover.insert(at, entry);

        // A LOW round flat East — LOW is not strictly higher than MID → impacts.
        let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);

        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Cover(entry),
            "a LOW round must impact MID cover, carrying the CoverEntry",
        );
        assert_eq!(result.at, at);
    }

    /// A DESTROYED cover cell does NOT stop the round — destroyed cover is excluded
    /// via `is_cover_destroyed`, so the round passes through.
    #[test]
    fn destroyed_cover_passes_through() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();

        let at = key(5, 2, 0);
        let entry = cover_entry(HeightBand::High);
        let mut cover = CoverLedger::new();
        cover.insert(at, entry);

        // The occupancy grid marks the cover cell destroyed.
        let mut grid = OccupancyGrid::new();
        grid.mark_cover_destroyed(at);

        let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);

        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert!(
            !matches!(result.kind, MarchKind::Cover(_)),
            "destroyed cover must NOT stop the round",
        );
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "with the cover destroyed and nothing else, the round misses off-grid",
        );
    }

    /// A cover entry whose own `destroyed` flag is set also passes through — the
    /// march checks the ledger entry's flag as well as the grid's exclusion set.
    #[test]
    fn ledger_destroyed_flag_passes_through() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let grid = OccupancyGrid::new();

        let at = key(5, 2, 0);
        let mut entry = cover_entry(HeightBand::High);
        entry.destroyed = crate::cover::Destroyed::new(true);
        let mut cover = CoverLedger::new();
        cover.insert(at, entry);

        let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);

        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "a ledger entry flagged destroyed must not stop the round",
        );
    }

    // --- AC #4: an intact slab stops a climbing ray; a destroyed/absent slab crosses. ---

    /// A climbing ray crossing a z-boundary with an intact `Present` slab is stopped
    /// (`Slab` result); a `Destroyed` slab at the same boundary lets it cross.
    #[test]
    fn present_slab_stops_climbing_ray_destroyed_crosses() {
        let tuning = CombatTuning::default();
        let muzzle = center(0, 0, 0);
        let dir = Vec3::new(1.0, 1.0, 1.0);
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();

        // The first z-boundary the staircase crosses is into level 1 at (1,1,*) —
        // the slab keyed at the UPPER level (1,1,1) (floor of the upper storey).
        let slab_at = key(1, 1, 1);

        // Present → stops the round at the boundary.
        let mut present = SurfaceGrid::new();
        present.set_slab(slab_at, SlabState::Present);
        let r_present = march_vector(muzzle, dir, &grid, &present, &cover, &tuning, far_shooter());
        assert_eq!(
            r_present.kind,
            MarchKind::Slab,
            "an intact Present slab must stop the climbing ray",
        );
        assert_eq!(r_present.at, slab_at, "the slab result names the slab cell");

        // Destroyed → the round crosses (it climbs on out the top → Miss).
        let mut destroyed = SurfaceGrid::new();
        destroyed.destroy_slab(slab_at);
        let r_destroyed = march_vector(
            muzzle,
            dir,
            &grid,
            &destroyed,
            &cover,
            &tuning,
            far_shooter(),
        );
        assert_ne!(
            r_destroyed.kind,
            MarchKind::Slab,
            "a Destroyed slab must NOT stop the round",
        );

        // Absent (the default) → the round also crosses freely.
        let absent = SurfaceGrid::new();
        let r_absent = march_vector(muzzle, dir, &grid, &absent, &cover, &tuning, far_shooter());
        assert_ne!(
            r_absent.kind,
            MarchKind::Slab,
            "an Absent slab must NOT stop the round",
        );
    }

    // --- AC #5: exit rules + no target stop. ---

    /// A ray climbing out the TOP of the grid is a clean sky `Miss`.
    #[test]
    fn ray_leaving_the_top_is_a_sky_miss() {
        let tuning = CombatTuning::default();
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();
        let surface = SurfaceGrid::new(); // no slabs to stop the climb

        // Straight up from the center of (3, 3, 0).
        let muzzle = center(3, 3, 0);
        let dir = Vec3::new(0.0, 0.0, 1.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "a ray climbing out the top is a clean sky Miss",
        );
    }

    /// A ray diving out the BOTTOM strikes the `Ground` at the exit cell.
    #[test]
    fn ray_leaving_the_bottom_strikes_ground() {
        let tuning = CombatTuning::default();
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();
        let surface = SurfaceGrid::new();

        // Straight down from the center of (4, 4, 0): crosses z = 0 in cell (4,4).
        let muzzle = center(4, 4, 0);
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Ground,
            "a ray diving out the bottom strikes the Ground",
        );
        assert_eq!(
            result.at,
            key(4, 4, 0),
            "the Ground result names the exit ground cell",
        );
    }

    /// A flat ray leaving the side of the grid is a clean `Miss`.
    #[test]
    fn ray_leaving_laterally_is_a_miss() {
        let tuning = CombatTuning::default();
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();
        let surface = SurfaceGrid::new();

        // Flat West from the center of (1, 1, 0): leaves the grid at x < 0.
        let muzzle = center(1, 1, 0);
        let dir = Vec3::new(-1.0, 0.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "a ray leaving laterally is a clean Miss",
        );
    }

    /// NO target stop — a round that clears the aim cell continues to the blocker
    /// BEHIND it: a clearable LOW ganger at the aim cell + a HIGH wall behind, with a
    /// HIGH round, sails over the ganger and strikes the wall behind.
    #[test]
    fn no_target_stop_round_continues_to_the_blocker_behind() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let entity = spawn_entity();

        let aim = key(5, 2, 0); // the (clearable) aim cell
        let behind = key(7, 2, 0); // a blocker further along the East ray
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(aim, Some(entity));
        grid.set_occupant_band(aim, Some(HeightBand::Low)); // a LOW ganger — clearable

        let behind_cover = cover_entry(HeightBand::High);
        let mut cover = CoverLedger::new();
        cover.insert(behind, behind_cover); // HIGH wall behind

        // A HIGH round flat East: clears the LOW ganger at `aim`, strikes the HIGH
        // wall at `behind`.
        let muzzle = at_height(2, 2, 0, high_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Cover(behind_cover),
            "the round must sail over the clearable aim ganger and hit the wall behind",
        );
        assert_eq!(
            result.at, behind,
            "the struck thing is the blocker BEHIND the aim cell"
        );
    }

    // --- AC #6: shooter's own cell never blocks; friendly fire is real. ---

    /// The shooter's own cell never blocks its own shot — an occupied / covered own
    /// cell does not stop the round; it leaves that cell and flies on.
    #[test]
    fn shooter_own_cell_never_blocks() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let entity = spawn_entity();

        let shooter_cell = key(2, 2, 0);
        // The shooter's own cell is occupied (by itself) AND holds HIGH cover — both
        // would stop a LOW round if it were any other cell.
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(shooter_cell, Some(entity));
        grid.set_occupant_band(shooter_cell, Some(HeightBand::High));
        let mut cover = CoverLedger::new();
        cover.insert(shooter_cell, cover_entry(HeightBand::High));

        // A LOW round fired from the shooter's own cell East — it must LEAVE the cell.
        let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, shooter_cell);
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "the round must leave the shooter's own cell (not blocked by its own occupant/cover)",
        );
        assert_ne!(
            result.at, shooter_cell,
            "the round must not have stopped in the shooter's own cell",
        );
    }

    /// Friendly fire is REAL — an allied ganger in the path at an equal-or-lower band
    /// impacts, exactly like any other actor (band-vs-band, no exemption list). The
    /// march has no faction input at all, so a teammate is struck the same way.
    #[test]
    fn friendly_fire_is_real() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let ally = spawn_entity();

        let ally_cell = key(4, 2, 0);
        let mut grid = OccupancyGrid::new();
        grid.set_occupant(ally_cell, Some(ally));
        grid.set_occupant_band(ally_cell, Some(HeightBand::Mid));
        let cover = CoverLedger::new();

        // A LOW round (not strictly higher than MID) flat East impacts the ally.
        let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Ganger(ally),
            "an allied ganger at equal-or-lower band is struck — friendly fire is real",
        );
        assert_eq!(result.at, ally_cell);
    }

    // --- AC #7: graceful degenerate marches + impact point via SimPos::new. ---

    /// A direction leaving the grid IMMEDIATELY is a graceful `Miss`, no panic — a
    /// ray fired West from the edge cell (0, 0, 0).
    #[test]
    fn direction_leaving_grid_immediately_is_a_graceful_miss() {
        let tuning = CombatTuning::default();
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();
        let surface = SurfaceGrid::new();

        let muzzle = center(0, 0, 0);
        let dir = Vec3::new(-1.0, 0.0, 0.0); // straight off the west edge
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "a ray leaving the grid immediately is a graceful Miss",
        );
    }

    /// A zero direction is graceful — a `Miss` at the muzzle, no panic / NaN.
    #[test]
    fn zero_direction_is_a_graceful_miss() {
        let tuning = CombatTuning::default();
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();
        let surface = SurfaceGrid::new();

        let muzzle = center(3, 3, 0);
        let result = march_vector(
            muzzle,
            Vec3::ZERO,
            &grid,
            &surface,
            &cover,
            &tuning,
            far_shooter(),
        );
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "a zero direction is a graceful Miss"
        );
        assert_eq!(
            result.impact, muzzle,
            "the impact point is the muzzle itself"
        );
    }

    /// An out-of-grid muzzle is graceful — a `Miss`, no panic (the grid degrades on
    /// any out-of-range coordinate).
    #[test]
    fn out_of_grid_muzzle_is_a_graceful_miss() {
        let tuning = CombatTuning::default();
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();
        let surface = SurfaceGrid::new();

        // x past the grid extent.
        let muzzle = SimPos::new(100.0, 5.0, 0.5);
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "an out-of-grid muzzle is a graceful Miss"
        );
    }

    /// The impact point is a `SimPos` (built via `SimPos::new`) lying on the ray — a
    /// cover hit's impact point is `muzzle + t × dir` for some `t ≥ 0`. Checks it is
    /// collinear with the ray (the cross product with `dir` is ~zero) and ahead of
    /// the muzzle.
    #[test]
    fn impact_point_lies_on_the_ray() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();
        let grid = OccupancyGrid::new();

        let at = key(5, 2, 0);
        let mut cover = CoverLedger::new();
        cover.insert(at, cover_entry(HeightBand::High));

        let muzzle = at_height(2, 2, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());

        let from_muzzle = *result.impact - *muzzle;
        // Collinear with dir → the cross product is ~zero.
        let cross = from_muzzle.cross(dir);
        assert!(
            cross.length() < 1.0e-4,
            "the impact point must lie on the ray (cross {cross:?})",
        );
        // Ahead of the muzzle (positive projection onto dir).
        assert!(
            from_muzzle.dot(dir) > 0.0,
            "the impact point must be ahead of the muzzle along dir",
        );
    }

    /// All geometry reads `tuning` for the band edges (no hardcoded fraction): a
    /// round whose within-level fraction sits exactly on the default MID→HIGH edge
    /// classifies HIGH and clears a MID ganger; raising the MID→HIGH edge above that
    /// fraction reclassifies the SAME ray as MID, which (equal to the MID ganger) now
    /// impacts. Same muzzle/dir, different tuning → the clearance flips, proving the
    /// band edge is read live from `tuning.projectile_band_edges`, never hardcoded.
    #[test]
    fn band_edges_come_from_tuning() {
        let surface = SurfaceGrid::new();
        let entity = spawn_entity();
        let target = key(5, 2, 0);

        // A round whose fraction is exactly the default MID→HIGH edge → HIGH by
        // default (clears a MID occupant). Raising the MID→HIGH edge above it makes
        // it MID (equal to a MID occupant → impacts). Same ray, different tuning.
        let default = CombatTuning::default();
        let probe = *default.projectile_band_edges.mid_high;

        let mut grid = OccupancyGrid::new();
        grid.set_occupant(target, Some(entity));
        grid.set_occupant_band(target, Some(HeightBand::Mid));
        let cover = CoverLedger::new();

        let muzzle = at_height(2, 2, 0, probe);
        let dir = Vec3::new(1.0, 0.0, 0.0);

        // Default: the round is HIGH, strictly above the MID occupant → sails over.
        let r_default = march_vector(
            muzzle,
            dir,
            &grid,
            &surface,
            &cover,
            &default,
            far_shooter(),
        );
        assert!(
            !matches!(r_default.kind, MarchKind::Ganger(_)),
            "under default edges the round is HIGH and clears the MID ganger",
        );

        // Raise the MID→HIGH edge above the probe: the round is now MID, equal to the
        // MID occupant → impacts. Proves the band edge is read from tuning.
        let mut raised = CombatTuning::default();
        raised.projectile_band_edges.mid_high = BandEdge::new(probe + 0.1);
        let r_raised = march_vector(muzzle, dir, &grid, &surface, &cover, &raised, far_shooter());
        assert_eq!(
            r_raised.kind,
            MarchKind::Ganger(entity),
            "raising the MID→HIGH edge reclassifies the round MID → it impacts the MID ganger",
        );
    }

    /// A march driven through `setup_battle`'s real grids end to end: build an
    /// occupancy + surface + cover grid via the `OccupancyInput` pour, then march a
    /// flat ray into a HIGH wall and assert a Cover stop — proving `march_vector`
    /// reads the same grids the rest of the sim builds.
    #[test]
    fn marches_over_real_built_grids() {
        let tuning = CombatTuning::default();
        let surface = SurfaceGrid::new();

        let wall_at = key(6, 3, 0);
        let input = OccupancyInput {
            terrain:   vec![TerrainPlacement::new(wall_at, TerrainKind::Wall)],
            occupants: Vec::<OccupantPlacement>::new(),
        };
        let grid = OccupancyGrid::build_from_occupancy_input(&input);

        let wall_entry = cover_entry(HeightBand::High);
        let mut cover = CoverLedger::new();
        cover.insert(wall_at, wall_entry);

        let muzzle = at_height(2, 3, 0, low_above_floor(&tuning));
        let dir = Vec3::new(1.0, 0.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Cover(wall_entry),
            "the march must stop on the wall built via the occupancy input",
        );
        assert_eq!(result.at, wall_at);
    }

    /// A long corner-to-corner diagonal completes within the iteration cap — it never
    /// spins, and resolves to a grid-exit result (`Miss` off the far side).
    #[test]
    fn long_diagonal_completes_within_step_cap() {
        let tuning = CombatTuning::default();
        let grid = OccupancyGrid::new();
        let cover = CoverLedger::new();
        let surface = SurfaceGrid::new();

        let muzzle = center(0, 0, 0);
        // A shallow diagonal so the ray crosses the full 60×60 span before exiting.
        let dir = Vec3::new(1.0, 1.0, 0.0);
        let result = march_vector(muzzle, dir, &grid, &surface, &cover, &tuning, far_shooter());
        assert_eq!(
            result.kind,
            MarchKind::Miss,
            "a clear corner-to-corner diagonal exits the grid as a Miss",
        );
        // The exit cell is on the far edge (one of the last in-grid cells).
        assert!(
            result.at.x == i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX) - 1
                || result.at.y == i32::try_from(GRID_HEIGHT).unwrap_or(i32::MAX) - 1,
            "the exit cell sits on a far edge: {:?}",
            result.at,
        );
        // MAX_LEVELS is referenced structurally; confirm a level-0 flat ray stays on
        // level 0.
        assert_eq!(result.at.z, 0);
        let _ = MAX_LEVELS;
    }
}
