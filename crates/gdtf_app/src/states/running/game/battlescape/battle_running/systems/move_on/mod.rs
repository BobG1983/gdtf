mod phase;
mod shown;
mod system;

#[cfg(test)]
mod test;

pub(in crate::states::running::game::battlescape::battle_running) use phase::EndTransition;
pub(in crate::states::running::game::battlescape::battle_running) use system::move_on;
