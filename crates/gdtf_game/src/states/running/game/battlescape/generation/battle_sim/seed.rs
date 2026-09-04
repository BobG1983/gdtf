use bevy::prelude::info;
use gdtf_battle_sim::rng::BattleSeed;

const SEED_ENV_VAR: &str = "GDTF_BATTLE_SEED";

pub(crate) fn resolve_root_seed() -> BattleSeed {
    let seed = if let Ok(raw) = std::env::var(SEED_ENV_VAR) {
        if let Some(n) = parse_seed(&raw) {
            info!(
                seed = n,
                env_var = SEED_ENV_VAR,
                "GDTF_BATTLE_SEED: using env-pinned battle seed",
            );
            n
        } else {
            let fallback = time_seed_u64();
            info!(
                seed = fallback,
                raw_env = raw,
                env_var = SEED_ENV_VAR,
                "GDTF_BATTLE_SEED: env var could not be parsed as u64; using wall-clock seed",
            );
            fallback
        }
    } else {
        let ts = time_seed_u64();
        info!(
            seed = ts,
            "GDTF_BATTLE_SEED: not set; using wall-clock seed",
        );
        ts
    };
    BattleSeed::new(seed)
}

fn parse_seed(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok()
}

fn time_seed_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_micros() as u64)
}

#[cfg(test)]
mod tests {
    use super::parse_seed;

    #[test]
    fn parse_seed_accepts_decimal_u64() {
        assert_eq!(parse_seed("42"), Some(42));
        assert_eq!(parse_seed("0"), Some(0));
        assert_eq!(parse_seed("18446744073709551615"), Some(u64::MAX));
    }

    #[test]
    fn parse_seed_rejects_non_numeric() {
        assert_eq!(parse_seed("abc"), None);
        assert_eq!(parse_seed(""), None);
        assert_eq!(parse_seed("0xDEAD"), None);
    }

    #[test]
    fn parse_seed_trims_whitespace() {
        assert_eq!(parse_seed("  99  "), Some(99));
    }
}
