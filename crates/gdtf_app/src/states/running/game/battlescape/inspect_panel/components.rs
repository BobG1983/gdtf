//! Marker components for the battlescape inspect panel (GTW-274).
//!
//! The inspect panel is the twin of the status panel, anchored TOP-RIGHT: it inspects
//! whatever the cursor hovers. It holds BOTH the shared ganger
//! [`stat_block`](super::super::stat_block) (shown when a ganger is hovered) AND a small
//! object stat block (hardness + integrity, shown when a non-floor object is hovered);
//! exactly one — or neither — is visible at a time, driven by the EFFECTIVE
//! [`InspectTarget`](gdtf_battle_input::InspectTarget). The markers are **unit structs** —
//! presence alone is the signal (the status-panel marker precedent).

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** node of the inspect-panel tree, so the `OnExit(BattleRunning)`
    /// despawn recursively tears down the whole panel by this one marker, and the per-update
    /// mutate toggles the whole panel's visibility by it.
    ///
    /// The root is an ABSOLUTE, fixed-% (`Val::Vw`/`Val::Vh`) UI overlay anchored TOP-RIGHT,
    /// hovering OVER the map on the UI camera — it is removed from layout flow and contributes
    /// NOTHING to the world-map viewport inset. Only the bottom bar reduces the map; this panel
    /// does NOT inset it (the overhaul reverses the GTW-271 right-margin inset). Widened to
    /// `pub` under `test-support` so the inspect integration test can assert the panel's
    /// overlay + fixed-% layout and its whole-panel Hidden state on bare floor, and
    /// `pub(crate)` otherwise (`unreachable_pub`-clean in the binary).
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectPanelRoot;
}

crate::support_item! {
    /// Marks the inspect panel's shared ganger stat-block host, so the update finds THIS
    /// panel's [`StatBlockRefs`](super::super::stat_block::StatBlockRefs) and toggles the
    /// ganger sub-block's visibility. Widened to `pub` under `test-support` so the inspect
    /// integration test can find the rendered ganger stat block.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectStatBlockHost;
}

crate::support_item! {
    /// Marks the inspect panel's **object** stat block container (hardness + integrity),
    /// shown when a non-floor object is hovered and hidden otherwise. Widened to `pub`
    /// under `test-support` so the inspect integration test can assert the object block
    /// appears on an object hover.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectBlock;
}

crate::support_item! {
    /// Marks the object block's **title** `Text` (e.g. "Cover") — the object kind heading
    /// (GTW-295 AC3, the analogue of the ganger block's name title). Widened to `pub` under
    /// `test-support` for the inspect integration test.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectText;
}

crate::support_item! {
    /// Marks the object block's **integrity** `ProgressBar` track (current/max structural
    /// HP). Widened to `pub` under `test-support` for the inspect integration test.
    ///
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectBar;
}

/// Marks the object block's **Integrity** label `Text` — the static "Integrity" caption
/// beside the integrity bar (GTW-295 AC3 — the bar reads bare without it).
///
/// Crate-internal only (the static label is set once at spawn and never asserted by a test,
/// so it is NOT widened to `pub` via `support_item!`). A unit marker: presence on an entity is
/// the whole signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape::inspect_panel) struct InspectObjectIntegrity;

crate::support_item! {
    /// Marks the object block's **Hardness** `Text` line (e.g. "Hardness 3") — the cover's
    /// `armor_hardness` from its [`CoverEntry`](gdtf_battle_sim::CoverEntry), mutated in place
    /// (GTW-295 AC3). Widened to `pub` under `test-support` for the inspect integration test. A
    /// unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectHardness;
}

crate::support_item! {
    /// Marks the object block's **Protection** `Text` line (e.g. "Protection 2") — the
    /// cover's `armor_protection` from its [`CoverEntry`](gdtf_battle_sim::CoverEntry),
    /// mutated in place (GTW-295 AC3). Widened to `pub` under `test-support` for the inspect
    /// integration test. A unit marker: presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectProtection;
}

crate::support_item! {
    /// Marks the object block's **Height band** `Text` line (e.g. "Height: High") — the
    /// cover's `height_band` from its [`CoverEntry`](gdtf_battle_sim::CoverEntry), mutated in
    /// place (GTW-295 AC3). Widened to `pub` under `test-support` for the inspect integration
    /// test. A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct InspectObjectHeight;
}
