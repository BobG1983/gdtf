//! Area fields: placement, duration, drain, immunity, and tick.

mod apply_effect;
mod drain;
mod duration;
mod effect;
mod field;
mod immunity;
mod registry;
mod tick;

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_lifetime;
#[cfg(test)]
mod test_turn_start;
#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyFieldEffect, DrainExempt, FieldExpired, OccupantArmor, OccupantDrain};
pub use drain::{ApplyDrain, FieldDamage};
pub use duration::{ApplyDuration, FieldDuration, FieldTurns};
pub use effect::FieldEffect;
pub use field::FieldDef;
pub use immunity::{ApplyImmunity, ImmuneArmorTypes};
pub use registry::{FieldDefRegistry, FieldKey, FieldRegistry, PlacedField};
pub use tick::{FieldAfflicted, FieldOngoing, FieldTicked, tick_fields};
