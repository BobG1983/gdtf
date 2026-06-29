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
//!   ([`default_theme`] + [`CombatTuning::default`] unconditionally, plus an empty
//!   [`WeaponRegistry`] + an empty [`ArmorRegistry`] + a default [`LoadedSituation`]
//!   ONLY when there is no `AssetServer`). Under the real GUI launch the `Load` scene
//!   resolves the shipped theme / tuning from assets and `insert_resource`-overwrites
//!   those seeds, and — because its `poll_and_resolve` only RESOLVES the weapon /
//!   armor registries / `situations/skirmish.ron` while those resources are ABSENT —
//!   the registry + situation seeds are deliberately withheld when an `AssetServer` is
//!   present so the real `assets/content/weapons/*.weapon.ron` + `assets/content/armor/*.armor.ron` +
//!   skirmish win (the asset versions are the QA battlefield). With no `AssetServer` /
//!   a failed asset the empty fallbacks still keep the machine traversing `Load`. See
//!   [`seed_load_fallbacks`] for the full rationale (A1 + GTW-297 `AC3b`).
//! - **`Update`** drives the ONE non-automatic transition: the menu does not
//!   auto-advance (GTW-121), so [`drive_past_menu`] sets
//!   `NextState<RunningState>::Game` once [`RunningState::Menu`] rests. Everything
//!   below that — `Game → … → BattleScape → Generation → AnimateIn → BattleRunning`
//!   — descends through the scenes' own `move_on` systems with no further nudging.
//!
//! The drive is idempotent / self-disarming: the `AutoBattleActive` resource it
//! gates on is removed the moment it leaves the menu, so it nudges exactly once.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::{PrefabRegistry2, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
    tuning::CombatTuning,
    weapon::WeaponRegistry,
};
use gdtf_ui::theme::default_theme;

use crate::states::{LoadedSituation, RunningState};

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

