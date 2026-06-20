//! Systems for the battlescape bottom bar (GTW-275 layout overhaul) — spawn / despawn the
//! opaque strip on the `BattleScapeState::BattleRunning` boundary.

mod opacify;
mod spawn;

pub(in crate::states::running::game::battlescape) use opacify::{
    opacify_bottom_bar, repad_bottom_bar,
};
pub(in crate::states::running::game::battlescape) use spawn::{
    despawn_bottom_bar, spawn_bottom_bar,
};
