mod plugin;
pub(in crate::states::running::game::battlescape::generation) use plugin::BattleSimPlugin;

mod deploy;
mod procgen;
#[cfg(feature = "dev_tools")]
pub(crate) use deploy::deploy_over_generated;
#[cfg(feature = "dev_tools")]
pub(crate) use procgen::outcome_from_packing_error;
mod seed;
#[cfg(feature = "dev_tools")]
pub(crate) use seed::resolve_root_seed;
mod systems;

#[cfg(test)]
mod test;
