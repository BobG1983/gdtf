mod plugin;
pub(in crate::states::running::game::battlescape::generation) use plugin::BattleSimPlugin;

mod content;
mod deploy;
mod procgen;
#[cfg(feature = "dev_tools")]
pub(crate) use content::ProcgenContent;
#[cfg(feature = "dev_tools")]
pub(crate) use deploy::deploy_over_generated;
#[cfg(feature = "dev_tools")]
pub(crate) use procgen::outcome_from_packing_error;
mod resolved;
#[cfg(any(feature = "headless_test", feature = "dev_tools", feature = "mcp"))]
crate::support_use!(resolved::ResolvedBattleSeed;);
mod seed;
#[cfg(feature = "dev_tools")]
pub(crate) use seed::resolve_root_seed;
mod systems;

#[cfg(test)]
mod test;
