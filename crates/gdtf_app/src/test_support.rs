//! Test-support surface for building and asserting on the real GDTF state
//! machine from an external crate.
//!
//! This module is gated behind the `test-support` feature and is absent from
//! release builds — the production public API of `gdtf_app` is exactly
//! [`crate::GdtfApp`]. It re-exports the otherwise crate-internal state enums
//! and [`ScenesPlugin`] so an out-of-crate harness can name them, and provides
//! [`register_headless`] to wire the full headless state stack in the correct
//! parent-before-child order.
//!
//! # The one-hop panel ledger (GTW-569)
//!
//! Every UI panel/scene that exposes test-only markers owns ONE
//! `#[cfg(feature = "test-support")] pub(crate) mod test_support` submodule in
//! its `mod.rs`, and THIS ledger re-exports those items directly from that
//! submodule — no re-export climb through the intermediate `mod.rs` files.
//! Exporting a new panel marker is exactly **2 edits**:
//!
//! 1. add the marker to the panel's own `test_support` submodule (or add that
//!    submodule, plus a `pub(crate)` widening of the module path if the panel
//!    is new — see `crate::support`);
//! 2. add the marker, by explicit name, to the `pub use` ledger below.
//!
//! The ledger names every item EXPLICITLY — never a glob (`::*`) — so a
//! duplicate marker name across two panels fails to compile HERE (E0252)
//! instead of silently shadowing.
//!
//! The state enums ([`AppState`] / [`RunningState`] / [`GameState`] /
//! [`BattleScapeState`] / [`AfterMathState`]), [`ScenesPlugin`],
//! [`LoadedSituation`], `BottomBarRoot`, and the `dev::auto_battle` items are
//! deliberately NOT part of the panel ledger: they keep their `support_use!`
//! climbs because the GTW-321 co-location contract keeps `crate::states::<Enum>`
//! nameable at the states root and/or the production binary reads the same
//! re-export (see `crate::support`).

use bevy::{
    app::App,
    asset::AssetServer,
    ecs::system::{Commands, Res},
    input::InputPlugin,
    state::{
        app::{AppExtStates, StatesPlugin},
        state::State,
    },
};
use gdtf_battle_sim::tuning::GangerStatTuning;
pub use gdtf_ui::UiPlugin;

/// Reads the current [`AppState`] — the ONE read-back helper the integration-test
/// files share (GTW-576), replacing the per-file `app_state` copies. Panics only if
/// the app never registered [`AppState`], which for a GDTF harness is a broken
/// fixture, not a runtime condition.
#[must_use]
pub fn app_state(app: &App) -> AppState {
    app.world().resource::<State<AppState>>().get().clone()
}

/// True once `Load` has RELEASED — the machine reached [`AppState::Intro`] or the
/// state past it ([`AppState::Running`]). The ONLY sanctioned "did Load release"
/// probe (GTW-601).
///
/// [`AppState::Intro`] is a TRANSIENT stop: its `FixedUpdate` marker+advance
/// scaffold queues Intro's own move-on as soon as it runs, and one long
/// asset-I/O frame can accumulate >= 2 fixed timesteps, so a single
/// `App::update` can traverse Intro entirely. Worse, the update-THEN-check
/// advance helpers (`advance_until_resource_exists`) consume one update even
/// when their signal already fired — when the awaited registry is the LAST
/// Load-gate resource, that throwaway update is exactly the frame `Load`
/// releases. An `app_state(app) == AppState::Intro` equality probe therefore
/// races a window that can be ZERO frames wide and only ever observes
/// `Running` (the GTW-589 flake, measured at 50% on one test under parallel
/// cargo load — GTW-601). The state machine is linear
/// (`Init -> Load -> Intro -> Running`), so accepting `Running` still proves
/// Intro was traversed and the Load gate held until it released.
#[must_use]
pub fn load_released(app: &App) -> bool {
    matches!(app_state(app), AppState::Intro | AppState::Running)
}

