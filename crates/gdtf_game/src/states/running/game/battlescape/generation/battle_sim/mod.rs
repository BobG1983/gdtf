mod plugin;
pub(in crate::states::running::game::battlescape::generation) use plugin::BattleSimPlugin;

mod content;
mod context;
#[cfg(feature = "headless_test")]
crate::support_use!(context::BattleGenerationContext;);
mod deploy;
mod preplaced;
mod procgen;
#[cfg(feature = "headless_test")]
crate::support_use!(preplaced::PreplacedGangers;);
mod resolved;
#[cfg(any(feature = "headless_test", feature = "mcp"))]
crate::support_use!(resolved::ResolvedBattleSeed;);
mod seed;
mod systems;
#[cfg(feature = "dev_tools")]
pub(crate) use systems::advance_battle_generation;

#[cfg(test)]
mod test;
