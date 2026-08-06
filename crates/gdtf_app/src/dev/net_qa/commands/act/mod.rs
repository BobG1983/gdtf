//! The acts a QA client drives the battle with, one command per file.
pub(crate) mod aiming;
pub(crate) mod contextual;
pub(crate) mod end_turn;
pub(crate) mod facing;
pub(crate) mod fire;
pub(crate) mod move_to;
pub(crate) mod reload;
pub(crate) mod select;
pub(crate) mod select_clear;
pub(crate) mod select_cycle;
pub(crate) mod sets;
pub(crate) mod stance;
pub(crate) mod support;

#[cfg(test)]
mod test;

pub(crate) use aiming::ActSetAiming;
#[cfg(feature = "headless_test")]
pub use contextual::ContextualReply;
pub(crate) use contextual::{
    ActEnterEmplacement, ActExecute, ActExitEmplacement, ActMelee, ActOpenDoor, ActShove,
    ActStabilize, ActThrowGrenade,
};
pub(crate) use end_turn::ActEndTurn;
pub(crate) use facing::ActSetFacing;
pub(crate) use fire::ActFire;
pub(crate) use move_to::ActMove;
pub(crate) use reload::ActReload;
pub(crate) use select::ActSelect;
pub(crate) use select_clear::ActSelectClear;
pub(crate) use select_cycle::{ActSelectNext, ActSelectPrev};
crate::support_use!(sets::ActCommandSystems;);
pub(crate) use stance::ActSetStance;
