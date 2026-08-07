//! Player and AI acts driven by buffered request messages.

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
pub use enter_emplacement::{
    CanEnterEmplacement, CanExitEmplacement, can_enter_emplacement, can_exit_emplacement,
    dispatch_enter_emplacement, dispatch_exit_emplacement, enter_emplacement_tu_cost,
    exit_emplacement_tu_cost,
};
pub use fire::{
    BattleGridsParam, CanEngage, FireArcDecision, FireDeclaration, RoundCount, WeaponProbes,
    can_engage, decide_fire_arc, dispatch_fire, fire_arc_tu_cost,
};
pub use injury::{InjuryInflicted, apply_injury};
pub use melee::{CanMelee, MeleeAttacker, MeleeReach, can_melee, dispatch_melee, melee_tu_cost};
pub use movement::{
    CanMove, MoveRejected, MoveRejection, MovementOccurred, can_move, dispatch_move,
    move_step_tu_costs, move_tu_cost,
};
pub use open_door::{CanOpenDoor, can_open_door, dispatch_open_door, open_door_tu_cost};
pub use plugin::SimActsPlugin;
pub use posture::{dispatch_set_aiming, dispatch_set_facing, dispatch_set_stance};
pub use reload::{
    CanReload, ReloadOutcome, ReloadResult, can_reload, dispatch_reload, reload_tu_cost,
};
pub use request::{
    AimRequest, EndTurnRequested, EnterEmplacementRequested, ExecuteDownedRequested,
    ExitEmplacementRequested, FireRequested, MeleeRequested, MeleeResolved, MeleeStruck,
    MeleeTarget, MoveRequested, OpenDoorRequested, ReloadRequested, SetAimingRequested,
    SetFacingRequested, SetStanceRequested, ShoveRequested, ShoveSource, StabilizeDownedRequested,
    ThrowGrenadeRequested, ThrowResolved,
};
pub use shove::{
    CanShove, ShoveActor, ShoveOutcome, ShoveTarget, can_shove, dispatch_shove, resolve_shove,
    shove_tu_cost,
};
pub use throw_grenade::{
    CanThrowGrenade, can_throw_grenade, dispatch_throw_grenade, throw_grenade_tu_cost,
};
