//! Magazine state and fire readiness checks.

mod ammo;
mod guard;

#[cfg(test)]
mod test;

pub use ammo::{
    AmmoCompatible, LoadedRounds, Magazine, MagazineEmpty, MagazineFull, ReloadTu, ammo_compatible,
    clamp_burst,
};
pub use guard::{
    CanFire, FireActor, FireRefusal, InBounds, can_fire, fire_refusal, in_bounds, mode_tu_cost,
};
