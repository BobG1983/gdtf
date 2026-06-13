mod print_state;
pub(in crate::scenes::running::game::battlescape::aftermath::display_after) use print_state::{
    print_on_enter, print_on_exit,
};

mod cleanup;
pub(in crate::scenes::running::game::battlescape::aftermath::display_after) use cleanup::cleanup;

mod track_complete;
pub(in crate::scenes::running::game::battlescape::aftermath::display_after) use track_complete::game_battlescape_aftermath_display_after_complete;

mod move_on;
pub(in crate::scenes::running::game::battlescape::aftermath::display_after) use move_on::move_on;
