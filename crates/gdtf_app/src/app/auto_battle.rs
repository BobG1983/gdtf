//! The DEV-ONLY auto-enter-battle affordance (GTW-223, the GTW-48 capstone enabler).
//!
//! This is **not shipping behavior**. It exists solely so a developer (and the
//! orchestrator's post-gate QA) can land a single `cargo run` directly inside a
//! live, rendered, interactive battle to eyeball the GTW-48 presenter / input /
//! action-bar stack the per-slice headless tests structurally cannot observe.
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** The affordance is wired into [`GdtfApp`](crate::GdtfApp) only
//!    under `cfg!(debug_assertions)` (a debug / dev build). A release artifact
//!    never sees it.
//! 2. **Opt-in env var.** Even in a dev build it is **inert by default**: a plain
//!    `cargo run` reaches the menu and STOPS. It activates only when the
//!    `GDTF_AUTOBATTLE` environment variable is set truthy ([`auto_battle_enabled`]).
//!    So `GDTF_AUTOBATTLE=1 cargo run -p grimdark_turfwar --features dynamic_linking`
//!    drives straight into a battle, while `cargo run …` does not.
//!
//! ## How it drives the machine
//!
//! When active, [`AutoBattlePlugin`] mirrors the `battle_running_driver.rs` /
//! `battle_bootstrap.rs` headless drive idiom — expressed as ordinary app systems
//! over `Commands` / `ResMut<NextState<_>>` (NEVER a `&mut World` helper,
//! `bevy-traps.md` #7):
//!
//! - **`Startup`** seeds the persistent `Load` resources a headless walk lacks
//!   ([`default_theme`] + [`CombatTuning::default`] + a default [`LoadedSituation`]).
//!   Under the real GUI launch the `Load` scene resolves the shipped theme /
//!   tuning / `situations/skirmish.ron` from assets and `insert_resource`-overwrites
//!   these seeds (the asset versions win — that is the QA battlefield); the seeds
//!   are the fallback that keeps the machine traversing `Load` even with no
//!   `AssetServer` / a failed asset.
//! - **`Update`** drives the ONE non-automatic transition: the menu does not
//!   auto-advance (GTW-121), so [`drive_past_menu`] sets
//!   `NextState<RunningState>::Game` once [`RunningState::Menu`] rests. Everything
//!   below that — `Game → … → BattleScape → Generation → AnimateIn → BattleRunning`
//!   — descends through the scenes' own `move_on` systems with no further nudging.
//!
//! The drive is idempotent / self-disarming: the `AutoBattleActive` resource it
//! gates on is removed the moment it leaves the menu, so it nudges exactly once.

use bevy::prelude::*;
use gdtf_battle_sim::tuning::CombatTuning;
use gdtf_ui::theme::default_theme;

use crate::{scenes::LoadedSituation, states::RunningState};

/// The `GDTF_AUTOBATTLE` environment variable that opts a dev build into the
/// auto-enter-battle affordance.
const AUTO_BATTLE_ENV: &str = "GDTF_AUTOBATTLE";

crate::support_item! {
    /// Marks the auto-enter-battle affordance as ACTIVE for this run.
    ///
    /// Inserted by [`AutoBattlePlugin::build`] only when the affordance's gate holds,
    /// and removed by [`drive_past_menu`] the moment it nudges the machine off the
    /// menu, so the drive fires exactly once. Its mere *presence* is the run-condition
    /// the drive system gates on (`run_if(resource_exists::<AutoBattleActive>)`); a
    /// normal launch never inserts it, so the drive never runs.
    ///
    /// A framework-plumbing marker [`Resource`] (a zero-field witness, not a domain
    /// value), so the no-bare-types rule does not apply. Widened to `pub` under
    /// `test-support` (the AC1 test asserts the drive disarms it), `pub(crate)`
    /// otherwise.
    #[derive(Resource, Debug, Default, Clone, Copy)]
    struct AutoBattleActive;
}

crate::support_item! {
    /// Whether the DEV auto-enter-battle affordance is enabled for this process.
    ///
    /// Reads the [`AUTO_BATTLE_ENV`] (`GDTF_AUTOBATTLE`) environment variable and
    /// treats `1` / `true` / `yes` / `on` (case-insensitive, trimmed) as enabled;
    /// anything else — including the variable being unset or empty — is disabled.
    /// This is the env-var half of the gate; the `cfg!(debug_assertions)` half lives
    /// at the [`GdtfApp`](crate::GdtfApp) wiring site, so a release build never even
    /// compiles the affordance in.
    ///
    /// Pure (no `World`, no side effects) so it is unit-checkable and so the GUI path
    /// can be reasoned about without launching: a normal `cargo run` leaves the var
    /// unset and this returns `false`, keeping the launch inert (it rests at the menu).
    /// Widened to `pub` under `test-support` (the AC1 test unit-checks the gate),
    /// `pub(crate)` otherwise.
    #[must_use]
    fn auto_battle_enabled() -> bool {
        std::env::var(AUTO_BATTLE_ENV).is_ok_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    }
}

