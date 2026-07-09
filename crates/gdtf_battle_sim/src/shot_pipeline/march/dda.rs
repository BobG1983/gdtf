//! The voxel-DDA traversal primitives (Amanatides–Woo) — the per-axis stepping
//! state, the grid-bounds tests, the per-voxel occupant impact test, and the
//! z-boundary slab crossing. Driven by [`march_vector`](super::march_vector).

use bevy::{math::Vec3, prelude::Entity};

use crate::{
    clearance::{Clearance, round_band_for_cell, round_clears_occupant},
    cover::{CoverLedger, HeightBand},
    march::{
        geom::{key_of, key_of_clamped, point_at, xy_in_grid, z_in_grid},
        result::{MarchKind, MarchResult},
    },
    metric::{CellLevel, MAX_LEVELS, SimPos},
    occupancy::{GRID_HEIGHT, GRID_WIDTH, OccupancyGrid},
    surface::{SlabState, SurfaceGrid},
    tuning::CombatTuning,
};

/// A generous per-march iteration cap — far above the longest possible voxel walk
/// (the grid's `60 + 60 + 8` voxel diameter, plus slack), so a degenerate ray can
/// never spin forever while a legitimate corner-to-corner shot always completes.
pub(super) const MAX_STEPS: u32 = (GRID_WIDTH + GRID_HEIGHT + MAX_LEVELS as usize) as u32 * 4;

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

/// The occupant impact at one crossed voxel, or `None` if the round clears
/// everything there — a ganger ([`MarchKind::Ganger`]) checked first, then standing
/// cover ([`MarchKind::Cover`], destroyed cover excluded), then a tag-derived VISION
/// occluder ([`MarchKind::Slab`], GTW-502 — a `BlocksVision`-tagged piece off the
/// grid's [`VisionBlocking`](crate::occupancy::VisionBlocking) surface, destroyed
/// cells excluded). The clearance is band-vs-band ([`round_clears_occupant`]):
/// equal-or-lower impacts, strictly-higher clears. **Any** actor impacts, including
/// the shooter's own gang (true friendly fire — AC #2, #6).
///
/// **Zero-regression + no-double-count (GTW-502).** The occupant + cover clauses are
/// UNCHANGED and run FIRST; the occluder clause is purely additive. A `Wall`/`Cover`
/// already occludes via its [`CoverLedger`](crate::cover::CoverLedger) entry (clause 2),
/// so the occluder clause is reached only when no occupant/cover stopped the round —
/// where its NET-NEW effect is occluding an explicitly-`BlocksVision`-tagged `Slab` (a
/// gap the cover ledger never held). Because the cover-ledger clause runs FIRST, an intact
/// `Wall`/`Cover` already stopped the round before the occluder clause is reached, so the
/// additive clause can only agree (idempotent); a `Cover`/`Emplacement` derives its ledger band
/// exactly and every shipped `Wall` (all `High`) derives that same band, so the two coincide for
/// shipped content anyway.
///
/// `here_point` is the round's position at the voxel ENTRY (the reported impact
/// point, recorded on the [`MarchResult`]). `test_band` is the band the clearance
/// rule compares against — the **lowest** band the round occupies anywhere INSIDE
/// this voxel (GTW-329). A steep shot's z changes within a single cell, so the round
/// can ENTER a cell in a higher band yet dip into the occupant's band before it
/// exits; banding the clearance test against the round's lowest in-voxel band lets a
/// point-blank shot at a lower-stanced target connect instead of sailing over the
/// entry boundary and diving into the ground. For a flat shot the in-voxel band does
/// not change, so `test_band` equals the entry band and the verdict is unchanged.
///
/// A **dead** occupant is transparent: when a ganger would otherwise be struck but
/// `is_dead(entity)` is `true`, the round passes through the corpse and the test
/// falls through to the cover below it (and, finding nothing, returns `None` so the
/// march continues to the next voxel). A live occupant — including a
/// [`LifeState::Downed`](crate::ganger::LifeState::Downed) one — still stops the
/// round; only [`LifeState::Dead`](crate::ganger::LifeState::Dead) is skipped, which
/// is exactly what the caller's predicate encodes (GTW-317). The predicate is
/// read-only and draws no RNG — the skip is purely deterministic.
pub(super) fn impact_at(
    here: CellLevel,
    here_point: SimPos,
    test_band: HeightBand,
    occupancy: &OccupancyGrid,
    cover: &CoverLedger,
    is_dead: &impl Fn(Entity) -> bool,
) -> Option<MarchResult> {
    // 1. A ganger occupant, banded by its silhouette band off the grid — unless it is
    //    a corpse (`is_dead`), in which case the round passes through it (GTW-317).
    if let (Some(entity), Some(occ_band)) =
        (occupancy.occupant(&here), occupancy.occupant_band(&here))
        && round_clears_occupant(test_band, occ_band) == Clearance::Impacts
        && !is_dead(entity)
    {
        return Some(MarchResult {
            kind:   MarchKind::Ganger(entity),
            at:     here,
            band:   test_band,
            impact: here_point,
        });
    }
    // 2. Standing cover, banded by its CoverEntry band, excluded when destroyed (AC #3).
    if !occupancy.is_cover_destroyed(&here)
        && let Some(entry) = cover.peek(&here)
        && !*entry.destroyed
        && round_clears_occupant(test_band, entry.height_band) == Clearance::Impacts
    {
        return Some(MarchResult {
            kind:   MarchKind::Cover(*entry),
            at:     here,
            band:   test_band,
            impact: here_point,
        });
    }
    // 3. A tag-derived VISION occluder (GTW-502 C5), banded by its `BlocksVision` band off the
    //    grid's `VisionBlocking` surface, height-aware via the SAME `round_clears_occupant`
    //    gate and excluded when destroyed (the surface reader already drops destroyed-cover
    //    cells). This clause runs AFTER the occupant + cover checks, which are UNCHANGED — so
    //    there is ZERO regression and NO double-count: an intact `Wall`/`Cover` already
    //    returned via clause 2 above (its CoverLedger entry), so this clause is reached only
    //    when no occupant/cover stopped the round, where it occludes an explicitly-tagged
    //    `Slab` (the NET-NEW gap-closer the cover ledger never held) — and even for a
    //    `Wall`/`Cover` the band it derives is IDENTICAL to the cover entry's, so it could
    //    only agree. The occluder is reported as a `MarchKind::Slab` blocker: a tag-driven
    //    vision occluder is a same-storey opaque surface, and `Slab` is the existing
    //    impassable-geometry blocker kind `is_clear` already classifies as occluding (it is
    //    NOT a `CoverEntry`, so it cannot borrow `MarchKind::Cover`'s payload).
    if let Some(occluder_band) = occupancy.vision_occluder_at(&here)
        && round_clears_occupant(test_band, occluder_band) == Clearance::Impacts
    {
        return Some(MarchResult {
            kind:   MarchKind::Slab,
            at:     here,
            band:   test_band,
            impact: here_point,
        });
    }
    None
}

