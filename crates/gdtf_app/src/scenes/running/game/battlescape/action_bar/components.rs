//! Marker components for the battle action-bar (GTW-228 / GTW-48 S9 / 222c).
//!
//! Each action-bar button carries a unit-struct marker naming its act, so the
//! GTW-122-precedent action systems can find a specific button by its meaning and
//! keep their per-marker queries DISJOINT (one marker per act → no query conflict).
//! The markers are unit structs — presence alone is the signal (no-bare-types rule:
//! a button's identity is a named type, never a bare label string compared at
//! runtime). They mirror the menu's [`BattlescapeButton`](super::super) marker shape.
//!
//! ## Visibility (the GTW-145 test-only-surface convention)
//!
//! Each marker is declared through [`crate::support_item!`], which widens it to
//! `pub` under the `test-support` feature — so the external integration tests can
//! name it through [`crate::test_support`] — and keeps it `pub(crate)` otherwise, so
//! it stays internal in the production `grimdark_turfwar` binary (which compiles
//! `gdtf_app` WITHOUT `test-support`) and satisfies `unreachable_pub`. This mirrors
//! the menu button markers (`menu/components.rs`).
//!
//! ## Existing-act buttons vs DEFERRED buttons
//!
//! The five EXISTING-act markers ([`StanceCycleButton`] / [`AimToggleButton`] /
//! [`FireModeSelectButton`] / [`LevelUpButton`] / [`LevelDownButton`]) each route a
//! press to the matching [`ActIntent`](gdtf_battle_input::ActIntent) on the shared
//! 222a seam. There is deliberately NO `FireButton`: a button press carries no
//! `HoveredCell` target, so FIRE stays the left-click-on-target surface (landed in
//! 222b); explicit-target fire is GTW-11. The DEFERRED acts (reload, end-turn) have
//! NO sim act yet, so they are rendered as `DisabledButton` ([`ReloadButton`] /
//! [`EndTurnButton`]) that emit NO intent under any interaction — the `Without<
//! DisabledButton>` action filter excludes them (the menu `HiveScape` precedent).

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **stance-cycle** action button — a press pushes
    /// [`ActIntent::StanceCycle`](gdtf_battle_input::ActIntent::StanceCycle), the same
    /// intent the stance-cycle KEY pushes (parallel surfaces over one seam). The drain
    /// emits a [`SetStanceRequested`](gdtf_battle_sim::acts::SetStanceRequested) for the
    /// `SelectedShooter`.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceCycleButton;
}

crate::support_item! {
    /// Marks the **aim-toggle** action button — a press pushes
    /// [`ActIntent::AimToggle`](gdtf_battle_input::ActIntent::AimToggle), the same intent
    /// the aim-toggle KEY pushes. The drain emits a
    /// [`SetAimingRequested`](gdtf_battle_sim::acts::SetAimingRequested) toggling the
    /// `SelectedShooter`'s aim.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AimToggleButton;
}

crate::support_item! {
    /// Marks the **fire-mode-select** action button — a press pushes
    /// [`ActIntent::FireModeCycle`](gdtf_battle_input::ActIntent::FireModeCycle), the same
    /// intent the fire-mode-cycle KEY pushes. The drain advances
    /// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) among the selected
    /// weapon's offered modes (no sim message; the chosen mode rides the next fire). This
    /// is a fire-mode SELECT (no target needed), distinct from a FIRE button (which is
    /// deliberately absent — FIRE is the left-click-on-target surface).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct FireModeSelectButton;
}

crate::support_item! {
    /// Marks the **level-up** action button — a press pushes
    /// [`ActIntent::LevelUp`](gdtf_battle_input::ActIntent::LevelUp), the same intent the
    /// level-up KEY pushes. The drain raises the presenter's
    /// [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) one storey (clamped to the top).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LevelUpButton;
}

crate::support_item! {
    /// Marks the **level-down** action button — a press pushes
    /// [`ActIntent::LevelDown`](gdtf_battle_input::ActIntent::LevelDown), the same intent
    /// the level-down KEY pushes. The drain lowers the presenter's
    /// [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) one storey (floored at 0).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LevelDownButton;
}

crate::support_item! {
    /// Marks the **reload** action button — a DEFERRED act (GTW-228): no `reload()` act
    /// nor `reload_tu` leaf exists yet (`gdtf_battle_sim::magazine` ships only ammo state +
    /// `spend_round`/`clamp_burst`; a reload act is OUT of E4, owned by the magazine/reload
    /// epic). Rendered as a [`DisabledButton`](gdtf_ui::DisabledButton) that emits NO
    /// intent under any interaction (the `Without<DisabledButton>` action filter excludes
    /// it — the menu `HiveScape` precedent). Re-enabled when the reload act lands.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ReloadButton;
}

crate::support_item! {
    /// Marks the **end-turn** action button — a DEFERRED act (GTW-228): no end-turn /
    /// turn-advance / next-round act exists yet (`gdtf_battle_sim::battle` ships only the
    /// E10 setup/teardown lifecycle, not a turn loop; `tu::reset_tu` is a round-start refill
    /// helper, not a player act — the turn loop is owned by E9/GTW-14). Rendered as a
    /// [`DisabledButton`](gdtf_ui::DisabledButton) that emits NO intent under any
    /// interaction (the `Without<DisabledButton>` action filter excludes it). Re-enabled
    /// when the turn loop lands.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EndTurnButton;
}

crate::support_item! {
    /// Marks the **flee-battle** action button — an ENABLED APP/LIFECYCLE button, NOT a
    /// sim act. A press runs the dedicated `flee_button_pressed` handler, which inserts the
    /// `BattleRunningComplete` end-signal marker (via the
    /// [`insert_battle_running_complete`](crate::scenes::running::game::battlescape::battle_running::insert_battle_running_complete)
    /// door), so the existing marker-gated `move_on` advances
    /// `BattleRunning → AnimateOut → AfterMath` — the player's explicit "I'm leaving" out
    /// (requirement 5(b)).
    ///
    /// Unlike the five sim-act buttons, flee is NOT routed through the
    /// [`PendingActIntent`](gdtf_battle_input::PendingActIntent) /
    /// [`dispatch_act_intents`](gdtf_battle_input::dispatch_act_intents) sim seam: that seam
    /// carries only sim `*Requested` acts against the `SelectedShooter`, and flee is not a
    /// sim verb (no actor, no TU, no `*Requested`). Unlike the DEFERRED [`ReloadButton`] /
    /// [`EndTurnButton`], flee is ENABLED — it carries NO
    /// [`DisabledButton`](gdtf_ui::DisabledButton), so it is interactive and its handler's
    /// `Without<DisabledButton>` press filter INCLUDES it.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct FleeButton;
}

/// Marks the **root** node of the action-bar tree (the [`spawn_panel`](gdtf_ui::spawn_panel)
/// box holding the buttons), so the `OnExit(BattleRunning)` despawn finds and recursively
/// tears down the whole bar by this one marker rather than tracking each child entity.
///
/// Internal-only (the spawn/despawn systems are the only readers), so — unlike the per-act
/// markers — it is NOT widened through `support_item!`/the test-support chain; it stays
/// crate-internal and `unreachable_pub`-clean. A unit marker: presence on an entity is the
/// whole signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::scenes::running::game::battlescape::action_bar) struct ActionBarRoot;
