mod plugin;
mod systems;
pub(in crate::states) use plugin::RunningScenePlugin;
#[cfg(any(
    all(debug_assertions, feature = "net_qa"),
    all(feature = "dev_tools", not(feature = "test-support"))
))]
pub(crate) use systems::UiCamera;
mod resources;

mod running_state;
crate::support_use!(running_state::RunningState;);

pub(crate) mod menu;
pub(in crate::states::running) use menu::MenuScenePlugin;

pub(crate) mod game;
pub(in crate::states::running) use game::GameScenePlugin;
crate::support_use!(game::GameState;);
crate::support_use!(game::BattleScapeState;);
crate::support_use!(game::AfterMathState;);

pub(crate) mod options;
pub(in crate::states::running) use options::OptionsScenePlugin;

mod quit;
pub(in crate::states::running) use quit::QuitScenePlugin;
