//! Melee weapons: components, fight modes, specs, registry.

mod bundle;
mod components;
mod fight_mode;
mod registry;
mod spec;

pub use bundle::{MeleeDamageProfile, MeleeWeaponBundle};
pub use components::{MeleeWeapon, Reach};
pub use fight_mode::{FightMode, FightModeKind, FightModeSpec, Strikes, TuCost};
pub use registry::{FISTS_KEY, MeleeWeaponRegistry};
pub use spec::MeleeWeaponSpec;
