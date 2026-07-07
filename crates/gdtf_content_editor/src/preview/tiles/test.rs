//! Treatment-table unit tests (GTW-594 A1, editor half): only the Active class renders
//! full-bright; the below-ghost and void-grid classes are categorically distinct.

use bevy::prelude::*;
use gdtf_battle_presenter::{ContextDepth, StoreyTreatment};

use super::sprites::{GHOST_BELOW_TINT, STIPPLE_TINT, VOID_GRID_TINT, base_tile_tint};

/// The A1 law, editor half: ONLY [`StoreyTreatment::Active`] maps to full-bright
/// [`Color::WHITE`]; every context depth maps to the categorical ghost tint (blue-grey,
/// translucent — never full-bright), and Hidden draws nothing.
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
        "the below-ghost is translucent (hue+alpha — GTW-594 C2); got alpha {ghost_alpha}",
    );
}

/// The three categorical classes are pairwise DISTINCT pixels (GTW-594 C2): the
/// authored-here white, the below-ghost blue-grey, and the void-grid faint grey never
/// collapse onto one another (an authoring slip here would erase a class).
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
    // The stipple overlay tints differently from the base ghost so the pattern reads over it.
    assert_ne!(
        STIPPLE_TINT, GHOST_BELOW_TINT,
        "the stipple pattern must read over the ghost base tint",
    );
}