/// The result of advancing the DDA one voxel — continue marching, or stop on a
/// terminal [`MarchResult`] (an intact slab, or a grid exit).
pub(super) enum Step {
    /// The ray stepped into the next in-grid voxel; keep marching.
    Continue,
    /// The march ended on this step (slab / top / bottom / lateral exit).
    Stopped(MarchResult),
}

/// The mutable voxel-walk state of a march — the current integer voxel, the three
/// per-axis DDA traversals, and the `t` the ray entered the current voxel at.
pub(super) struct MarchState {
    /// Current voxel x (cell coord).
    pub(super) vx:      i32,
    /// Current voxel y (cell coord).
    pub(super) vy:      i32,
    /// Current voxel z (storey index).
    pub(super) vz:      i32,
    /// The x-axis DDA traversal.
    ax:                 AxisDda,
    /// The y-axis DDA traversal.
    ay:                 AxisDda,
    /// The z-axis DDA traversal.
    az:                 AxisDda,
    /// The ray parameter `t` at which the ray entered the current voxel (0 at the
    /// muzzle's own voxel).
    pub(super) entry_t: f32,
}

impl MarchState {
    /// Initialise the walk at integer voxel `(vx, vy, vz)` for the ray `muzzle + t × dir`.
    pub(super) fn new(vx: i32, vy: i32, vz: i32, muzzle: SimPos, dir: Vec3) -> Self {
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

    /// The round's position at the **exit boundary** of the voxel it currently sits
    /// in — `muzzle + t_exit × dir`, where `t_exit` is the next voxel boundary the ray
    /// reaches (the smallest of the three per-axis `t_max`s, Amanatides–Woo).
    ///
    /// Paired with the entry point ([`entry_t`](MarchState::entry_t)), this gives the
    /// two endpoints of the ray's segment through the current voxel — the round's band
    /// sweeps monotonically between them, so the lowest band it occupies in the voxel
    /// is the lower of the entry and exit bands (GTW-329). A non-moving / degenerate
    /// ray has all-`INFINITY` `t_max`s, so `t_exit` is `INFINITY` and the exit point
    /// coincides with the (capped) march end — harmless, as such a ray takes no step.
    pub(super) fn exit_point(&self, muzzle: SimPos, dir: Vec3) -> SimPos {
        let t_exit = self.ax.t_max.min(self.ay.t_max).min(self.az.t_max);
        point_at(muzzle, dir, t_exit)
    }

    /// Step to the next voxel along the smallest-`t_max` axis (Amanatides–Woo),
    /// testing the slab on a z-crossing and the grid faces on an exit. Returns
    /// [`Step::Stopped`] on an intact slab or a grid exit, else [`Step::Continue`]
    /// after advancing into the next in-grid voxel.
    pub(super) fn advance(
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
