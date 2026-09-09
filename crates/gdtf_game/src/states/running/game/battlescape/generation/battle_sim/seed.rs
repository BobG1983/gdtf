use bevy::prelude::info;
use gdtf_battle_sim::rng::BattleSeed;

pub(crate) fn resolve_root_seed() -> BattleSeed {
    let seed = time_seed_u64();
    info!(seed, "battle seed: resolved from the wall clock");
    BattleSeed::new(seed)
}

fn time_seed_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_micros() as u64)
}
