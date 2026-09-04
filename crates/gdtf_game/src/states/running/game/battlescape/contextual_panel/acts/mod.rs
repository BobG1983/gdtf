//! Contextual act panel modules; see `docs/authoring/contextual-act-recipe.md`.
pub(in crate::states::running::game::battlescape) mod enter_emplacement;
pub(in crate::states::running::game::battlescape) mod execute;
pub(in crate::states::running::game::battlescape) mod exit_emplacement;
pub(in crate::states::running::game::battlescape) mod melee;
pub(in crate::states::running::game::battlescape) mod open_door;
pub(in crate::states::running::game::battlescape) mod shove;
pub(in crate::states::running::game::battlescape) mod stabilize;
pub(in crate::states::running::game::battlescape) mod throw_grenade;
