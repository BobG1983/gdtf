//! The loading-screen marker + stacking constant for the GTW-419 procgen loading state.

use bevy::prelude::*;

crate::support_item! {
    /// Marker for the full-viewport battlescape LOADING SCREEN overlay (GTW-419).
    ///
    /// One root entity spawned `OnEnter(BattleScapeState::Generation)` and despawned via
    /// [`DespawnOnExit`](bevy::state::prelude::DespawnOnExit)`(BattleScapeState::Generation)`
    /// — the state-scoped-entity idiom (`bevy-traps.md` #1, applied to entities). While the
    /// sim assembles the level + builds the battle (the brief Generation phase), this opaque
    /// overlay covers the whole window so NO frame of an unbuilt / partial level is ever shown
    /// to the player (AC2): the level / presenter render on the WORLD camera, the UI overlay
    /// renders on the higher UI camera, so an opaque full-viewport UI node paints over
    /// whatever the world camera has drawn (`bevy-traps.md` #8 — occlusion working FOR us).
    ///
    /// `support_item!` widens it to `pub` under the `test-support` feature so the headless
    /// `GdtfTestAppBuilder` AC tests can name it through
    /// [`test_support`](crate::test_support); it is `pub(crate)` (and
    /// `unreachable_pub`-clean) in the production binary.
    #[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
    struct LoadingScreenRoot;
}

/// The loading screen's stacking order ([`GlobalZIndex`](bevy::ui::GlobalZIndex) — the higher,
/// the nearer the viewer).
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`](bevy::ui::GlobalZIndex)
/// (the framework carve-out, not a domain value). Set HIGH — strictly above every battlescape
/// HUD band (bottom bar `10`, on-bar cluster / combat log `11`, contextual panel `20`) so that
/// if any HUD or partial-level UI ever co-exists with the loading screen it is painted OVER, not
/// under (`bevy-traps.md` #8). In practice the HUD panels spawn only in `BattleRunning`, so
/// during Generation nothing competes — but the high z makes the no-partial-frame occlusion
/// invariant STRUCTURAL and future-proof (and headless-assertable, AC2 / T2).
pub(in crate::states::running::game::battlescape::generation) const LOADING_SCREEN_Z: i32 = 1000;

/// The loading box's window-relative width (`Vw`).
///
/// A framework-plumbing `const` fed to a [`Val::Vw`](bevy::ui::Val) (the responsive-sizing
/// carve-out, not a domain value). Relative units only (`ui-responsive-not-px`).
pub(in crate::states::running::game::battlescape::generation) const LOADING_BOX_W_VW: f32 = 40.0;

/// The loading box's window-relative minimum height (`Vh`).
///
/// A framework-plumbing `const` fed to a [`Val::Vh`](bevy::ui::Val) (the responsive-sizing
/// carve-out). Relative units only.
pub(in crate::states::running::game::battlescape::generation) const LOADING_BOX_MIN_H_VH: f32 =
    12.0;
