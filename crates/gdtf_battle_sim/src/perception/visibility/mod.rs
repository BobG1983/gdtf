//! FOV accrual, fog of war, and squad visibility state.

mod act_view;
mod compute;
mod fog_select;
mod recompute;
mod squad;

#[cfg(test)]
mod test;

pub use act_view::{ActObserved, ActVisibility, ActorIdentified, classify_act};
pub use compute::{FovObserver, accrue, union_fov};
pub use fog_select::{OmniscientFog, move_fog};
pub use recompute::{recompute_visibility, should_recompute_visibility};
pub use squad::{
    CellExplored, CellVisible, FactionRelation, GangerVisible, SquadVisibility, is_ganger_visible,
};