/// The ONE test-side Load-gate seed source (GTW-580): the production
/// [`seed_load_fallbacks`] plus the [`GangerStatTuning`] delta — together with
/// the GTW-629 seam rider this covers the WHOLE `transition_to_intro` gate.
///
/// Since GTW-629 the production seed carries the BESPOKE loads only (injuries
/// pair / prefab / situation / tunings / theme); the eight generic content
/// families are seeded at APP BUILD by their own `register_content_family`
/// line (the seam's headless-fallback rider), so under a `GdtfTestAppBuilder`
/// app the family registries are already present before this system ever runs.
///
/// Every tier-(a) `MinimalPlugins` load test seeds the gate through this system
/// (via the shared `tests/load_suite/gate.rs` helpers) instead of hand-stamping
/// its own `insert_resource` block, so adding a gate-blocking BESPOKE resource
/// touches the production seed only (a new content FAMILY touches nothing —
/// its registration line is its fallback) and NO pre-existing test file.
///
/// # Why the delta lives here and not in `seed_load_fallbacks`
///
/// `seed_load_fallbacks` has a PRODUCTION caller — `AutoBattlePlugin` registers
/// it at `Startup` — so widening it is an auto-battle runtime behavior change,
/// out of GTW-580's scope. Its asset-less branch omits the gate-blocking
/// [`GangerStatTuning`] (a latent gap: the GTW-384 gate clause landed without the
/// matching fallback seed, invisible in production because the GUI launch always
/// has an `AssetServer`), so swapped in unmodified it leaves a tier-(a) walk
/// resting in `Load` forever. This wrapper closes exactly that delta, mirroring
/// the production seed's `AssetServer`-absent gating (the `AC3b` seed-shadow rule:
/// with a server present NOTHING extra is seeded, so the real resolves win).
///
/// The production seed also inserts an [`InjuryTables`](gdtf_battle_sim::injuries::InjuryTables)
/// the gate does NOT require (the `dispatch_fire` panic guard for asset-less full
/// walks). It rides along here deliberately: the seeded set is a strict SUPERSET
/// of the gate set, and a non-gate resource can neither hold nor release the
/// `transition_to_intro` run-condition chain, so gate assertions are unaffected.
pub fn seed_load_gate(asset_server: Option<Res<AssetServer>>, mut commands: Commands) {
    // Mirror the production seed's gating: only the asset-less branch seeds the
    // stand-in, so a real-asset harness still resolves the shipped stat tuning.
    if asset_server.is_none() {
        commands.insert_resource(GangerStatTuning::default());
    }
    seed_load_fallbacks(asset_server, commands);
}

// GTW-736: the QA net_qa server's transport + router test surface. Gated exactly like the
// module itself — `all(debug_assertions, feature = "net_qa")` — so a build without the
// feature (the CI static suite) omits both the module and this re-export. `NetQaPlugin`
// carries the `with_channels` / `spawn_test_listener` test constructors; the rest are the
// channel + config types the GTW-736 integration suite names directly.
#[cfg(all(debug_assertions, feature = "net_qa"))]
pub use crate::dev::net_qa::{
    IncomingRequest, NET_QA_PROTOCOL_VERSION, NetIoTimeout, NetQaPlugin, NetQaPort, Responder,
};
// The GTW-655 procgen-stepper command/latch types + its forced-enable test constructor —
// `dev_tools`-gated because the whole stepper module compiles out unless that feature is on
// (it pulls in `bevy_egui`). Retired the GTW-434/GTW-498 procgen-visualizer ledger entry this
// module replaces (its `debug_assertions`-gated scene was deleted in the same change).
#[cfg(feature = "dev_tools")]
pub use crate::dev::procgen_stepper::{
    AutoRunning, AutoStepDelay, PendingStepCommand, ProcgenStepperPlugin, StepCommand,
    stage_summary, stepper_enabled,
};
pub use crate::{
    dev::auto_battle::{AutoBattleActive, AutoBattlePlugin, auto_battle_enabled},
    states::{
        AfterMathState, AppState, BattleScapeState, GameState, LoadedSituation, RunningState,
        ScenesPlugin,
        running::{
            game::battlescape::{
                BottomBarRoot,
                action_bar::test_support::{
                    AimToggleButton, EndTurnButton, FleeButton, LevelDownButton, LevelUpButton,
                    ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
                    StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
                    StanceStandingButton,
                },
                battle_running::test_support::BattleRunningComplete,
                combat_log::test_support::{CombatLogLine, CombatLogRoot},
                contextual_panel::test_support::{
                    ContextualPanelRoot, EnterEmplacementButton, ExecuteButton,
                    ExitEmplacementButton, MeleeButton, OpenDoorButton, ShoveButton,
                    StabilizeButton, ThrowGrenadeButton,
                },
                generation::loading_screen::test_support::LoadingScreenRoot,
                inspect_panel::test_support::{
                    InspectObjectBar, InspectObjectBlock, InspectObjectHardness,
                    InspectObjectHeight, InspectObjectProtection, InspectObjectText,
                    InspectPanelRoot, InspectStatBlockHost,
                },
                select_cycle::test_support::{SelectCycleRoot, SelectNextButton, SelectPrevButton},
                stat_block::test_support::{
                    StatFaction, StatHpBar, StatHpLabel, StatInjuryLine, StatInjuryList, StatName,
                    StatPortrait, StatStance, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList,
                    StatWoundsPips, portrait_index_for_name,
                },
                status_panel::stability_readout::test_support::StabilityBar,
                weapon_panel::test_support::{
                    AimLabel, AimPanel, CombinedWeaponPanel, ReloadButton, WeaponContent,
                    WeaponImage, WeaponItemButton, WeaponItemPanel, WeaponMagazineText,
                    WeaponNameText, WeaponPanelRoot,
                },
            },
            menu::test_support::{
                BattlescapeButton, HiveScapeButton, MenuTitle, OptionsButton, QuitButton,
            },
        },
        seed_load_fallbacks,
    },
};

