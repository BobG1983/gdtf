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

/// A1 (GTW play-test wave 3) — `seed_load_fallbacks` seeds the EMPTY
/// [`LoadedSituation`](crate::scenes::LoadedSituation) fallback ONLY when there is no
/// [`AssetServer`]: with one present (the real GUI launch) it must NOT, so the GTW-261
/// Load→Intro gate WAITS for the real `situations/skirmish.ron` instead of the empty
/// seed winning the race.
///
/// Drives the REAL `seed_load_fallbacks` system on its real `Startup` schedule (the
/// `AutoBattlePlugin` registers it there) — added directly here so the test needs none
/// of the unrelated `RunningState` sub-state machinery `drive_past_menu` requires. One
/// `app.update()` runs `Startup`. Two apps:
///
/// - WITHOUT an `AssetServer` (bare [`MinimalPlugins`]) → `LoadedSituation` IS seeded
///   (the headless / asset-less fallback that keeps the machine traversing `Load`).
/// - WITH an `AssetServer` (a real [`AssetPlugin`] on the task pools) → `LoadedSituation`
///   is NOT seeded (the A1 fix).
///
/// The theme + tuning + weapon-registry seeds are present in BOTH (unchanged by A1).
/// Driven from the test body (`bevy-traps.md` #7 carve-out). Gated on `test-support`
/// because `seed_load_fallbacks` is widened to `pub` only there.
#[cfg(feature = "test-support")]
#[test]
fn empty_situation_seed_only_without_asset_server() {
    use bevy::{asset::AssetPlugin, prelude::*};
    use gdtf_battle_sim::{tuning::CombatTuning, weapon::WeaponRegistry};
    use gdtf_ui::theme::GdtfTheme;

    use crate::scenes::LoadedSituation;

    // No AssetServer (bare MinimalPlugins): the empty fallback situation IS seeded.
    {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, seed_load_fallbacks);
        app.update();

        assert!(
            app.world().get_resource::<AssetServer>().is_none(),
            "precondition: MinimalPlugins gives no AssetServer",
        );
        assert!(
            app.world().get_resource::<LoadedSituation>().is_some(),
            "without an AssetServer the empty LoadedSituation fallback must be seeded",
        );
        assert!(
            app.world().get_resource::<GdtfTheme>().is_some()
                && app.world().get_resource::<CombatTuning>().is_some()
                && app.world().get_resource::<WeaponRegistry>().is_some(),
            "the theme / tuning / weapon-registry seeds must be present regardless of A1",
        );
    }

    // With a real AssetServer (AssetPlugin on the task pools): NO LoadedSituation seed.
    {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_systems(Startup, seed_load_fallbacks);
        app.update();

        assert!(
            app.world().get_resource::<AssetServer>().is_some(),
            "precondition: AssetPlugin provides an AssetServer",
        );
        assert!(
            app.world().get_resource::<LoadedSituation>().is_none(),
            "with an AssetServer present the seed must NOT insert an empty LoadedSituation \
             (A1 — the real skirmish must win the GTW-261 Load gate)",
        );
        assert!(
            app.world().get_resource::<GdtfTheme>().is_some()
                && app.world().get_resource::<CombatTuning>().is_some()
                && app.world().get_resource::<WeaponRegistry>().is_some(),
            "the theme / tuning / weapon-registry seeds must still be present with an AssetServer",
        );
    }
}
