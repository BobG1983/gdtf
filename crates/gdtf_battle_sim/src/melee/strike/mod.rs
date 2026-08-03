mod inputs;
mod resolve;
mod verdict;

pub use inputs::{Combatants, MeleeStrikeEnv, MeleeWeaponHit};
pub use resolve::resolve_melee_strike;
pub use verdict::MeleeStrike;
