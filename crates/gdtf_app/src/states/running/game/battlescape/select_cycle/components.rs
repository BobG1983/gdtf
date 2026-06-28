//! Marker components for the battlescape Prev/Next selection-cycle cluster (GTW-458).
//!
//! Each cluster node carries a unit-struct marker so its spawn/despawn + press-router
//! systems can find it and keep their per-marker queries DISJOINT (the action-bar marker
//! precedent). Presence alone is the signal (no-bare-types rule: a button's identity is a
//! named type, never a bare label string compared at runtime).
//!
//! Visibility follows the GTW-145 test-only-surface convention via
//! [`crate::support_item!`]: each marker widens to `pub` under the `test-support` feature so
//! the integration tests name it through [`crate::test_support`], and stays `pub(crate)`
//! otherwise so the production binary is `unreachable_pub`-clean (the action-bar / weapon-panel
//! marker precedent).

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** node of the Prev/Next selection-cycle cluster (GTW-458) — the
    /// vertical [`Column`](bevy::ui::FlexDirection::Column) at the far RIGHT of the bottom bar
    /// holding the Next-over-Prev buttons.
    ///
    /// Parented as a CHILD of the bottom bar (the Stance-Panel precedent), so the bar's own
    /// despawn tears it down. The AC tests assert its presence + its `<= 10%` width through
    /// [`crate::test_support`]. A unit marker: presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectCycleRoot;
}

crate::support_item! {
    /// Marks the **Next** selection-cycle button (GTW-458) — a press pushes
    /// [`ActIntent::SelectNext`](gdtf_battle_input::ActIntent::SelectNext) onto the shared
    /// act-intent seam (the SAME intent the `Tab` key pushes — ADR-0001, keys + buttons share
    /// one dispatch). Cycles the [`SelectedShooter`](gdtf_battle_input::SelectedShooter) to the
    /// next player ganger in `(z, y, x)` order, wrapping.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectNextButton;
}

crate::support_item! {
    /// Marks the **Prev** selection-cycle button (GTW-458) — a press pushes
    /// [`ActIntent::SelectPrev`](gdtf_battle_input::ActIntent::SelectPrev) onto the shared
    /// act-intent seam (the SAME intent `Shift+Tab` pushes — ADR-0001). Cycles the
    /// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) to the previous player ganger in
    /// `(z, y, x)` order, wrapping.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct SelectPrevButton;
}
