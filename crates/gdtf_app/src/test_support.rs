//! Test-support surface for building and asserting on the real GDTF state
//! machine from an external crate.
//!
//! This module is gated behind the `test-support` feature and is absent from
//! release builds — the production public API of `gdtf_app` is exactly
//! [`crate::GdtfApp`]. It re-exports the otherwise crate-internal state enums
//! and [`ScenesPlugin`] so an out-of-crate harness can name them, and provides
//! [`register_headless`] to wire the full headless state stack in the correct
//! parent-before-child order.

use bevy::{
    app::App,
    input::InputPlugin,
    state::app::{AppExtStates, StatesPlugin},
};
pub use gdtf_ui::UiPlugin;

pub use crate::{
    app::auto_battle::{
        AutoBattleActive, AutoBattlePlugin, auto_battle_enabled, seed_load_fallbacks,
    },
    states::{
        AfterMathState, AimLabel, AimPanel, AimToggleButton, AppState, BattleRunningComplete,
        BattleScapeState, BattlescapeButton, BottomBarRoot, CombatLogLine, CombatLogRoot,
        CombinedWeaponPanel, ContextualPanelRoot, EndTurnButton, ExecuteButton, FleeButton,
        GameState, HiveScapeButton, InspectObjectBar, InspectObjectBlock, InspectObjectHardness,
        InspectObjectHeight, InspectObjectProtection, InspectObjectText, InspectPanelRoot,
        InspectStatBlockHost, LevelDownButton, LevelUpButton, LoadedSituation, MenuTitle,
        ModeBurstButton, ModeControl, ModeFullButton, ModePanelRoot, ModeSingleButton,
        OpenDoorButton, OptionsButton, QuitButton, ReloadButton, RunningState, ScenesPlugin,
        StabilizeButton, StanceControl, StanceKneelingButton, StancePanelRoot, StanceProneButton,
        StanceStandingButton, StatFaction, StatHpBar, StatHpLabel, StatName, StatPortrait,
        StatStance, StatTuBar, StatTuLabel, StatWoundLine, StatWoundList, StatWoundsPips,
        WeaponContent, WeaponImage, WeaponItemButton, WeaponItemPanel, WeaponMagazineText,
        WeaponNameText, WeaponPanelRoot, portrait_index_for_name,
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
