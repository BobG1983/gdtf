use bevy::prelude::Color;

use super::super::palette::{FctValence, valence_color};

fn palette_swatches() -> Vec<(&'static str, Color)> {
    vec![
        (
            "blood red (Damage / Lethal — armor-broken, on-death)",
            valence_color(FctValence::Damage),
        ),
        ("wound amber (bleeding)", valence_color(FctValence::Status)),
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
