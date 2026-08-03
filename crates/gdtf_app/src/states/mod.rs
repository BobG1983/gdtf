mod app_state;
crate::support_use!(app_state::AppState;);

mod scaffold;

mod plugin;
crate::support_use!(plugin::ScenesPlugin;);

mod init;
pub(in crate::states) use init::InitScenePlugin;

mod intro;
pub(in crate::states) use intro::IntroScenePlugin;

mod load;
pub(in crate::states) use load::LoadScenePlugin;
#[cfg(any(feature = "test-support", feature = "dev_tools"))]
crate::support_use!(load::LoadedSituation;);
#[cfg(feature = "test-support")]
pub use load::seed_load_fallbacks;

pub(crate) mod running;
pub(in crate::states) use running::RunningScenePlugin;
crate::support_use!(running::RunningState;);
crate::support_use!(running::GameState;);
crate::support_use!(running::BattleScapeState;);
crate::support_use!(running::AfterMathState;);

mod teardown;
pub(in crate::states) use teardown::TeardownScenePlugin;
