mod formula;
mod result;

#[cfg(test)]
mod test;

pub use formula::resolve_hit;
pub use result::{
    DamageMagnitude, DamageReal, HitResult, HpDamage, IntegrityWear, PenetratingDamage,
};
