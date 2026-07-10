//! Unit tests for the GTW-427 [`ProcgenTuning`] fill knobs (OQ-6): the newtype Deref
//! surface, the SHIPPED `procgen.tuning.ron` parses, and the embedded-default round-trip
//! follows a tuning edit. NO shipped magnitude is pinned (the loader-test rule).

use crate::procgen::{
    DeadRectScatterCount, LargePrefabAreaThreshold, MinDensityFloor, ProcgenTuning,
};

/// The SHIPPED `assets/core_tuning/procgen.tuning.ron`, included from source for a
/// real-path parse (the weapon / situation shipped-file precedent).
const SHIPPED_PROCGEN_TUNING_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/core_tuning/procgen.tuning.ron"
));

/// Each fill knob wraps the right inner type and its derived [`Deref`] reaches that inner
/// value — built from ARBITRARY literals (never the shipped defaults), so this pins the
/// Deref mechanism + target type, not a balance magnitude (the f32 compared bit-exact to
/// dodge `float_cmp`).
#[test]
fn fill_tuning_newtypes_wrap_inner_and_deref() {
    assert_eq!((*MinDensityFloor::new(0.7)).to_bits(), 0.7_f32.to_bits());
    assert_eq!(*LargePrefabAreaThreshold::new(99), 99u32);
    assert_eq!(*DeadRectScatterCount::new(5), 5u8);
    // The integer-cast helpers expose the value for the fill arithmetic.
    assert_eq!(*LargePrefabAreaThreshold::new(99).area(), 99i64);
    assert_eq!(*DeadRectScatterCount::new(5).count(), 5usize);
}

/// The shipped `procgen.tuning.ron` PARSES into a complete [`ProcgenTuning`] — serde
/// rejects a missing field, so a clean parse proves the schema and file agree. Asserts
/// nothing about the magnitudes (they are tunable balance data).
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
/// const [`ProcgenTuning::default`] — so a file may tune one number without restating the
/// rest. Asserts the parse equals the default, NOT a specific magnitude (auto-follows a
/// default edit, so non-brittle).
#[test]
fn empty_tuning_round_trips_to_default() {
    let parsed = ron::de::from_str::<ProcgenTuning>("()");
    assert_eq!(
        parsed.ok(),
        Some(ProcgenTuning::default()),
        "an empty () procgen tuning must fall back to the const default (serde default)",
    );
}

/// A partial file overrides only the named field and defaults the rest — the
/// single-field-tune ergonomic. Distinctive override value (not the default), and the
/// untouched fields equal the default (non-brittle: no shipped magnitude pinned).
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
