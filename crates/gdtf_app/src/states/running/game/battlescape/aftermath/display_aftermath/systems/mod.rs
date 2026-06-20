mod print_state;
pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) use print_state::{
    print_on_enter, print_on_exit,
};

mod cleanup;
pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) use cleanup::cleanup;

mod track_complete;
pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) use track_complete::game_battlescape_aftermath_display_aftermath_complete;

mod move_on;
pub(in crate::states::running::game::battlescape::aftermath::display_aftermath) use move_on::move_on;
