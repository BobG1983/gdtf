//! On-death effects: explode, leave field, and resolution fan-out.

mod apply_effect;
mod component;
mod effect;
mod explode;
mod leave_field;
mod resolve;
mod signal;

#[cfg(test)]
mod test;
#[cfg(test)]
mod tests;

pub use apply_effect::{ApplyOnDeathEffect, DeathFanOut, VictimRow};
pub use component::OnDeath;
pub use effect::OnDeathEffect;
pub use explode::{ApplyExplode, ExplodeDamage};
pub use leave_field::ApplyLeaveField;
pub use resolve::{CoverOnDeathRegistry, resolve_on_death};
pub use signal::OnDeathOccurred;
