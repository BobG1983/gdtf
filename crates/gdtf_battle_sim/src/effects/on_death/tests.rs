//! Palette-level behaviour suite for the GTW-552 on-death effect palette: the closed
//! [`OnDeathEffect`](super::OnDeathEffect) vocabulary parses from RON by variant name (the
//! serde bridge, in the exact shipped authoring forms), and the enum's THIN delegation
//! `impl ApplyOnDeathEffect` routes each variant to its isolated behaviour (fanned directly
//! through the trait against a [`DeathFanOut`](super::DeathFanOut) surface, the seam
//! [`resolve_on_death`](crate::effects::on_death::resolve_on_death) drives).
//!
//! Per-effect fan semantics are asserted in each effect file's own `#[cfg(test)]`; this
//! suite proves the enum bridge (parse + delegation), not the drain / spawn maths. Per the
//! brittle-test rule, assertions check the MAPPING + DIRECTION, never a shipped magnitude.

use bevy::{
    ecs::system::SystemState,
    prelude::{Query, World},
};

use super::{ApplyOnDeathEffect, DeathFanOut, ExplodeDamage, OnDeathEffect, VictimRow};
use crate::{
    effects::fields::{
        FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
        FieldTurns, ImmuneArmorTypes,
    },
    ganger::{Hp, LifeState},
    metric::{Cell, CellLevel, Level},
    occupancy::OccupancyGrid,
    weapon::{BlastRadius, DamageType, HitType},
};

/// A ground-floor `(cell, level)` key at `(x, y)`.
fn ground(x: i32, y: i32) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(0))
}

/// The closed [`OnDeathEffect`] vocabulary deserializes each variant from RON by name, in
/// the exact shipped authoring forms (`volatile_charge.weapon.ron` /
/// `waste_drum.terrain_def.ron`) — payload newtypes as bare scalars (the
/// `#[serde(transparent)]` bridge). Pin-discriminating: a mis-named variant or a wrong
/// payload shape fails to parse. Asserts the MAPPING, not a magnitude.
#[test]
fn each_effect_variant_parses_from_ron() {
    // Explode authors as a struct variant: an AoE template + bare damage + wheel node.
    let Ok(explode) = ron::de::from_str::<OnDeathEffect>(
        "Explode(hit_type: Blast(radius: 1), damage: 8, damage_type: Blast)",
    ) else {
        unreachable!("Explode(hit_type:, damage:, damage_type:) must parse");
    };
    assert!(
        matches!(explode, OnDeathEffect::Explode { .. }),
        "Explode maps to the Explode variant"
    );

    // LeaveField authors as a struct variant with a bare string key.
    let Ok(leave) = ron::de::from_str::<OnDeathEffect>("LeaveField(field: \"toxic_waste_pool\")")
    else {
        unreachable!("LeaveField(field:) must parse");
    };
    assert!(
        matches!(leave, OnDeathEffect::LeaveField { .. }),
        "LeaveField maps to the LeaveField variant"
    );
}

/// The [`OnDeathEffect`] enum's THIN delegation `impl ApplyOnDeathEffect` routes each
/// variant to its isolated behaviour — fanning `Explode` through the enum drains the same
/// victim the isolated `ApplyExplode` does, and fanning `LeaveField` spawns the same field
/// the isolated `ApplyLeaveField` does. Proves the bridge forwards, not that the enum
/// carries logic.
#[test]
fn enum_delegates_to_the_isolated_behaviour() {
    let mut world = World::new();
    let victim = world.spawn((Hp::new(10), LifeState::Alive)).id();
    let mut grid = OccupancyGrid::new();
    grid.set_occupant(ground(6, 5), Some(victim));

    let mut defs = FieldDefRegistry::default();
    defs.insert(
        FieldKey::new("burning".to_owned()),
        FieldDef::new(
            FieldDamage::new(3),
            DamageType::Plasma,
            ImmuneArmorTypes::default(),
            FieldDuration::Turns(FieldTurns::new(2)),
        ),
    );
    let mut fields = FieldRegistry::new();
    let mut cascade = Vec::new();
    let mut state: SystemState<Query<VictimRow>> = SystemState::new(&mut world);
    {
        let Ok(mut victims) = state.get_mut(&mut world) else {
            unreachable!("a plain Query SystemParam always validates");
        };
        let mut fan_out = DeathFanOut {
            grid:       &grid,
            victims:    &mut victims,
            fields:     &mut fields,
            field_defs: Some(&defs),
            cascade:    &mut cascade,
        };

        // The Explode arm delegates to ApplyExplode: the adjacent victim's Hp moved.
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(5),
            damage_type: DamageType::Blast,
        }
        .fan_at(ground(5, 5), &mut fan_out);

        // The LeaveField arm delegates to ApplyLeaveField: the referenced field spawned.
        OnDeathEffect::LeaveField {
            field: FieldKey::new("burning".to_owned()),
        }
        .fan_at(ground(7, 8), &mut fan_out);
    }

    assert_eq!(
        world.get::<Hp>(victim).map(|h| **h),
        Some(5),
        "the enum delegates Explode to ApplyExplode (the victim's Hp drained)"
    );
    assert!(
        fields.field_at(&ground(7, 8)).is_some(),
        "the enum delegates LeaveField to ApplyLeaveField (the field spawned)"
    );
}
