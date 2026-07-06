//! Unit tests for the DEV-ONLY auto-enter-battle gate (relocated from the inline
//! `auto_battle` test module, GTW-201).

use super::{plugin::recognised_truthy, *};

/// The gate predicate is a pure function of the env var: enabled exactly for
/// the recognised truthy spellings, disabled otherwise. Asserting against a
/// process-global env var is racy across parallel tests, so this drives the
/// REAL [`recognised_truthy`] parser [`auto_battle_enabled`] applies (GTW-622
/// C8: the former local copy is folded into that production fn), proving the
/// env-var path's recognition logic without mutating the shared environment.
#[test]
fn truthy_spellings_enable_falsey_disable() {
    for truthy in ["1", "true", "TRUE", "Yes", " on "] {
        assert!(
            recognised_truthy(truthy),
            "{truthy:?} should enable the affordance"
        );
    }
    for falsey in ["", "0", "false", "no", "off", "maybe"] {
        assert!(
            !recognised_truthy(falsey),
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

/// A1 (GTW play-test wave 3) + `AC3b` (GTW-297) — `seed_load_fallbacks` seeds the EMPTY
/// [`LoadedSituation`](crate::states::LoadedSituation) **and** the empty
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) fallbacks ONLY when there
/// is no [`AssetServer`]: with one present (the real GUI launch) it must NOT seed EITHER,
/// so the Load scene's `poll_and_resolve` — which only RESOLVES the situation / registry
/// while those resources are ABSENT — WAITS for and populates the real
/// `content/situations/skirmish.ron` + `assets/content/weapons/ranged/*.weapon.ron` instead of an empty seed
/// winning the race (the empty registry shadow is the `AC3b` `WeaponNotFound` black-screen
/// bug). The two seeds are SYMMETRIC.
///
/// Drives the REAL `seed_load_fallbacks` system on its real `Startup` schedule (the
/// `AutoBattlePlugin` registers it there) — added directly here so the test needs none
/// of the unrelated `RunningState` sub-state machinery `drive_past_menu` requires. One
/// `app.update()` runs `Startup`. Two apps:
///
/// - WITHOUT an `AssetServer` (bare [`MinimalPlugins`]) → `LoadedSituation` AND
///   `WeaponRegistry` ARE seeded (the headless / asset-less fallbacks that keep the
///   machine traversing `Load`).
/// - WITH an `AssetServer` (a real [`AssetPlugin`] on the task pools) → NEITHER
///   `LoadedSituation` NOR `WeaponRegistry` is seeded (the A1 / `AC3b` fix).
///
/// The theme + tuning seeds are present in BOTH (they are resolved UNCONDITIONALLY by the
/// Load scene, overwriting the seed in place, so they need no such gate). Driven from the
/// test body (`bevy-traps.md` #7 carve-out). Gated on `test-support` because
/// `seed_load_fallbacks` is widened to `pub` only there.
///
/// PIN: the WITH-AssetServer half goes red if EITHER empty seed reverts to being inserted
/// unconditionally — the exact regression that re-introduces the A1 empty-battlefield /
/// `AC3b` `WeaponNotFound` failures.
#[cfg(feature = "test-support")]
#[test]
fn empty_fallbacks_seeded_only_without_asset_server() {
    use bevy::{asset::AssetPlugin, prelude::*};
    use gdtf_battle_sim::{tuning::CombatTuning, weapon::WeaponRegistry};
    use gdtf_ui::theme::GdtfTheme;

    use crate::states::LoadedSituation;

    // No AssetServer (bare MinimalPlugins): both empty fallbacks ARE seeded.
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
            app.world().get_resource::<WeaponRegistry>().is_some(),
            "without an AssetServer the empty WeaponRegistry fallback must be seeded (keeps the \
             GTW-257 Load gate satisfiable on an asset-less drive)",
        );
        assert!(
            app.world().get_resource::<GdtfTheme>().is_some()
                && app.world().get_resource::<CombatTuning>().is_some(),
            "the theme / tuning seeds must be present regardless (resolved unconditionally)",
        );
    }

    // With a real AssetServer (AssetPlugin on the task pools): NEITHER empty fallback is
    // seeded — the real folder / situation resolve must win.
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
            app.world().get_resource::<WeaponRegistry>().is_none(),
            "with an AssetServer present the seed must NOT insert an empty WeaponRegistry \
             (AC3b — else poll_and_resolve skips resolve_weapons and battle setup fails with \
             WeaponNotFound)",
        );
        assert!(
            app.world().get_resource::<GdtfTheme>().is_some()
                && app.world().get_resource::<CombatTuning>().is_some(),
            "the theme / tuning seeds must still be present with an AssetServer",
        );
    }
}
