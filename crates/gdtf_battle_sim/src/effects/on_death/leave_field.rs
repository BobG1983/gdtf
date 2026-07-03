//! The **`LeaveField`** on-death effect (GTW-547; GTW-552 one-file-per-effect) — the
//! isolated [`ApplyLeaveField`] behaviour that spawns a persistent GTW-545 field at the
//! death cell, so the cell becomes a live hazard (a fuel barrel leaving burning ground
//! when it is smashed).

use super::{ApplyOnDeathEffect, DeathFanOut};
use crate::{fields::FieldKey, metric::CellLevel};

/// **`LeaveField`** — spawn the referenced GTW-545 field at the death cell (GTW-547;
/// isolated per GTW-552).
///
/// Resolves the authored [`FieldKey`] against the fan-out surface's
/// [`field_defs`](DeathFanOut::field_defs) catalog to its
/// [`FieldDef`](crate::fields::FieldDef) and calls the GTW-545
/// [`FieldRegistry::spawn`](crate::fields::FieldRegistry::spawn) placement API at the
/// death cell, so the cell becomes a live hazard that persists + ticks per GTW-545 rules.
///
/// BORROWS its key from the enum variant (the delegation arm lends `&FieldKey` for the
/// duration of one fan — a [`FieldKey`] owns a `String`, and the fan never needs to keep
/// it).
pub struct ApplyLeaveField<'k> {
    /// The field catalog KEY (the GTW-547 ticket's `FieldDefRef`) resolved against the
    /// [`FieldDefRegistry`](crate::fields::FieldDefRegistry) to the spawned field.
    field: &'k FieldKey,
}

impl<'k> ApplyLeaveField<'k> {
    /// Build the field-leaving effect borrowing its authored catalog key.
    #[must_use]
    pub const fn new(field: &'k FieldKey) -> Self {
        Self { field }
    }
}

impl ApplyOnDeathEffect for ApplyLeaveField<'_> {
    /// Fan the field: resolve the key against the catalog and spawn the field at the
    /// death cell. FAIL-CLOSED twice over, never a panic: with the catalog absent
    /// (app/Load-owned — a battle with no field content has none) the fan does nothing
    /// (the [`tick_fields`](crate::fields::tick_fields) Option-resource precedent), and an
    /// unresolvable key (no field file with that stem loaded) fans nothing (the setup-time
    /// [`FieldNotFound`](crate::situation::BattleSetupError) abort's runtime counterpart).
    fn fan_at(&self, at: CellLevel, fan_out: &mut DeathFanOut<'_, '_, '_>) {
        let Some(defs) = fan_out.field_defs else {
            return; // no field catalog this battle — fail closed, no panic
        };
        let Some(def) = defs.def(self.field) else {
            return; // no such field loaded — fail closed, no panic
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
        effects::on_death::{DeathFanOut, VictimRow},
        fields::{
            FieldDamage, FieldDef, FieldDefRegistry, FieldDuration, FieldKey, FieldRegistry,
            FieldTurns, ImmuneArmorTypes,
        },
        metric::{Cell, CellLevel, Level},
        occupancy::OccupancyGrid,
        weapon::DamageType,
    };

    /// A ground-floor `(cell, level)` key at `(x, y)`.
    fn ground(x: i32, y: i32) -> CellLevel {
        CellLevel::new(Cell::new(x, y), Level::new(0))
    }

    /// A one-entry field catalog holding `key` → a small test field.
    fn catalog_with(key: &str) -> FieldDefRegistry {
        let mut defs = FieldDefRegistry::default();
        defs.insert(
            FieldKey::new(key.to_owned()),
            FieldDef::new(
                FieldDamage::new(3),
                DamageType::Plasma,
                ImmuneArmorTypes::default(),
                FieldDuration::Turns(FieldTurns::new(2)),
            ),
        );
        defs
    }

    /// Run `ApplyLeaveField(key).fan_at(at, …)` against `defs`, returning the placement
    /// registry after the fan. Bare-`World` + `SystemState` is the sanctioned pure-sim
    /// unit-test idiom (`bevy-traps.md` #7 carve-out (b)).
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

    /// A resolvable key spawns the referenced field at the death cell.
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

    /// With the catalog ABSENT (a battle with no field content), the fan does nothing —
    /// fail-closed, no panic.
    #[test]
    fn an_absent_catalog_fans_nothing() {
        let fields = fan(&FieldKey::new("burning".to_owned()), None, ground(7, 8));
        assert!(
            fields.field_at(&ground(7, 8)).is_none(),
            "no catalog → nothing spawned (fail-closed)"
        );
    }

    /// An UNRESOLVABLE key (no field with that stem loaded) fans nothing — fail-closed,
    /// no panic.
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
