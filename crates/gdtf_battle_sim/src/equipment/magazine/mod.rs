mod ammo;
mod guard;

#[cfg(test)]
mod test;

pub use ammo::{
    AmmoCompatible, LoadedRounds, Magazine, MagazineEmpty, MagazineFull, ReloadTu, ammo_compatible,
    clamp_burst,
};
pub use guard::{CanFire, FireActor, InBounds, can_fire, in_bounds, mode_tu_cost};
