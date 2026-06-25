//! AC #5: `climb_aim_dir` — zero prior shots points muzzle→aim exactly; climb
//! monotone +z with prior shots; unit length for all inputs. Plus the [`AimDir`]
//! newtype `Deref` house-style check.

use super::support::TOL;
use crate::{
    central_axis::climb_aim_dir, cone::PriorShots, metric::SimPos, stability::RecoilGrowth,
    tuning::RecoilClimb,
};

#[test]
fn zero_prior_shots_points_muzzle_to_aim_exactly() {
    let muzzle = SimPos::new(2.0, 2.0, 1.0);
    let aim = SimPos::new(8.0, 5.0, 1.5);
    let dir = climb_aim_dir(
        muzzle,
        aim,
        PriorShots::first(),
        RecoilClimb::new(0.05),
        RecoilGrowth::new(0.5),
    );
    // The untilted unit muzzle→aim axis.
    let expected = (*aim - *muzzle).normalize_or_zero();
    let got = dir.vec();
    assert!(
        (got - expected).length() < TOL,
        "zero prior shots must yield the untilted muzzle→aim axis exactly: {got:?} vs {expected:?}",
    );
    // And it is unit length.
    assert!((got.length() - 1.0).abs() < TOL);
}

#[test]
fn climb_increases_z_component_monotonically_with_prior_shots() {
    let muzzle = SimPos::new(0.0, 0.0, 1.0);
    // A target ahead and roughly level, so the base axis is near-horizontal and
    // a positive tilt clearly raises +z.
    let aim = SimPos::new(10.0, 0.0, 1.0);
    let climb = RecoilClimb::new(0.05);
    let growth = RecoilGrowth::new(0.5);

    let mut prev_z = f32::NEG_INFINITY;
    for shots in 0u16..6 {
        let dir = climb_aim_dir(muzzle, aim, PriorShots::new(shots), climb, growth);
        let z = dir.vec().z;
        assert!(
            z >= prev_z,
            "the +z component must be non-decreasing across prior shots: {z} after {prev_z} at {shots}",
        );
        if shots > 0 {
            assert!(
                z > prev_z,
                "a positive climb must strictly raise +z per prior shot at {shots}",
            );
        }
        // Always unit length.
        assert!(
            (dir.vec().length() - 1.0).abs() < TOL,
            "unit length at {shots} shots"
        );
        prev_z = z;
    }
}

#[test]
fn climb_aim_dir_is_unit_length_for_arbitrary_inputs() {
    // Arbitrary muzzle/aim pairs and recoil inputs — the result is always unit.
    let cases = [
        (
            SimPos::new(1.0, 2.0, 0.5),
            SimPos::new(7.0, 9.0, 3.0),
            PriorShots::new(3),
        ),
        (
            SimPos::new(-4.0, 3.0, 2.0),
            SimPos::new(2.0, -1.0, 0.0),
            PriorShots::new(5),
        ),
        (
            SimPos::new(0.0, 0.0, 0.0),
            SimPos::new(0.0, 0.0, 6.0), // a near-vertical base axis
            PriorShots::new(2),
        ),
    ];
    for (muzzle, aim, shots) in cases {
        let dir = climb_aim_dir(
            muzzle,
            aim,
            shots,
            RecoilClimb::new(0.07),
            RecoilGrowth::new(0.6),
        );
        let len = dir.vec().length();
        assert!(
            (len - 1.0).abs() < TOL,
            "climb_aim_dir must be unit length for arbitrary inputs, got {len}",
        );
    }
}

#[test]
fn degenerate_zero_axis_is_handled_without_panic() {
    // muzzle == aim: the base axis is zero-length; the fallback keeps the result
    // unit length rather than producing NaN or panicking.
    let p = SimPos::new(3.0, 3.0, 1.0);
    let dir = climb_aim_dir(
        p,
        p,
        PriorShots::first(),
        RecoilClimb::new(0.05),
        RecoilGrowth::new(0.5),
    );
    assert!((dir.vec().length() - 1.0).abs() < TOL);
}

#[test]
fn aim_dir_newtype_derefs_to_inner() {
    let dir = climb_aim_dir(
        SimPos::new(0.0, 0.0, 0.0),
        SimPos::new(1.0, 0.0, 0.0),
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    );
    // Deref and vec() reach the same inner Vec3.
    assert_eq!(*dir, dir.vec());
}
