mod fight;
mod strike;
mod structure;

#[cfg(test)]
mod tests;

pub use fight::{
    Connected, FightMargin, FightOutcome, MeleeDamageMult, apply_melee_multiplier,
    melee_damage_mult, opposed_fight,
};
pub use strike::{Combatants, MeleeStrike, MeleeStrikeEnv, MeleeWeaponHit, resolve_melee_strike};
pub use structure::{StructuralMult, resolve_structural_melee};
