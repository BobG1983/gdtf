//! Marker components for the battlescape status HUD panel (GTW-252).
//!
//! The status panel is a battle-scoped `gdtf_ui` panel showing the selected player
//! ganger's vitals. Its tree is a [`spawn_panel`](gdtf_ui::spawn_panel) box (the
//! [`StatusPanelRoot`] marker) holding one themed [`Text`](bevy::prelude::Text)
//! child per vitals line — each carrying its own ZST line-marker so the update
//! system can target that line's `Text` precisely (one marker per line → disjoint
//! `Query<&mut Text, With<…>>` writes, the action-bar per-act-marker precedent).
//!
//! All markers are **unit structs** — presence alone is the signal. They are not
//! domain values, so the no-bare-types rule does not apply (a marker's identity is
//! a named type, never a bare label string compared at runtime). They mirror the
//! sibling [`action_bar`](super::super::action_bar) module's marker shape.
//!
//! ## Visibility (the GTW-145 test-only-surface convention)
//!
//! The root marker is internal-only (the spawn/despawn systems are its only
//! readers), so — like [`ActionBarRoot`](super::super::action_bar) — it stays
//! `pub(in …status_panel)` and is NOT widened. The per-line markers are widened to
//! `pub` under the `test-support` feature via [`crate::support_item!`], so the
//! external integration tests can name them through [`crate::test_support`] to
//! assert each line's `Text` content (AC2 / AC3); they stay `pub(crate)` otherwise
//! so the production `grimdark_turfwar` binary build stays `unreachable_pub`-clean
//! (the action-bar per-act-marker re-export precedent).

use bevy::prelude::*;

/// Marks the **root** node of the status-panel tree (the
/// [`spawn_panel`](gdtf_ui::spawn_panel) box holding the vitals-line `Text`
/// children), so the `OnExit(BattleRunning)` despawn finds and recursively tears
/// down the whole panel by this one marker rather than tracking each child.
///
/// Internal-only (the spawn/despawn systems are the only readers), so — like
/// [`ActionBarRoot`](super::super::action_bar) — it is NOT widened through
/// `support_item!`/the test-support chain; it stays crate-internal and
/// `unreachable_pub`-clean. A unit marker: presence on an entity is the whole
/// signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::scenes::running::game::battlescape::status_panel) struct StatusPanelRoot;

crate::support_item! {
    /// Marks the **identity** vitals line — the selected ganger's cell `(x, y, level)`
    /// (from [`Position`](gdtf_battle_sim::Position)) and [`Faction`](gdtf_battle_sim::Faction).
    ///
    /// There is no ganger-name/id component today; the cell + faction stand in for an
    /// identity. A real `GangerName` newtype on the spawn bundle is FLAGGED future work
    /// (a follow-up ticket), deliberately not invented here.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct IdentityText;
}

crate::support_item! {
    /// Marks the **stance** vitals line — the selected ganger's
    /// [`Stance`](gdtf_battle_sim::Stance) posture (Standing / Crouching / Prone).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StanceText;
}

crate::support_item! {
    /// Marks the **time-units** vitals line — the selected ganger's
    /// [`Tu`](gdtf_battle_sim::Tu) current pool over its [`TuMax`](gdtf_battle_sim::TuMax)
    /// ceiling, shown as `cur/max` (e.g. "TU 7/10").
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct TuText;
}

crate::support_item! {
    /// Marks the **hit-points** vitals line — the selected ganger's current
    /// [`Hp`](gdtf_battle_sim::Hp) plus its [`Wounds`](gdtf_battle_sim::Wounds) count.
    ///
    /// There is no `HpMax` component today, so the line shows current HP only; a max
    /// is NOT fabricated. An `HpMax` source is FLAGGED future work.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HpText;
}

crate::support_item! {
    /// Marks the **life-state** vitals line — the selected ganger's
    /// [`LifeState`](gdtf_battle_sim::LifeState) (Alive / Downed / Dead).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct LifeText;
}
