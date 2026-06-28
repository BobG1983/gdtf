//! Systems for the Prev/Next selection-cycle cluster (GTW-458): the spawn/despawn lifecycle
//! and the press → intent router.

mod actions;
mod spawn;

pub(in crate::states::running::game::battlescape) use actions::select_cycle_button_intents;
pub(in crate::states::running::game::battlescape) use spawn::{
    despawn_select_cycle, spawn_select_cycle,
};
