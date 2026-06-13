mod print_state;
pub(in crate::scenes::running::game::battlescape::aftermath::animate_out) use print_state::{
    print_on_enter, print_on_exit,
};

mod cleanup;
pub(in crate::scenes::running::game::battlescape::aftermath::animate_out) use cleanup::cleanup;

mod track_complete;
pub(in crate::scenes::running::game::battlescape::aftermath::animate_out) use track_complete::game_battlescape_aftermath_animate_out_complete;

mod move_on;
pub(in crate::scenes::running::game::battlescape::aftermath::animate_out) use move_on::move_on;
