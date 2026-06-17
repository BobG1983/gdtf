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
//! The level markers ([`LevelUpButton`] / [`LevelDownButton`]) and the
//! [`AimToggleButton`] each route a press to the matching
//! [`ActIntent`](gdtf_battle_input::ActIntent) on the shared 222a seam. The three
//! STANCE toggle markers ([`StanceStandingButton`] / [`StanceKneelingButton`] /
//! [`StanceProneButton`], GTW-267) and the three MODE toggle markers
//! ([`ModeSingleButton`] / [`ModeBurstButton`] / [`ModeFullButton`], GTW-265) are the
//! mutually-exclusive 3-toggle sub-panels that REPLACED the blind
//! `StanceCycleButton` cycle and the GTW-254 fire-mode popup picker respectively: each
//! pressed toggle DIRECT-sets its option (a stance press pushes a direct
//! [`ActIntent::SetStance`](gdtf_battle_input::ActIntent::SetStance); a mode press sets
//! [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) to the read-back spec), and
//! the currently-held option is shown via the `gdtf_ui`
//! [`ActiveButton`](gdtf_ui::ActiveButton) paint marker (the GTW-253 hook, made sticky
//! by GTW-266). There is deliberately NO `FireButton`: a button press carries no
//! `HoveredCell` target, so FIRE stays the left-click-on-target surface (landed in
//! 222b); explicit-target fire is GTW-11. The DEFERRED acts (reload, end-turn) have
//! NO sim act yet, so they are rendered as `DisabledButton` ([`ReloadButton`] /
//! [`EndTurnButton`]) that emit NO intent under any interaction — the `Without<
//! DisabledButton>` action filter excludes them (the menu `HiveScape` precedent).

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **Stand** stance-toggle button (GTW-267) — a press pushes a direct
    /// [`ActIntent::SetStance`](gdtf_battle_input::ActIntent::SetStance)`(`[`StanceKind::Standing`](gdtf_battle_sim::StanceKind::Standing)`)`,
    /// setting the [`SelectedShooter`](gdtf_battle_input::SelectedShooter)'s posture
    /// directly to standing (NOT a blind cycle). One of the three mutually-exclusive
    /// stance toggles; `sync_stance_buttons_active` marks the one matching the selected
    /// ganger's current [`Stance`](gdtf_battle_sim::Stance) with
    /// [`ActiveButton`](gdtf_ui::ActiveButton).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceStandingButton;
}

crate::support_item! {
    /// Marks the **Kneel** stance-toggle button (GTW-267) — a press pushes a direct
    /// [`ActIntent::SetStance`](gdtf_battle_input::ActIntent::SetStance)`(`[`StanceKind::Crouching`](gdtf_battle_sim::StanceKind::Crouching)`)`,
    /// setting the [`SelectedShooter`](gdtf_battle_input::SelectedShooter)'s posture
    /// directly to kneeling (the doc's "kneel" = [`StanceKind::Crouching`](gdtf_battle_sim::StanceKind::Crouching)).
    /// One of the three mutually-exclusive stance toggles.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceKneelingButton;
}

crate::support_item! {
    /// Marks the **Prone** stance-toggle button (GTW-267) — a press pushes a direct
    /// [`ActIntent::SetStance`](gdtf_battle_input::ActIntent::SetStance)`(`[`StanceKind::Prone`](gdtf_battle_sim::StanceKind::Prone)`)`,
    /// setting the [`SelectedShooter`](gdtf_battle_input::SelectedShooter)'s posture
    /// directly to prone. One of the three mutually-exclusive stance toggles.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceProneButton;
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
    /// Marks the **Single** fire-mode toggle button (GTW-265) — a press sets
    /// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) directly to the selected
    /// weapon's [`ModeKind::Single`](gdtf_battle_sim::ModeKind::Single) spec (read back off
    /// the weapon's [`FireMode`](gdtf_battle_sim::FireMode) selector, never fabricated).
    /// One of the (up to three) mutually-exclusive mode toggles in the Mode sub-panel,
    /// REPLACING the GTW-254 popup picker; spawned ONLY when the selected weapon offers
    /// this mode. `sync_mode_buttons_active` marks the live mode with
    /// [`ActiveButton`](gdtf_ui::ActiveButton).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeSingleButton;
}

crate::support_item! {
    /// Marks the **Burst** fire-mode toggle button (GTW-265) — a press sets
    /// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) directly to the selected
    /// weapon's [`ModeKind::Burst`](gdtf_battle_sim::ModeKind::Burst) spec (read back off
    /// the weapon's selector). One of the mutually-exclusive mode toggles, spawned ONLY
    /// when the selected weapon offers Burst.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeBurstButton;
}

crate::support_item! {
    /// Marks the **Full-auto** fire-mode toggle button (GTW-265) — a press sets
    /// [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode) directly to the selected
    /// weapon's [`ModeKind::Full`](gdtf_battle_sim::ModeKind::Full) spec (read back off
    /// the weapon's selector). One of the mutually-exclusive mode toggles, spawned ONLY
    /// when the selected weapon offers Full.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModeFullButton;
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
/// Widened to `pub(in …battlescape)` (GTW-271) so the sibling battlescape-level
/// `set_world_viewport` system can MEASURE the bar root's [`ComputedNode`](bevy::ui::ComputedNode)
/// height to inset the world-map viewport's BOTTOM margin. It is NOT widened through
/// `support_item!`/the test-support chain — it stays inside the battlescape neighborhood and
/// `unreachable_pub`-clean. A unit marker: presence on an entity is the whole signal
/// (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::scenes::running::game::battlescape) struct ActionBarRoot;

crate::support_item! {
    /// Marks the **root** of the vertical Stance sub-panel (GTW-267) — the
    /// [`spawn_panel`](gdtf_ui::spawn_panel) column holding the three stance toggles
    /// ([`StanceStandingButton`] / [`StanceKneelingButton`] / [`StanceProneButton`]). A
    /// stable always-visible sub-panel inside the action bar; spawned with the bar in
    /// `spawn_action_bar` and torn down with it via the [`ActionBarRoot`] recursive
    /// despawn (it is parented under the bar root).
    ///
    /// Widened through `support_item!` so the integration tests can find the panel.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StancePanelRoot;
}

crate::support_item! {
    /// Marks the **root** of the vertical Mode sub-panel (GTW-265) — the
    /// [`spawn_panel`](gdtf_ui::spawn_panel) column whose CHILDREN are the per-mode
    /// toggle buttons ([`ModeSingleButton`] / [`ModeBurstButton`] / [`ModeFullButton`]).
    /// The buttons are (re)built to exactly the modes the SELECTED weapon offers whenever
    /// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) changes
    /// (`rebuild_mode_buttons`), so a Single+Burst weapon shows exactly two toggles (no
    /// Full). The panel itself is stable (spawned with the bar, torn down with it); only
    /// its toggle children are rebuilt.
    ///
    /// Widened through `support_item!` so the integration tests can find the panel.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ModePanelRoot;
}
