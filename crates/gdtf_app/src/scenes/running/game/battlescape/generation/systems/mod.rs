mod print_state;
pub(in crate::scenes::running::game::battlescape::generation) use print_state::{
    print_on_enter, print_on_exit,
};

mod cleanup;
pub(in crate::scenes::running::game::battlescape::generation) use cleanup::cleanup;

mod track_complete;
pub(in crate::scenes::running::game::battlescape::generation) use track_complete::game_battlescape_generation_complete;

mod move_on;
pub(in crate::scenes::running::game::battlescape::generation) use move_on::move_on;
