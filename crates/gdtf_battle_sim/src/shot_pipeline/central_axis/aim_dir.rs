//! The recoil-climb-tilted firing axis — the [`AimDir`] unit-direction newtype and
//! [`climb_aim_dir`], which tilts the muzzle→aim axis up by the recoil climb.

use bevy::math::Vec3;

use crate::{cone::PriorShots, metric::SimPos, stability::RecoilGrowth, tuning::RecoilClimb};

/// A **unit firing-axis direction** in the cubic-voxel metric — the (possibly
/// recoil-tilted) muzzle→aim axis the cone is sampled about.
///
/// A named newtype over [`Vec3`] (no-bare-types: a firing direction is a domain
/// value, not a bare vector) whose invariant is **unit length** — it is always
/// constructed normalised by [`climb_aim_dir`]. Distinct from a position
/// ([`SimPos`]) and from a [`crate::ganger::Direction::forward_step`] ground step:
/// this is the full 3D shot axis, including the vertical recoil tilt. Private inner +
/// derived `Deref` (house style); `z` is the `+z` ("up") component the climb tilts.
#[derive(bevy::prelude::Deref, Debug, Clone, Copy, PartialEq)]
pub struct AimDir(Vec3);

impl AimDir {
    /// The inner unit direction vector, by value.
    ///
    /// A convenience over the derived [`Deref`](bevy::prelude::Deref) for callers
    /// that want the [`Vec3`] itself (e.g. the cone sampler) without dereferencing —
    /// the vector is unit length by [`climb_aim_dir`]'s construction.
    #[must_use]
    pub const fn vec(self) -> Vec3 {
        self.0
    }
}

/// The **recoil-climb-tilted firing axis** — the unit muzzle→aim direction, rotated
/// **up** (`+z`) by `prior_shots × recoil_climb × recoil_growth` radians before the
/// cone is sampled (resolution.md §1a: round *i*'s "central axis tilts upward by
/// `prior_shots × recoil_climb × recoil_growth` radians", with `recoil_growth` the
/// E2.2 stability second-curve output, so a braced/prone shooter climbs strictly
/// less).
///
/// With **zero** prior shots the tilt angle is `0`, so the returned [`AimDir`] is the
/// **untilted** unit muzzle→aim axis exactly. With positive prior shots, growth, and
/// climb, the axis pitches up — its `+z` component rises monotonically — about the
/// horizontal axis perpendicular to the muzzle→aim direction (so the heading is
/// preserved and only the elevation changes). The result is **always unit length**.
///
/// `recoil_climb` is the tuning [`crate::tuning::RecoilClimb`]; `recoil_growth` is the
/// E2.2 [`RecoilGrowth`]; `prior_shots` is the burst's [`PriorShots`]. Angular tilt in
/// the cubic-voxel metric — zero pixels.
#[must_use]
pub fn climb_aim_dir(
    muzzle: SimPos,
    aim_point: SimPos,
    prior_shots: PriorShots,
    recoil_climb: RecoilClimb,
    recoil_growth: RecoilGrowth,
) -> AimDir {
    // The base muzzle→aim axis, normalised. A degenerate zero-length axis (muzzle ==
    // aim) falls back to the +X unit so the result stays unit length and panic-free.
    let raw = *aim_point - *muzzle;
    let base = raw.normalize_or_zero();
    let base = if base == Vec3::ZERO { Vec3::X } else { base };

    // The climb angle: prior_shots × recoil_climb × recoil_growth radians. Zero prior
    // shots → 0 radians → no tilt (the base axis returned exactly).
    let tilt = f32::from(*prior_shots) * *recoil_climb * *recoil_growth;
    if tilt == 0.0 {
        return AimDir(base);
    }

    // Tilt the axis UP by rotating `base` toward `+z` within the vertical plane the
    // two share, so only the elevation changes (heading preserved) and the tilt is
    // always upward regardless of heading. `up_perp` is the component of `+z`
    // orthogonal to `base` — the in-plane unit perpendicular pointing to the upward
    // side. For a base already pointing straight up/down `up_perp` is degenerate, so
    // the climb leaves it unchanged (a vertical shot has no heading to tilt).
    let up = Vec3::Z;
    let up_perp = up - base * up.dot(base);
    if up_perp.length_squared() <= 0.0 {
        return AimDir(base);
    }
    let up_perp = up_perp.normalize_or_zero();

    // Rotate `base` toward `up_perp` by `tilt` (an in-plane rotation toward +z), then
    // re-normalise to guarantee the unit-length invariant against f32 drift.
    let (sin, cos) = tilt.sin_cos();
    let rotated = base * cos + up_perp * sin;
    AimDir(rotated.normalize_or_zero())
}
