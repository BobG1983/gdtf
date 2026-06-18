//! Marker components for the battlescape hover-inspect panel (GTW-274).
//!
//! The hover panel is the twin of the status panel, anchored TOP-RIGHT: it inspects
//! whatever the cursor hovers. It holds BOTH the shared ganger
//! [`stat_block`](super::super::stat_block) (shown when a ganger is hovered) AND a small
//! object stat block (hardness + integrity, shown when a non-floor object is hovered);
//! exactly one — or neither — is visible at a time, driven by the
//! [`HoveredCell`](gdtf_battle_input::HoveredCell). The markers are **unit structs** —
//! presence alone is the signal (the status-panel marker precedent).

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** node of the hover-panel tree, so the `OnExit(BattleRunning)`
    /// despawn recursively tears down the whole panel by this one marker, and the per-update
    /// mutate toggles the whole panel's visibility by it.
    ///
    /// The sibling battlescape-level `set_world_viewport` system MEASURES this root's
    /// [`ComputedNode`](bevy::ui::ComputedNode) width to inset the world-map viewport's RIGHT
    /// margin (the GTW-271 right-margin twin of the status panel's left-margin inset).
    /// Widened to `pub` under `test-support` so the hover integration test can assert the
    /// panel's whole-panel Hidden state on bare floor, and `pub(crate)` otherwise (reachable
    /// from the battlescape viewport system, `unreachable_pub`-clean in the binary).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HoverPanelRoot;
}

crate::support_item! {
    /// Marks the hover panel's shared ganger stat-block host, so the update finds THIS
    /// panel's [`StatBlockRefs`](super::super::stat_block::StatBlockRefs) and toggles the
    /// ganger sub-block's visibility. Widened to `pub` under `test-support` so the hover
    /// integration test can find the rendered ganger stat block.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HoverStatBlockHost;
}

crate::support_item! {
    /// Marks the hover panel's **object** stat block container (hardness + integrity),
    /// shown when a non-floor object is hovered and hidden otherwise. Widened to `pub`
    /// under `test-support` so the hover integration test can assert the object block
    /// appears on an object hover.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HoverObjectBlock;
}

crate::support_item! {
    /// Marks the object block's **name/hardness** `Text` (e.g. "Cover · Hardness 3").
    /// Widened to `pub` under `test-support` for the hover integration test.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HoverObjectText;
}

crate::support_item! {
    /// Marks the object block's **integrity** `ProgressBar` track (current/max structural
    /// HP). Widened to `pub` under `test-support` for the hover integration test.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct HoverObjectBar;
}
