//! E10 drives the authoritative sim from BUFFERED Bevy MESSAGES, not direct calls: a
//! - `request` — the eight [`#[derive(Message)]`](bevy::prelude::Message) `*Requested`
pub mod downed;
mod enter_emplacement;
mod fire;
mod injury;
mod melee;
pub mod movement;
mod open_door;
mod plugin;
mod posture;
mod reload;
mod request;
mod shove;
mod throw_grenade;

#[cfg(test)]
mod test;

pub use downed::{dispatch_execute_downed, dispatch_stabilize_downed};
pub use enter_emplacement::{dispatch_enter_emplacement, dispatch_exit_emplacement};
pub use fire::{
    BattleGridsParam, CanEngage, FireArcDecision, FireDeclaration, RoundCount, WeaponProbes,
    can_engage, decide_fire_arc, dispatch_fire,
};
pub use injury::{InjuryInflicted, apply_injury};
pub use melee::dispatch_melee;
pub use movement::{MoveRejected, MoveRejection, MovementOccurred, dispatch_move};
pub use open_door::dispatch_open_door;
pub use plugin::SimActsPlugin;
pub use posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance};
pub use reload::{ReloadOutcome, ReloadResult, dispatch_reload};
pub use request::{
    AimRequest, EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
    ExitEmplacementRequested, FireRequested, MeleeRequested, MeleeResolved, MeleeStruck,
    MeleeTarget, MoveRequested, OpenDoorRequested, ReloadRequested, SetAimingRequested,
    SetFacingRequested, SetStanceRequested, ShoveRequested, ShoveSource, StabilizeDownedRequested,
    ThrowGrenadeRequested, ThrowResolved,
};
pub use shove::{ShoveOutcome, dispatch_shove, resolve_shove};
pub use throw_grenade::dispatch_throw_grenade;
