use bevy::prelude::*;
use gdtf_battle_presenter::{ContextDepth, StoreyTreatment};

use super::sprites::{GHOST_BELOW_TINT, STIPPLE_TINT, VOID_GRID_TINT, base_tile_tint};

#[test]
fn only_the_active_class_is_full_bright() {
    assert_eq!(
        base_tile_tint(StoreyTreatment::Active),
        Some(Color::WHITE),
        "the ACTIVE storey's tiles render full-bright (authored-here)",
    );
    assert_eq!(
        base_tile_tint(StoreyTreatment::Hidden),
        None,
        "a Hidden storey draws nothing",
    );
    for depth in 1..=8_u8 {
        let tint = base_tile_tint(StoreyTreatment::ContextBelow(ContextDepth::new(depth)));
        assert_eq!(
            tint,
            Some(GHOST_BELOW_TINT),
            "a context storey at depth {depth} renders the categorical below-ghost tint",
        );
        assert_ne!(
            tint,
            Some(Color::WHITE),
            "a context storey must NEVER render full-bright (A1: only Active may)",
        );
    }
    let ghost_alpha = GHOST_BELOW_TINT.alpha();
    assert!(
        ghost_alpha < 1.0,
        "the below-ghost is translucent (hue+alpha — C2); got alpha {ghost_alpha}",
    );
}

#[test]
fn the_three_classes_are_pairwise_distinct() {
    assert_ne!(
        GHOST_BELOW_TINT, VOID_GRID_TINT,
        "exists-below and empty must read differently",
    );
    assert_ne!(
        Color::WHITE,
        GHOST_BELOW_TINT,
        "authored-here and exists-below must read differently",
    );
    assert_ne!(
        Color::WHITE,
        VOID_GRID_TINT,
        "authored-here and empty must read differently",
    );
    assert_ne!(
        STIPPLE_TINT, GHOST_BELOW_TINT,
        "the stipple pattern must read over the ghost base tint",
    );
}
