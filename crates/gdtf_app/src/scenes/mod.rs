mod plugin;
pub(crate) use plugin::ScenesPlugin;

mod init;
pub(in crate::scenes) use init::InitScenePlugin;

mod intro;
pub(in crate::scenes) use intro::IntroScenePlugin;

mod load;
pub(in crate::scenes) use load::LoadScenePlugin;

mod main_menu;
pub(in crate::scenes) use main_menu::MainMenuScenePlugin;

mod playing;
pub(in crate::scenes) use playing::PlayingScenePlugin;

mod teardown;
pub(in crate::scenes) use teardown::TeardownScenePlugin;
