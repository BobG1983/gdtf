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

/// A1 (GTW play-test wave 3) + `AC3b` (GTW-297) — the Load-owned
/// `seed_load_fallbacks` (GTW-629 moved it to `crate::states::load::fallbacks`;
/// this affordance stays its one production registrar) seeds the BESPOKE
/// fallbacks — the empty [`LoadedSituation`](crate::states::LoadedSituation),
/// the injuries pair, the prefab registry, and the procgen tuning — ONLY when
/// there is no [`AssetServer`]: with one present (the real GUI launch) it must
/// seed NONE of them, so the bespoke resolves — which only RUN while their
/// resource is ABSENT — WAIT for and populate the real assets instead of an
/// empty seed winning the race (the `AC3b` shadow class). The eight generic
/// content families carry NO arm here at all — their headless fallback rides
/// `register_content_family` (the GTW-629 seam rider, pinned in
/// `gdtf_assets`' seam tests).
///
/// Drives the REAL `seed_load_fallbacks` system on its real `Startup` schedule (the
/// `AutoBattlePlugin` registers it there) — added directly here so the test needs none
/// of the unrelated `RunningState` sub-state machinery `drive_past_menu` requires. One
/// `app.update()` runs `Startup`. Two apps:
///
/// - WITHOUT an `AssetServer` (bare [`MinimalPlugins`]) → every bespoke fallback IS
///   seeded (the headless / asset-less fallbacks that keep the machine traversing
///   `Load`).
/// - WITH an `AssetServer` (a real [`AssetPlugin`] on the task pools) → NONE of the
///   gated fallbacks is seeded (the A1 / `AC3b` fix).
///
/// The theme + tuning seeds are present in BOTH — they are UNGATED, and this test pins
/// that as-is behavior. That lack of a gate is a pre-existing DORMANT DEFECT (surfaced
/// at the GTW-629 gate; the fix rides its own bug ticket): with a real `AssetServer`
/// the two seeds SHADOW the shipped `ui_theme.tuning.ron` / `combat.tuning.ron`,
/// because the theme resolve skips once a `GdtfTheme` exists, the tuning resolve is
/// absence-gated (the GTW-564 hot-RON chain), and the hot-RON redrive fires on
/// `AssetEvent::Modified` only — see the `seed_load_fallbacks` doc. Driven from the
/// test body (`bevy-traps.md` #7 carve-out). Gated on `test-support` because
/// `seed_load_fallbacks` is widened to `pub` only there.
///
/// PIN: the WITH-AssetServer half goes red if ANY gated seed reverts to being inserted
/// unconditionally — the exact regression that re-introduces the A1 empty-battlefield /
/// `AC3b` shadow failures.
#[cfg(feature = "test-support")]
#[test]
fn empty_fallbacks_seeded_only_without_asset_server() {
    use bevy::{asset::AssetPlugin, prelude::*};
    use gdtf_battle_sim::{
        injuries::{InjuryRegistry, InjuryTables},
        level::PrefabRegistry,
        procgen::ProcgenTuning,
        tuning::CombatTuning,
    };
    use gdtf_ui::theme::GdtfTheme;

    use crate::states::{LoadedSituation, seed_load_fallbacks};

    /// Whether every GATED bespoke fallback is present in `app`'s world.
    fn gated_seeds_present(app: &App) -> [bool; 5] {
        [
            app.world().get_resource::<LoadedSituation>().is_some(),
            app.world().get_resource::<InjuryRegistry>().is_some(),
            app.world().get_resource::<InjuryTables>().is_some(),
            app.world().get_resource::<PrefabRegistry>().is_some(),
            app.world().get_resource::<ProcgenTuning>().is_some(),
        ]
    }

    // No AssetServer (bare MinimalPlugins): every bespoke fallback IS seeded.
    {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, seed_load_fallbacks);
        app.update();

        assert!(
            app.world().get_resource::<AssetServer>().is_none(),
            "precondition: MinimalPlugins gives no AssetServer",
        );
        assert_eq!(
            gated_seeds_present(&app),
            [true; 5],
            "without an AssetServer every bespoke fallback (situation / injuries pair / \
             prefabs / procgen tuning) must be seeded (keeps the Load gate satisfiable on \
             an asset-less drive)",
        );
        assert!(
            app.world().get_resource::<GdtfTheme>().is_some()
                && app.world().get_resource::<CombatTuning>().is_some(),
            "the theme / tuning seeds must be present regardless (UNGATED as-is behavior — \
             see the fn doc's dormant-defect note)",
        );
    }

    // With a real AssetServer (AssetPlugin on the task pools): NO gated fallback is
    // seeded — the real bespoke resolves must win.
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
        assert_eq!(
            gated_seeds_present(&app),
            [false; 5],
            "with an AssetServer present the seed must insert NO gated fallback (A1 / AC3b \
             — each bespoke resolve only runs while its resource is ABSENT, so a pre-seeded \
             empty resource would shadow the real asset)",
        );
        assert!(
            app.world().get_resource::<GdtfTheme>().is_some()
                && app.world().get_resource::<CombatTuning>().is_some(),
            "the theme / tuning seeds must still be present with an AssetServer",
        );
    }
}
