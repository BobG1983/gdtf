//! Tests of the `effect_roles.spritedef.ron` parsing, per-damage-type row
//! resolution, and compass indexing (mirrors `roles.rs`).

use bevy::math::Vec3;
use gdtf_battle_sim::weapon::DamageType;

use super::super::roles::{
    COMPASS_DIRECTIONS, DIRECTION_COUNT, EffectRoles, nearest_direction_index,
};

/// The shipped `effect_roles.ron` parses into `EffectRoles` and exposes every FX role —
/// a `ron::de` round-trip of the SHIPPED bytes.
///
/// It asserts the file PARSES and HAS all roles (a missing field is a deserialize error);
/// it does NOT pin a tunable index magnitude (those are data the engineer eyeballs and may
/// adjust). A light distinctness guard catches an all-collapsed authoring slip — the four
/// damage-type rows must not share one directional strip.
#[test]
fn shipped_effect_roles_ron_parses_with_all_roles() {
    const SHIPPED: &str =
        include_str!("../../../../../../assets/sprites/effect_roles.spritedef.ron");
    let parsed: Result<EffectRoles, _> = ron::de::from_str(SHIPPED);
    assert!(
        parsed.is_ok(),
        "shipped effect_roles.ron must parse into EffectRoles, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(roles) = parsed else {
        return;
    };
    // The three consequence roles must not all collapse onto one index (an authoring slip).
    let conseq_same =
        roles.bleed == roles.armor_break && roles.armor_break == roles.cover_destroyed;
    assert!(
        !conseq_same,
        "the consequence FX roles must not all share one index (authoring slip)",
    );
    // The four damage-type rows must each carry a DISTINCT directional strip (a per-type
    // color variant) — an all-collapsed authoring slip would make every shot look alike.
    let rows = [
        roles.orange.directions,
        roles.blue.directions,
        roles.green.directions,
        roles.purple.directions,
    ];
    for (i, a) in rows.iter().enumerate() {
        for b in rows.iter().skip(i + 1) {
            assert_ne!(
                a, b,
                "each damage-type row must carry its own directional strip (a per-type color)",
            );
        }
    }
}

/// Each `DamageType` maps to a per-type FX row, the row carries a full 8-way directional
/// strip + a 3-frame impact, and the sweep is total over `DamageType::ALL` — the per-type
/// MECHANISM the contract requires built across the enum (only Kinetic ships in data today).
#[test]
fn fx_for_resolves_every_damage_type_to_a_full_row() {
    const SHIPPED: &str =
        include_str!("../../../../../../assets/sprites/effect_roles.spritedef.ron");
    let parsed: Result<EffectRoles, _> = ron::de::from_str(SHIPPED);
    assert!(
        parsed.is_ok(),
        "shipped effect_roles.ron must parse, got: {:?}",
        parsed.as_ref().err(),
    );
    let Ok(roles) = parsed else {
        return;
    };
    for damage in DamageType::ALL {
        let fx = roles.fx_for(damage);
        assert_eq!(
            fx.directions.len(),
            DIRECTION_COUNT,
            "{damage:?} must resolve to a full 8-way directional strip",
        );
        assert_eq!(
            fx.impact.len(),
            3,
            "{damage:?} must resolve to a 3-frame impact strip",
        );
    }
    // Kinetic (the only type in skirmish data) reads the orange row; the fallback is orange.
    assert_eq!(
        roles.fx_for(DamageType::Kinetic),
        &roles.orange,
        "Kinetic (the in-data type) must read the orange row",
    );
    assert_eq!(
        roles.fallback(),
        &roles.orange,
        "the fallback row must be the orange row",
    );
}

/// `nearest_direction_index` picks the compass column whose heading the trajectory points
/// closest to — each cardinal/diagonal heading resolves to its OWN column, and a column's
/// own heading is its own nearest (round-trip identity).
#[test]
fn nearest_direction_index_picks_the_matching_compass_column() {
    // Each authored compass column's own heading must resolve back to that column.
    for (index, dir) in COMPASS_DIRECTIONS.iter().enumerate() {
        let picked = nearest_direction_index(Vec3::new(dir.x, dir.y, 0.0));
        assert_eq!(
            picked, index,
            "compass column {index}'s own heading must pick column {index}, got {picked}",
        );
    }
    // A z-only (straight up/down) trajectory has no XY heading -> defaults to column 0 (E).
    assert_eq!(
        nearest_direction_index(Vec3::new(0.0, 0.0, 1.0)),
        0,
        "a straight-up shot (no XY heading) must default to column 0",
    );
    // The picked index is always a valid strip column.
    assert!(
        nearest_direction_index(Vec3::new(0.3, -0.9, 0.2)) < DIRECTION_COUNT,
        "the picked direction index must be a valid strip column",
    );
}
