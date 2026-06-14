mod plugin;
mod systems;
pub(in crate::scenes::running) use plugin::MenuScenePlugin;

mod components;
crate::support_use! {
    crate::scenes::running::menu::components::{
        BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
    };
}
