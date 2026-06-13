mod plugin;
mod systems;
pub(in crate::scenes) use plugin::RunningScenePlugin;
mod resources;

mod menu;
pub(in crate::scenes::running) use menu::MenuScenePlugin;

mod game;
pub(in crate::scenes::running) use game::GameScenePlugin;

mod options;
pub(in crate::scenes::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::scenes::running) use quit::QuitScenePlugin;
