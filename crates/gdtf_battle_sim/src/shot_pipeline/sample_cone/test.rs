use bevy::math::Vec3;

use crate::{
    central_axis::{AimDir, climb_aim_dir},
    cone::{ConeAngle, PriorShots},
    ganger::Shooting,
    metric::SimPos,
    rng::{BattleSeed, ShotRng},
    sample_cone::{ConcentrationP, concentration_p, sample_cone_vector},
    stability::RecoilGrowth,
    tuning::{ConcentrationCoeff, ConcentrationCoeffs, RecoilClimb},
    weapon::Accuracy,
};

const TOL: f32 = 1.0e-5;

const SAMPLES: usize = 2_000;

fn aim_dir_from(muzzle: SimPos, aim: SimPos) -> AimDir {
    climb_aim_dir(
        muzzle,
        aim,
        PriorShots::first(),
        RecoilClimb::new(0.0),
        RecoilGrowth::new(0.0),
    )
}

fn coeffs(base: f32, scale: f32) -> ConcentrationCoeffs {
    ConcentrationCoeffs {
        base:  ConcentrationCoeff::new(base),
        scale: ConcentrationCoeff::new(scale),
    }
}

fn deviation(axis: Vec3, dir: Vec3) -> f32 {
    axis.dot(dir).clamp(-1.0, 1.0).acos()
}

#[test]
fn sampled_vector_is_unit_length_across_many_seeded_draws() {
    let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 3.0, 1.5));
    let cone = ConeAngle::new(0.30);
    let p = ConcentrationP::new(1.5);
    let mut rng = ShotRng::from_root(BattleSeed::new(0xA11C));
    for _ in 0..SAMPLES {
        let shot = sample_cone_vector(aim, cone, p, rng.rng());
        assert!(
            (shot.vec().length() - 1.0).abs() < TOL,
            "sampled shot must be unit length, got {}",
            shot.vec().length(),
        );
    }
}

#[test]
fn deviation_never_exceeds_the_cone_angle() {
    let axis_vec = aim_dir_from(SimPos::new(1.0, 1.0, 1.0), SimPos::new(9.0, 2.0, 2.0));
    let cone = ConeAngle::new(0.25);
    let p = ConcentrationP::new(1.0);
    let mut rng = ShotRng::from_root(BattleSeed::new(0xED6E));
    for _ in 0..SAMPLES {
        let shot = sample_cone_vector(axis_vec, cone, p, rng.rng());
        let dev = deviation(axis_vec.vec(), shot.vec());
        assert!(
            dev <= *cone + TOL,
            "deviation {dev} must never exceed θ_cone {}",
            *cone,
        );
    }
}

#[test]
fn zero_cone_returns_the_axis_exactly() {
    let aim = aim_dir_from(SimPos::new(2.0, 2.0, 1.0), SimPos::new(7.0, 5.0, 1.2));
    let cone = ConeAngle::new(0.0);
    let p = ConcentrationP::new(2.0);
    let mut rng = ShotRng::from_root(BattleSeed::new(0xDEAD));
    for _ in 0..16 {
        let shot = sample_cone_vector(aim, cone, p, rng.rng());
        assert_eq!(
            shot.vec(),
            aim.vec(),
            "a zero cone must return the central axis exactly",
        );
    }
}

#[test]
fn same_seed_yields_the_same_sample_stream() {
    let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(8.0, 1.0, 1.0));
    let cone = ConeAngle::new(0.4);
    let p = ConcentrationP::new(1.2);

    let draw_stream = || {
        let mut rng = ShotRng::from_root(BattleSeed::new(0x5EED));
        (0..SAMPLES)
            .map(|_| sample_cone_vector(aim, cone, p, rng.rng()).vec())
            .collect::<Vec<_>>()
    };
    let a = draw_stream();
    let b = draw_stream();
    assert_eq!(a, b, "two identically-seeded RNGs must give one stream");
}

