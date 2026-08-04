//! On-death leave-field effect.

use super::{ApplyOnDeathEffect, DeathFanOut};
use crate::{effects::fields::FieldKey, metric::CellLevel};

/// Spawns a field at the death cell from a catalog key.
pub struct ApplyLeaveField<'k> {
    field: &'k FieldKey,
}

impl<'k> ApplyLeaveField<'k> {
    /// Build the applicator.
    #[must_use]
    pub const fn new(field: &'k FieldKey) -> Self {
        Self { field }
    }
}

impl ApplyOnDeathEffect for ApplyLeaveField<'_> {
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>) {
        let Some(defs) = fan_out.field_defs else {
            return;
        };
        let Some(def) = defs.def(self.field) else {
            return;
        };
        fan_out.fields.spawn(at, def.clone());
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        ecs::system::SystemState,
        prelude::{Query, World},
    };

    use super::{ApplyLeaveField, ApplyOnDeathEffect};
    use crate::{
        effects::{
            fields::{
                FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
                ImmuneArmorTypes,
            },
            on_death::{DeathFanOut, VictimRow},
        },
        metric::{Cell, CellLevel, Level},
        occupancy::OccupancyGrid,
        test_support::field_turns,
        weapon::DamageType,
    };

    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    fn catalog_with(key: &str) -> FieldDefRegistry {
        let mut defs = FieldDefRegistry::default();
        defs.insert(
            FieldKey::new(key.to_owned()),
            FieldDef::new(
                FieldDamage::new(3),
                DamageType::Plasma,
                ImmuneArmorTypes::default(),
                FieldDuration::Turns(field_turns(2)),
            ),
        );
        defs
    }

    fn fan(key: &FieldKey, defs: Option<&FieldDefRegistry>, at: CellLevel) -> FieldRegistry {
        let mut world = World::new();
        let grid = OccupancyGrid::new();
        let mut fields = FieldRegistry::new();
        let mut cascade = Vec::new();
        let mut state: SystemState<Query<VictimRow>> = SystemState::new(&mut world);
        let Ok(mut victims) = state.get_mut(&mut world) else {
            unreachable!("a plain Query SystemParam always validates");
        };
        let mut fan_out = DeathFanOut {
            grid:       &grid,
            victims:    &mut victims,
            fields:     &mut fields,
            field_defs: defs,
            cascade:    &mut cascade,
        };
        ApplyLeaveField::new(key).fan_at(at, &mut fan_out);
        fields
    }

    #[test]
    fn leave_field_spawns_the_referenced_field_at_the_death_cell() {
        let defs = catalog_with("burning");
        let fields = fan(
            &FieldKey::new("burning".to_owned()),
            Some(&defs),
            ground(7, 8),
        );
        assert!(
            fields.field_at(&ground(7, 8)).is_some(),
            "the referenced field was spawned at the death cell"
        );
    }

    #[test]
    fn an_absent_catalog_fans_nothing() {
        let fields = fan(&FieldKey::new("burning".to_owned()), None, ground(7, 8));
        assert!(
            fields.field_at(&ground(7, 8)).is_none(),
            "no catalog → nothing spawned (fail-closed)"
        );
    }

    #[test]
    fn an_unresolvable_key_fans_nothing() {
        let defs = catalog_with("burning");
        let fields = fan(
            &FieldKey::new("no_such_field".to_owned()),
            Some(&defs),
            ground(7, 8),
        );
        assert!(
            fields.field_at(&ground(7, 8)).is_none(),
            "an unknown key → nothing spawned (fail-closed)"
        );
    }
}