crate::support_item! {
    /// Seeds the persistent `Load` resources the auto-battle drive needs so the state
    /// machine can traverse `Load` even without a resolved asset stack.
    ///
    /// Inserts [`default_theme`] + [`CombatTuning::default`] unconditionally, and an empty
    /// [`WeaponRegistry`] + an empty [`ArmorRegistry`] + the empty UUID-keyed
    /// [`TerrainDefRegistry`] / [`UuidThemeRegistry`] / [`PrefabRegistry2`] + a default (EMPTY)
    /// [`LoadedSituation`] **only when no [`AssetServer`] is present** (a headless / asset-less
    /// build). Under the real GUI launch the `Load` scene later `insert_resource`-overwrites
    /// the theme / tuning with the shipped assets (the real theme + tuning), and — crucially —
    /// its `poll_and_resolve` only RESOLVES the registries / situation from
    /// `assets/content/weapons/*.weapon.ron` + `assets/content/armor/*.armor.ron` +
    /// `assets/terrain/<theme>/*.terrain_def.ron` + `situations/skirmish.ron` while those
    /// resources are still ABSENT, so the empty seeds must NOT be present for the real assets
    /// to win.
    /// Runs once in `Startup` (before the first `Update`, hence before `Load` resolves), so
    /// the unconditional seeds are in place no matter how the assets resolve, and the
    /// gated ones are absent whenever a real asset stack can resolve the genuine articles.
    ///
    /// **A1 (GTW play-test wave 3) — and GTW-297 (`AC3b`).** The default [`LoadedSituation`]
    /// is the EMPTY situation (zero gangers) and the default [`WeaponRegistry`] holds zero
    /// weapons. Post-GTW-261 a present [`LoadedSituation`] SATISFIES the Load→Intro gate,
    /// and per GTW-257 `poll_and_resolve` only runs
    /// `resolve_weapons` / `resolve_situation` `if` the registry / situation is still
    /// ABSENT. So seeding EITHER empty fallback UNCONDITIONALLY shadowed the real load
    /// under `DefaultPlugins`: the empty situation won the GTW-261 gate race (auto-battle
    /// dropped into an empty battlefield), and the empty registry made `resolve_weapons`
    /// skip loading `assets/content/weapons/*.weapon.ron` entirely — so battle setup's
    /// `weapons.spec("stub_pistol")` returned `None` and aborted with
    /// [`WeaponNotFound`](gdtf_battle_sim::situation::BattleSetupError), never reaching
    /// `BattleRunning` (a black screen). Gating BOTH empty seeds on the [`AssetServer`]
    /// being ABSENT (SYMMETRIC seeds) means: with an asset stack present (the real GUI
    /// launch) NONE of the empty fallbacks is inserted, so `poll_and_resolve` WAITS for and
    /// populates the real skirmish + the real weapon registry + the real armor registry + the
    /// real UUID-keyed terrain / theme / prefab registries; without one (a headless asset-less
    /// drive) all empty fallbacks are still seeded so the machine keeps traversing `Load` and
    /// the GTW-257 / GTW-269 / GTW-487 / GTW-489 Load→Intro gate (which requires a
    /// [`WeaponRegistry`], an [`ArmorRegistry`], a [`TerrainDefRegistry`], a
    /// [`UuidThemeRegistry`], and a [`PrefabRegistry2`]) is satisfied. The
    /// unconditional theme + tuning seeds are
    /// overwritten in place by the resolved assets (their resolve is unconditional), so
    /// they need no such gate.
    ///
    /// Param-only (`bevy-traps.md` #7): [`Commands`] + an `Option<Res<AssetServer>>` probe
    /// (`Option` so it is panic-free whether or not the asset stack is wired) — no
    /// `&mut World`.
    ///
    /// Widened to `pub` under `test-support` (the A1 test drives it directly as a `Startup`
    /// system — the real registered path minus the unrelated `drive_past_menu` that needs
    /// the `RunningState` sub-state machinery), `pub(crate)` otherwise, keeping the binary
    /// `unreachable_pub`-clean.
    fn seed_load_fallbacks(asset_server: Option<Res<AssetServer>>, mut commands: Commands) {
        commands.insert_resource(default_theme());
        commands.insert_resource(CombatTuning::default());
        // A1 / AC3b — only seed the empty fallback registries + situation when there is NO
        // AssetServer. With one present the real `assets/content/weapons/*.weapon.ron` +
        // `assets/content/armor/*.armor.ron` registries and `situations/skirmish.ron` must win:
        // `poll_and_resolve` only resolves them while ABSENT, so a pre-seeded empty resource
        // would shadow the real load (the registry shadow is the AC3b WeaponNotFound bug).
        // The three seeds are SYMMETRIC (GTW-269 adds the armor registry to the set).
        if asset_server.is_none() {
            commands.insert_resource(WeaponRegistry::default());
            commands.insert_resource(ArmorRegistry::default());
            // GTW-437: the InjuryRegistry is a gate-blocking resource too; seed the empty
            // fallback when there is no AssetServer so headless walks still reach Intro
            // (the A1 / AC3b pattern for the injury registry).
            commands.insert_resource(InjuryRegistry::default());
            // GTW-438: the InjuryTables is read by the fire path's `roll_injury` (the
            // first reader), so seed the empty fallback alongside the registry for parity
            // with the resolve path — else `dispatch_fire`'s `Res<InjuryTables>` would
            // panic on a missing resource in an asset-less headless drive. An empty table
            // means the roll finds no bucket and still takes-then-discards its one draw.
            commands.insert_resource(InjuryTables::default());
            // GTW-415: the GangRegistry is a gate-blocking resource too; seed the empty
            // fallback when there is no AssetServer so headless walks still reach Intro
            // (the A1 / AC3b pattern for the gang registry). With an AssetServer present
            // the real `assets/content/gangs/*.gang.ron` registry must win — so this is
            // gated on `is_none()` exactly like the weapon/armor registries (else the
            // empty seed would shadow `resolve_gangs`, which only runs while the registry
            // is ABSENT — the AC3b shadow class).
            commands.insert_resource(GangRegistry::default());
            // GTW-489: the UUID-keyed PrefabRegistry2 is a gate-blocking resource; seed the
            // empty fallback when there is no AssetServer so headless walks still reach Intro
            // (the A1 / AC3b pattern). With an AssetServer present the real
            // `assets/maps/**/*.prefab_v2.ron` resolve must win — so this is gated on
            // `is_none()` exactly like the other registries (else the empty seed would shadow
            // resolve_prefabs_v2, which only runs while the registry is ABSENT). GTW-494: this
            // is the ONLY prefab registry (the legacy prefab-registry seed was retired).
            commands.insert_resource(PrefabRegistry2::default());
            // GTW-487: the UUID-keyed TerrainDefRegistry + UuidThemeRegistry are gate-blocking
            // too; seed the empty fallbacks when there is no AssetServer so headless walks still
            // reach Intro (the A1 / AC3b pattern). With an AssetServer present the real
            // per-theme `terrain/` resolve must win — so this is gated on `is_none()` exactly
            // like the other registries (else the empty seed would shadow resolve_terrain_defs /
            // resolve_theme_defs, which only run while their registry is ABSENT). GTW-494: these
            // are the ONLY terrain / theme registries (the legacy terrain / theme registry
            // seeds were retired).
            commands.insert_resource(TerrainDefRegistry::default());
            commands.insert_resource(UuidThemeRegistry::default());
            commands.insert_resource(LoadedSituation::new(
                gdtf_battle_sim::situation::Situation::default(),
            ));
        }
    }
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
