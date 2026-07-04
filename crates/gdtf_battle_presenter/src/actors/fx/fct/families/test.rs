//! The ONE table-driven pairwise-distinctness test over the pop palette (GTW-572,
//! acceptance 6) — it replaces the per-family O(N²) valence-distinctness tests the old
//! reader clones each carried (`the_dot_valence_differs…`, `the_field_valence_differs…`,
//! `the_suppression_valence_differs…`, `the_on_death_marker_is_lethal_not_attrition`).
//!
//! Adding a family = ONE row in the table (P11 — no pre-existing test edited beyond that
//! line); the pairwise sweep then pins the new swatch against every other automatically.

use bevy::prelude::Color;

use super::super::palette::{FctValence, valence_color};

/// Every DISTINCT swatch the pop palette draws consequence families in, one row per
/// swatch. `Damage` and `Lethal` deliberately SHARE the blood red (the lethal pop reads
/// heavier via emphasis, same hue — pinned in `fct/test.rs`), so the blood family is ONE
/// row here, not two.
fn palette_swatches() -> Vec<(&'static str, Color)> {
    vec![
        (
            "blood red (Damage / Lethal — armor-broken, on-death)",
            valence_color(FctValence::Damage),
        ),
        ("wound amber (bleeding)", valence_color(FctValence::Wound)),
        (
            "neutral grey (miss / graze)",
            valence_color(FctValence::Neutral),
        ),
        (
            "suppressed blue-grey (suppression)",
            valence_color(FctValence::Suppressed),
        ),
        ("toxic green (DOT)", valence_color(FctValence::Dot)),
        ("hazard orange (field)", valence_color(FctValence::Field)),
    ]
}

/// PIN-DISCRIMINATING — every pair of palette swatches is DISTINCT, so each consequence
/// family reads as its own signal: collapsing any mapping onto a shared swatch (e.g. the
/// DOT green onto the damage red) fails exactly one pair here.
#[test]
fn every_pair_of_palette_swatches_is_distinct() {
    let swatches = palette_swatches();
    for (i, (name_a, color_a)) in swatches.iter().enumerate() {
        for (name_b, color_b) in swatches.iter().skip(i + 1) {
            assert_ne!(
                color_a, color_b,
                "the {name_a} swatch must differ from the {name_b} swatch — each family \
                 reads as its own signal",
            );
        }
    }
}
