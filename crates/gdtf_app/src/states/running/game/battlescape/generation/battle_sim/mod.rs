//! The battle-sim integration sub-module of the Generation scene: the
//! [`BattleSimPlugin`] that drives `gdtf_battle_sim` into the running app
//! (E10.5 / GTW-207).

mod plugin;
pub(in crate::states::running::game::battlescape::generation) use plugin::BattleSimPlugin;

mod deploy;
mod procgen;
// `pub(crate)`, not private, and ONLY under `dev_tools`: the GTW-655 dev-tools stepper
// (`crate::dev::procgen_stepper`) is the sole consumer of this wider path — it finishes its
// staged drive through the SAME merge + finding-conversion logic `request_battle_setup` uses.
// Feature-gated so a non-`dev_tools` build never carries an unconsumed `pub(crate)` re-export
// (which would otherwise be a dead/unused-import warning under `-D warnings`).
#[cfg(feature = "dev_tools")]
pub(crate) use procgen::{outcome_from_emitted, outcome_from_packing_error};
mod seed;
#[cfg(feature = "dev_tools")]
pub(crate) use seed::resolve_root_seed;
mod systems;

#[cfg(test)]
mod test;
