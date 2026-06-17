//! Unit tests for the DEV-ONLY auto-enter-battle gate (relocated from the inline
//! `auto_battle` test module, GTW-201).

use super::*;

/// The gate predicate is a pure function of the env var: enabled exactly for
/// the recognised truthy spellings, disabled otherwise. Asserting against a
/// process-global env var is racy across parallel tests, so this exercises the
/// SAME recognition logic [`auto_battle_enabled`] applies, proving the env-var
/// path is wired without mutating the shared environment.
#[test]
fn truthy_spellings_enable_falsey_disable() {
    let recognise = |value: &str| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    };
    for truthy in ["1", "true", "TRUE", "Yes", " on "] {
        assert!(recognise(truthy), "{truthy:?} should enable the affordance");
    }
    for falsey in ["", "0", "false", "no", "off", "maybe"] {
        assert!(
            !recognise(falsey),
            "{falsey:?} should leave the affordance inert",
        );
    }
}

/// `with_enabled` records its flag verbatim and `from_env` agrees with the gate
/// predicate — the two construction paths the wiring + the test use. Gated on
/// `test-support` because `with_enabled` / `enabled` are the test-only inherent
/// surface (absent from the binary build); under `cargo dtest` the workspace's
/// feature unification turns `test-support` on, so this runs.
#[cfg(feature = "test-support")]
#[test]
fn construction_records_the_gate() {
    assert!(AutoBattlePlugin::with_enabled(true).enabled());
    assert!(!AutoBattlePlugin::with_enabled(false).enabled());
    assert_eq!(
        AutoBattlePlugin::from_env().enabled(),
        auto_battle_enabled(),
        "from_env must defer to the env-var gate",
    );
}
