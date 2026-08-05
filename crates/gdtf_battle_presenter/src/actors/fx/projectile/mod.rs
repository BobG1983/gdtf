mod advance;
mod pending;
mod spawn;
pub(in crate::actors::fx) mod travel;

pub use advance::advance_projectiles;
pub use pending::PendingImpact;
pub(in crate::actors::fx) use spawn::spawn_pops_at_anchor;
pub use spawn::spawn_shot_projectiles;
pub use travel::{ProjectileTravel, ShotProjectile};
