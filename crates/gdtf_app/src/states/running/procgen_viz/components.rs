//! Marker / data components for the DEV-ONLY procgen-visualizer screen (GTW-434).
//!
//! Each entity in the visualizer tree carries a named marker so the draw + control systems
//! (and the headless test) can find it by meaning rather than spawn order: the screen root,
//! the dark board quad, the per-prefab light quads (carrying their reveal index + tint), and
//! the STEP / AUTO control buttons. Unit markers are the no-bare-types "presence is the
//! signal" form; [`PrefabQuad`] additionally carries the data the test asserts on.
//!
//! Markers the headless test names (the screen root, the board quad, the prefab quads + their
//! tint, the STEP / AUTO buttons) are declared through `crate::support_item!` so they widen
//! to `pub` under `test-support` and stay `pub(crate)` (binary `unreachable_pub`-clean)
//! otherwise. The whole module is `#[cfg(debug_assertions)]`-gated by its parent.

use bevy::prelude::*;

use super::model::{QuadTint, RevealIndex, VizQuad};

crate::support_item! {
    /// Marks the visualizer screen ROOT node (the full-screen backdrop holding the board
    /// quad + the control bar) — the entity `OnExit` despawns and the headless test counts.
    ///
    /// A unit marker (no-bare-types: presence is the whole signal).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ProcgenVizRoot;
}

crate::support_item! {
    /// Marks the single DARK whole-level quad (the board extent) every per-prefab quad draws
    /// over (C2).
    ///
    /// A unit marker (no-bare-types: presence is the whole signal). The host node into which
    /// the per-prefab quads are parented — it carries the cell-grid coordinate frame the
    /// quads position against.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct BoardQuad;
}

crate::support_item! {
    /// Marks one per-prefab LIGHT tinted quad (C2/C3), carrying its reveal INDEX into the
    /// placement sequence and its [`QuadTint`] role.
    ///
    /// Spawned once per quad in the placement sequence; its [`Node`](bevy::ui::Node) display
    /// is toggled by the draw layer as the reveal count crosses its index. The headless test
    /// reads the [`tint`](PrefabQuad::tint) of the revealed quads to assert player = green /
    /// enemy = red / others = neutral (C3) on the REAL visualizer entities.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
    struct PrefabQuad {
        /// This quad's index into the placement sequence (`0` = player, `1` = enemy, then
        /// fill in placement order) — the draw layer reveals it once `revealed > index`.
        index: RevealIndex,
        /// This quad's tint role (player = green, enemy = red, fill = neutral, C3).
        tint:  QuadTint,
    }
}

impl PrefabQuad {
    /// Build a prefab-quad marker for the quad at `index` in the placement sequence.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(
        index: RevealIndex,
        tint: QuadTint,
    ) -> Self {
        Self { index, tint }
    }

    crate::support_item! {
        /// This quad's index into the placement sequence (test-facing — the C3 assertion
        /// matches the revealed quads by index through `test_support`, deref-ing the
        /// `RevealIndex` to a `usize`).
        #[must_use]
        const fn index(self) -> RevealIndex {
            self.index
        }
    }

    crate::support_item! {
        /// This quad's tint role (test-facing — the C3 assertion reads it through
        /// `test_support` to verify player = green / enemy = red / others = neutral).
        #[must_use]
        const fn tint(self) -> QuadTint {
            self.tint
        }
    }
}

crate::support_item! {
    /// Marks the STEP control button — a press reveals ONE more prefab quad (C1).
    ///
    /// A unit marker (no-bare-types: presence is the whole signal). The headless test finds
    /// it to drive the STEP path through the real `Changed<Interaction>` reader.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StepButton;
}

crate::support_item! {
    /// Marks the AUTO control button — a press reveals EVERY prefab quad at once (C1).
    ///
    /// A unit marker (no-bare-types: presence is the whole signal). The headless test finds
    /// it to drive the AUTO path through the real `Changed<Interaction>` reader.
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct AutoButton;
}

/// Spawn-time bundle of one per-prefab quad's marker — the [`PrefabQuad`] data built from a
/// quad's reveal [`RevealIndex`] + its [`VizQuad`]'s tint. Keeps the build system terse.
#[must_use]
pub(in crate::states::running::procgen_viz) const fn prefab_quad_marker(
    index: RevealIndex,
    quad: &VizQuad,
) -> PrefabQuad {
    PrefabQuad::new(index, quad.tint())
}
