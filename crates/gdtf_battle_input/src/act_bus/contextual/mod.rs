//! The GENERIC contextual-act seam + one input-layer descriptor module per act
//! (GTW-571).
//!
//! The `seam` submodule owns the [`ContextualAct`] descriptor trait, the per-act
//! [`PendingContextualIntents`] queue, the per-act generic
//! [`drain_contextual_intents`] system in the ONE explicitly-ordered
//! [`ContextualActSystems::Drain`] set, and the compile-time
//! [`ContextualActAppExt::add_contextual_act`] registrar. Each act module is the act's
//! whole input-layer surface — adding a contextual act adds ONE module here plus ONE
//! `add_contextual_act::<A>()` line in the plugin (see
//! `docs/authoring/contextual-act-recipe.md`).

mod seam;

pub use seam::{
    ContextualAct, ContextualActAppExt, ContextualActSystems, PendingContextualIntents,
    configure_contextual_act_drains, drain_contextual_intents,
};

mod enter_emplacement;
mod execute;
mod exit_emplacement;
mod melee;
mod open_door;
mod shove;
mod stabilize;
mod throw_grenade;

pub use enter_emplacement::EnterEmplacementAct;
pub use execute::ExecuteAct;
pub use exit_emplacement::ExitEmplacementAct;
pub use melee::MeleeAct;
pub use open_door::OpenDoorAct;
pub use shove::ShoveAct;
pub use stabilize::StabilizeAct;
pub use throw_grenade::ThrowGrenadeAct;

#[cfg(test)]
mod test;