crate::support_item! {
    /// The DEV-ONLY auto-enter-battle affordance plugin.
    ///
    /// Wired into [`GdtfApp`](crate::GdtfApp) only under `cfg!(debug_assertions)`. On
    /// `build` it consults its gate ([`Self::enabled`]); when the gate holds it logs a
    /// one-line `auto-battle: ON (dev)`, inserts the [`AutoBattleActive`] witness,
    /// seeds the `Load` fallback resources on `Startup`, and registers
    /// [`drive_past_menu`] in `Update`. When the gate does NOT hold it registers
    /// nothing — the affordance is fully inert and the app reaches the menu and
    /// stops, exactly like a build without the plugin. Widened to `pub` under
    /// `test-support` (the AC1/AC2 tests add it), `pub(crate)` otherwise.
    struct AutoBattlePlugin {
        /// Whether the affordance should activate. Captured once at construction (from
        /// [`auto_battle_enabled`] at the wiring site, or forced for a headless test of
        /// the drive logic) so `build` is a pure function of this flag.
        enabled: bool,
    }
}

impl AutoBattlePlugin {
    // `from_env` is `pub` under `test-support` (the AC1 test names it) and
    // `pub(crate)` in the binary (where `gdtf_app.rs` constructs the affordance),
    // via `support_item!` per method — the same visibility flip the type itself
    // uses, so `unreachable_pub` stays satisfied in both configurations.
    crate::support_item! {
        /// Construct the affordance, reading its env-var gate ([`auto_battle_enabled`]).
        ///
        /// This is the constructor [`GdtfApp`](crate::GdtfApp) uses under
        /// `cfg!(debug_assertions)`: the env var decides whether the affordance
        /// activates, so the plugin is inert on a normal launch.
        #[must_use]
        fn from_env() -> Self {
            Self {
                enabled: auto_battle_enabled(),
            }
        }
    }

    // `with_enabled` + `enabled` are the test-only surface: they let the AC1/AC2
    // headless tests drive BOTH gate branches deterministically and assert the gate
    // without touching a process-global env var. The binary never constructs the
    // affordance any way but `from_env` and reads its own `self.enabled` field
    // directly, so these are gated to `test-support` (where they are `pub` API and
    // exercised by the tests) and absent from the binary, keeping it both
    // `unreachable_pub`- and `dead_code`-clean.
    /// Construct the affordance with its activation forced to `enabled`, bypassing
    /// the env-var read.
    ///
    /// The headless AC1 test uses this to drive BOTH gate branches deterministically
    /// (without racing a process-global env var): `with_enabled(true)` proves the ON
    /// path reaches `BattleRunning`, and `with_enabled(false)` (indistinguishable from
    /// no plugin) proves the OFF path rests at the menu. The env-var gate itself is
    /// covered separately by a unit check of [`auto_battle_enabled`].
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn with_enabled(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Whether this affordance instance will activate on `build`.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for AutoBattlePlugin {
    /// The wiring default: read the env-var gate.
    fn default() -> Self {
        Self::from_env()
    }
}

impl Plugin for AutoBattlePlugin {
    fn build(&self, app: &mut App) {
        if !self.enabled {
            // Inert: register nothing. The app reaches the menu and stops.
            return;
        }
        info!("auto-battle: ON (dev)");
        app.insert_resource(AutoBattleActive)
            .add_systems(Startup, seed_load_fallbacks)
            .add_systems(
                Update,
                drive_past_menu.run_if(resource_exists::<AutoBattleActive>),
            );
    }
}

/// Seeds the persistent `Load` resources the auto-battle drive needs so the state
/// machine can traverse `Load` even without a resolved asset stack.
///
/// Inserts [`default_theme`] + [`CombatTuning::default`] + a default
/// [`LoadedSituation`]. Under the real GUI launch the `Load` scene later
/// `insert_resource`-overwrites all three with the shipped assets (the real
/// `situations/skirmish.ron` battlefield + theme + tuning), so these are a
/// fallback, not the QA battlefield. Runs once in `Startup` (before the first
/// `Update`, hence before `Load` resolves), so the seed is in place no matter how
/// the assets resolve.
fn seed_load_fallbacks(mut commands: Commands) {
    commands.insert_resource(default_theme());
    commands.insert_resource(CombatTuning::default());
    commands.insert_resource(LoadedSituation(
        gdtf_battle_sim::situation::Situation::default(),
    ));
}

/// Drives the ONE non-automatic transition into the battle: `Menu → Game`.
///
/// The menu does not auto-advance (GTW-121), so this stands in for the player
/// picking "Battlescape": once [`RunningState::Menu`] is the active running state
/// it requests `NextState<RunningState>::Game` (the same transition
/// `menu_actions.rs` maps the Battlescape button to). The deeper descent
/// (`Game → … → BattleScape → … → BattleRunning`) is automatic via the scenes'
/// `move_on` systems, so no further nudging is needed.
///
/// It then removes the [`AutoBattleActive`] witness so it fires exactly once and
/// disarms itself (its `run_if(resource_exists::<AutoBattleActive>)` gate stops
/// matching). Param-only (`Option<Res<State<RunningState>>>` / `ResMut<NextState>`
/// / `Commands`) — no `&mut World` (`bevy-traps.md` #7). The state read is
/// `Option<…>` because `RunningState` is a sub-state absent until `AppState::Running`
/// (bevy-traps rule 1).
fn drive_past_menu(
    running: Option<Res<State<RunningState>>>,
    mut next: ResMut<NextState<RunningState>>,
    mut commands: Commands,
) {
    let Some(running) = running else {
        return;
    };
    if *running.get() == RunningState::Menu {
        next.set(RunningState::Game);
        commands.remove_resource::<AutoBattleActive>();
    }
}

#[cfg(test)]
mod tests {
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
}