#[test]
fn scatter_is_genuinely_three_dimensional() {
    let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 0.0, 1.0));
    let axis = aim.vec();
    assert!(axis.z.abs() < TOL, "the test axis must be horizontal");
    let cone = ConeAngle::new(0.5);
    let p = ConcentrationP::new(1.0);
    let mut rng = ShotRng::from_root(BattleSeed::new(0x3D3D));

    let mut saw_lateral = false;
    let mut saw_vertical = false;
    for _ in 0..SAMPLES {
        let shot = sample_cone_vector(aim, cone, p, rng.rng()).vec();
        if shot.y.abs() > 1.0e-3 {
            saw_lateral = true;
        }
        if shot.z.abs() > 1.0e-3 {
            saw_vertical = true;
        }
        if saw_lateral && saw_vertical {
            break;
        }
    }
    assert!(
        saw_lateral && saw_vertical,
        "scatter must be genuinely 3D — both lateral ({saw_lateral}) and vertical ({saw_vertical}) deviation must appear",
    );
}

#[test]
fn concentration_p_rises_with_accuracy() {
    let c = coeffs(1.0, 2.0);
    let low = concentration_p(Shooting::new(1.0), Accuracy::new(0.5), c);
    let high_skill = concentration_p(Shooting::new(2.0), Accuracy::new(0.5), c);
    let high_weapon = concentration_p(Shooting::new(1.0), Accuracy::new(1.5), c);
    assert!(
        *high_skill > *low,
        "more Shooting must raise p ({} vs {})",
        *high_skill,
        *low,
    );
    assert!(
        *high_weapon > *low,
        "more weapon accuracy must raise p ({} vs {})",
        *high_weapon,
        *low,
    );
}

fn mean_deviation(aim: AimDir, cone: ConeAngle, p: ConcentrationP, seed: u64) -> f32 {
    let mut rng = ShotRng::from_root(BattleSeed::new(seed));
    let axis = aim.vec();
    let total: f32 = (0..SAMPLES)
        .map(|_| deviation(axis, sample_cone_vector(aim, cone, p, rng.rng()).vec()))
        .sum();
    let count = SAMPLES as f32;
    total / count
}

#[test]
fn larger_p_clusters_samples_nearer_the_axis() {
    let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 2.0, 1.5));
    let cone = ConeAngle::new(0.5);
    let low_p = mean_deviation(aim, cone, ConcentrationP::new(1.0), 0x00C0_FFEE);
    let high_p = mean_deviation(aim, cone, ConcentrationP::new(6.0), 0x00C0_FFEE);
    assert!(
        high_p < low_p,
        "a larger p must cluster nearer the axis (mean dev {high_p} < {low_p})",
    );
}

#[test]
fn high_p_concentrates_near_center_even_in_a_wide_cone() {
    let aim = aim_dir_from(SimPos::new(0.0, 0.0, 1.0), SimPos::new(10.0, 0.0, 1.0));
    let wide = ConeAngle::new(1.0);
    let high_p = ConcentrationP::new(8.0);
    let mean = mean_deviation(aim, wide, high_p, 0xBEEF);
    assert!(
        mean < *wide * 0.25,
        "high p must concentrate near center even in a wide cone (mean {mean} ≪ θ_cone {})",
        *wide,
    );
}

#[test]
fn shot_dir_derefs_to_inner() {
    let aim = aim_dir_from(SimPos::new(0.0, 0.0, 0.0), SimPos::new(1.0, 0.0, 0.0));
    let mut rng = ShotRng::from_root(BattleSeed::new(1));
    let shot = sample_cone_vector(
        aim,
        ConeAngle::new(0.1),
        ConcentrationP::new(1.0),
        rng.rng(),
    );
    assert_eq!(*shot, shot.vec());
}

#[test]
fn concentration_p_and_shooting_deref_to_inner() {
    assert!((*ConcentrationP::new(2.5) - 2.5).abs() < TOL);
    assert!((*Shooting::new(3.0) - 3.0).abs() < TOL);
}
