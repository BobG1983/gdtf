//! The combat-log tree markers: the anchored log ROOT container and the per-line unit
//! markers. Split out of the monolithic `components.rs` (GTW-583); the log rationale
//! lives on the parent `components` module.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** container of the combat-log tree — the bottom-left `flex column` the
    /// event lines append into, so the `OnExit(BattleRunning)` despawn finds and recursively
    /// tears down the whole log by this one marker, and the per-update append finds the parent
    /// to spawn each new line under.
    ///
    /// Widened toward `crate::test_support` via `support_item!` so the
    /// AC test can name it to assert the log gains line children. A unit marker: presence on an
    /// entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogRoot;
}

crate::support_item! {
    /// Marks one **combat-log line** entity — a UI [`Text`](bevy::prelude::Text) node holding a
    /// classified [`LogLine`](gdtf_battle_presenter::LogLine)'s text, parented under
    /// [`CombatLogRoot`]. The per-update overflow trim finds the visible lines by this marker
    /// (FIFO-despawning the OLDEST when the count exceeds the tuned max), and the AC test names
    /// it to assert the rendered lines + overflow.
    ///
    /// Widened toward `crate::test_support`. A unit marker: presence on an entity is the whole
    /// signal (no-bare-types rule). The line's lifetime / fade state rides the sibling
    /// `LogLineFade` component.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct CombatLogLine;
}
