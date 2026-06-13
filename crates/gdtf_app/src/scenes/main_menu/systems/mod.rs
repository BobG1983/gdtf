mod print_state;
pub(in crate::scenes::main_menu) use print_state::{print_on_enter, print_on_exit};

mod cleanup;
pub(in crate::scenes::main_menu) use cleanup::cleanup;

mod track_main_menu;
pub(in crate::scenes::main_menu) use track_main_menu::main_menu_complete;

mod move_on;
pub(in crate::scenes::main_menu) use move_on::move_on;
