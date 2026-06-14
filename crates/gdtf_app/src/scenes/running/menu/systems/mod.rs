mod print_state;
pub(in crate::scenes::running::menu) use print_state::{print_on_enter, print_on_exit};

mod cleanup;
pub(in crate::scenes::running::menu) use cleanup::clear_nav_map;

mod spawn;
pub(in crate::scenes::running::menu) use spawn::spawn_menu;
