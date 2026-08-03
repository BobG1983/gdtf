use bevy::math::Vec3;
use gdtf_battle_sim::weapon::DamageType;

use super::super::roles::{
    COMPASS_DIRECTIONS, DIRECTION_COUNT, EffectRoles, nearest_direction_index,
};

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
    let conseq_same =
        roles.bleed == roles.armor_break && roles.armor_break == roles.cover_destroyed;
    assert!(
        !conseq_same,
        "the consequence FX roles must not all share one index (authoring slip)",
    );
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

#[test]
fn nearest_direction_index_picks_the_matching_compass_column() {
    for (index, dir) in COMPASS_DIRECTIONS.iter().enumerate() {
        let picked = nearest_direction_index(Vec3::new(dir.x, dir.y, 0.0));
        assert_eq!(
            picked, index,
            "compass column {index}'s own heading must pick column {index}, got {picked}",
        );
    }
    assert_eq!(
        nearest_direction_index(Vec3::new(0.0, 0.0, 1.0)),
        0,
        "a straight-up shot (no XY heading) must default to column 0",
    );
    assert!(
        nearest_direction_index(Vec3::new(0.3, -0.9, 0.2)) < DIRECTION_COUNT,
        "the picked direction index must be a valid strip column",
    );
}
