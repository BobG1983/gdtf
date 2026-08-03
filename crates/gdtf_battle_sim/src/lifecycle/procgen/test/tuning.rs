use crate::procgen::{
    DeadRectScatterCount, LargePrefabAreaThreshold, MaxCoverageCap, MinDensityFloor, ProcgenTuning,
};

const SHIPPED_PROCGEN_TUNING_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/core_tuning/procgen.tuning.ron"
));

#[test]
fn fill_tuning_newtypes_wrap_inner_and_deref() {
    assert_eq!((*MinDensityFloor::new(0.7)).to_bits(), 0.7_f32.to_bits());
    assert_eq!((*MaxCoverageCap::new(0.6)).to_bits(), 0.6_f32.to_bits());
    assert_eq!(*LargePrefabAreaThreshold::new(99), 99u32);
    assert_eq!(*DeadRectScatterCount::new(5), 5u8);
    assert_eq!(*LargePrefabAreaThreshold::new(99).area(), 99i64);
    assert_eq!(*DeadRectScatterCount::new(5).count(), 5usize);
}

#[test]
fn default_coverage_cap_sits_above_the_density_floor() {
    let default = ProcgenTuning::default();
    assert!(
        *default.max_coverage_cap > *default.min_density_floor,
        "the default coverage cap must sit strictly above the default density floor",
    );
}

#[test]
fn shipped_procgen_tuning_parses() {
    let parsed = ron::de::from_str::<ProcgenTuning>(SHIPPED_PROCGEN_TUNING_RON);
    assert!(
        parsed.is_ok(),
        "the shipped procgen.tuning.ron must parse into a complete ProcgenTuning: {:?}",
        parsed.as_ref().err(),
    );
}

/// `#[serde(default)]`: an EMPTY tuple `()` parses (every field omitted) and yields the
#[test]
fn empty_tuning_round_trips_to_default() {
    let parsed = ron::de::from_str::<ProcgenTuning>("()");
    assert_eq!(
        parsed.ok(),
        Some(ProcgenTuning::default()),
        "an empty () procgen tuning must fall back to the const default (serde default)",
    );
}

#[test]
fn partial_tuning_overrides_one_field_and_defaults_rest() {
    let parsed = ron::de::from_str::<ProcgenTuning>("(dead_rect_scatter_count_k: 7)");
    assert!(
        parsed.is_ok(),
        "the partial tuning must parse: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(tuning) = parsed else {
        return;
    };
    assert_eq!(
        *tuning.dead_rect_scatter_count_k, 7u8,
        "the named field must take the authored override",
    );
    let default = ProcgenTuning::default();
    assert_eq!(
        tuning.min_density_floor.to_bits(),
        default.min_density_floor.to_bits(),
        "an omitted field must keep the default value",
    );
    assert_eq!(
        tuning.large_prefab_area_threshold, default.large_prefab_area_threshold,
        "an omitted field must keep the default value",
    );
}
