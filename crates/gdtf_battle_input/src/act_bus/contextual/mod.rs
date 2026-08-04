//! Contextual act families and shared drain into sim requests.

mod seam;

pub use seam::{
    ContextualAct, ContextualActAppExt, ContextualActSystems, PendingContextualIntents, SlotRank,
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
