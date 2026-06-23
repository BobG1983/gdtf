//! The targeting-hint spawn / despawn / update systems.

mod spawn;
mod update;

pub(in crate::states::running::game::battlescape) use spawn::{
    despawn_targeting_hint, spawn_targeting_hint,
};
pub(in crate::states::running::game::battlescape) use update::update_targeting_hint;
