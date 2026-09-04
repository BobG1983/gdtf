mod move_on;
pub(in crate::states::running::game::battlescape::battle_running) use move_on::{
    EndTransition, move_on,
};

mod end_battle_on_outcome;
pub(in crate::states::running::game::battlescape::battle_running) use end_battle_on_outcome::end_battle_on_outcome;