/// Registers the full headless GDTF state stack on `app`.
///
/// Wires, in order:
/// 1. [`StatesPlugin`] — installs the `StateTransition` schedule the state
///    machinery needs (normally pulled in by `DefaultPlugins`, which a headless
///    test does not add).
/// 2. [`AppState`] as the top-level state.
/// 3. The sub-states in parent-before-child order — [`RunningState`],
///    [`GameState`], [`BattleScapeState`], [`AfterMathState`] — because a
///    `SubStates` type must be registered after its `#[source(...)]` parent (see
///    `.claude/rules/bevy-traps.md` rule 5).
/// 4. [`ScenesPlugin`], which adds every scene plugin.
/// 5. [`InputPlugin`] — the render-free Bevy input layer. `MinimalPlugins`
///    omits it, but [`UiPlugin`]'s focus-nav bridge reads the
///    `ButtonInput<KeyCode>` resource / keyboard message buffers that
///    `InputPlugin` registers; without it the bridge has nothing to read.
///    `InputPlugin` is headless-safe (no window/render). `DefaultPlugins`
///    already includes it, so [`crate::GdtfApp`] needs no change. (As of Bevy
///    0.19 the `InputDispatchPlugin` that owns `InputFocus` ships in
///    `DefaultPlugins`, not in `UiPlugin`; this `MinimalPlugins` harness never
///    gets it — only the focus-nav bridge + `DirectionalNavigationPlugin`.)
/// 6. [`UiPlugin`], the UI registration seam — added here to keep this headless
///    path a faithful mirror of [`crate::GdtfApp`], which also adds it. A
///    harness test can then assert `is_plugin_added::<UiPlugin>()` and prove the
///    real registration path wires the UI, not merely that `gdtf_ui` compiles.
///
/// This intentionally does **not** add `MinimalPlugins`; the headless app
/// builder composes those around `register_headless`. `init_state` /
/// `add_sub_state` are idempotent in Bevy 0.18, so the sub-state registrations
/// that `ScenesPlugin`'s scene plugins also perform are a no-op the second time.
pub fn register_headless(app: &mut App) {
    app.add_plugins(StatesPlugin);
    app.init_state::<AppState>();
    // The sub-states (RunningState/GameState/BattleScapeState/AfterMathState) are
    // registered by ScenesPlugin's scene plugins, in parent-before-child order
    // (each scene plugin `add_*state`s before its `add_plugins` pulls in child
    // plugins — bevy-traps rule 5). Registering them here too is redundant and
    // logs `WARN bevy_state::app: Sub state X is already initialized` once each —
    // `add_sub_state` is idempotent-but-warns (it no-ops on the second call but
    // emits the warning). So we rely solely on ScenesPlugin. (GTW-139)
    app.add_plugins(InputPlugin);
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}

/// Registers the GDTF state stack and scene/UI plugins onto an app that **already
/// has `DefaultPlugins`** (i.e. the official `no_renderer.rs` headless set), so
/// the real `Load` orchestration runs against a live `AssetServer`.
///
/// Unlike [`register_headless`], this does **not** add `StatesPlugin` or
/// `InputPlugin`: `DefaultPlugins` already includes both, and a Bevy plugin may
/// only be added once (a duplicate add panics). It mirrors exactly what
/// [`crate::GdtfApp`] does on top of `DefaultPlugins`:
///
/// 1. [`AppState`] as the top-level state ([`init_state`](AppExtStates::init_state)).
/// 2. [`ScenesPlugin`], whose scene plugins register their own sub-states in
///    parent-before-child order (idempotent in Bevy 0.18).
/// 3. [`UiPlugin`], the UI registration seam.
///
/// This is the seam the GTW-134 real-asset harness builds on to drive the `Load`
/// scene with a real `AssetServer` (`bevy-traps.md` rule 1: the kick-off needs
/// an `AssetServer`, which `MinimalPlugins` lacks).
pub fn register_scenes_with_default_plugins(app: &mut App) {
    app.init_state::<AppState>();
    app.add_plugins(ScenesPlugin);
    app.add_plugins(UiPlugin);
}
