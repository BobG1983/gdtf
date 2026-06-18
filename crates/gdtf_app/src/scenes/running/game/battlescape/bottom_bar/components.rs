//! Marker + the shared height const for the battlescape BOTTOM BAR (GTW-275 layout
//! overhaul, item 6).
//!
//! The bottom bar is the ONE opaque strip at the bottom of the battlescape that reduces the
//! world map (the map ends at the bar's top edge — the only viewport inset). The status /
//! hover panels are corner overlays that contribute NOTHING to the inset; the weapon panel +
//! the (out-of-scope) controls + contextual panels all sit INSIDE this strip. Its
//! [`BottomBarRoot`] marker is what `set_world_viewport` MEASURES for the BOTTOM inset, and
//! what the AC tests assert.

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** node of the battlescape bottom bar (the opaque full-width strip at
    /// the bottom of the screen — GTW-275 layout overhaul item 6).
    ///
    /// The bottom bar is the ONLY UI that reduces the world map: `set_world_viewport`
    /// MEASURES this root's [`ComputedNode`](bevy::ui::ComputedNode) HEIGHT and insets the
    /// world-camera viewport's BOTTOM by it (full width, no side / top inset — items 1 / 4).
    /// Widened toward `crate::test_support` via [`support_item!`](crate::support_item) so the
    /// AC tests can assert the bar's presence + measure, AND re-exported to the battlescape
    /// neighborhood so the sibling viewport system can name it. A unit marker: presence on an
    /// entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct BottomBarRoot;
}

/// The HEIGHT of the bottom bar, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val) — the responsive-units ruling: relative units only, no fixed
/// px but a hairline border).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node) (the `CELL_PX`-class
/// carve-out, not a domain value). The single source of truth for the bar height: the weapon
/// panel sizes itself to fill the bar's height (so it cannot bleed out the top — item 6), and
/// `set_world_viewport` insets the map by the bar's measured height (item 1). Sized so the
/// later mode / aim / stance widgets (GTW-277) + the contextual panel (GTW-294) drop in. The
/// bottom strip of the mockup is roughly a quarter of the window height.
pub(in crate::scenes::running::game::battlescape) const BOTTOM_BAR_H_VH: f32 = 26.0;

/// The bottom bar's INNER padding on the TOP and BOTTOM edges, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO fixed px,
/// so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `padding.top` /
/// `padding.bottom` (the `CELL_PX`-class carve-out, not a domain value). Insets the bar's CONTENT
/// (the weapon cluster + the stance column — especially the bottom row: Prone / the firemode
/// buttons / Aim) DOWNWARD off the panel's top edge and UPWARD off the window's bottom edge, so the
/// content is not flush against either, matching the mockup's clear empty space above AND below the
/// bottom-panel content (screenshot review 2026-06-18). Vertical edges use `Vh` so the inset tracks
/// the window's HEIGHT (the axis the bar's height lives on). It is RE-applied every theme pass by
/// `repad_bottom_bar` (`apply_theme`'s `box_node` overwrites the whole `padding` from the panel
/// sub-theme margin, which has no breathing room), so the inset survives a theme repaint. A small
/// fraction of the viewport — the bar's measured `ComputedNode` height grows by it, so
/// `set_world_viewport` insets the map a touch more (unchanged otherwise).
pub(in crate::scenes::running::game::battlescape) const BOTTOM_BAR_PAD_Y_VH: f32 = 1.5;

/// The bottom bar's INNER padding on the LEFT and RIGHT edges, as a fraction of the WINDOW WIDTH
/// ([`Val::Vw`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO fixed px,
/// so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `padding.left` /
/// `padding.right` (the `CELL_PX`-class carve-out, not a domain value). Insets the bar's CONTENT
/// (the weapon cluster on the left, the stance column toward the right) INWARD off the panel's left
/// and right edges, so neither is flush against the window's side, matching the mockup's clear
/// empty space beside the bottom-panel content (screenshot review 2026-06-18). Horizontal edges use
/// `Vw` so the inset tracks the window's WIDTH (the axis the bar's full `Vw(100)` width lives on),
/// keeping a uniform visual gutter on resize. RE-applied every theme pass by `repad_bottom_bar`
/// alongside [`BOTTOM_BAR_PAD_Y_VH`] (the theme's `box_node` clobbers the whole `padding`).
pub(in crate::scenes::running::game::battlescape) const BOTTOM_BAR_PAD_X_VW: f32 = 0.8;

/// The bottom bar's four-sided inner [`UiRect`] padding (left/right in `Vw`, top/bottom in `Vh`).
///
/// The single source of truth for the bar's content inset, shared by `spawn_bottom_bar` (so the
/// padding is right on the FIRST frame) and `repad_bottom_bar` (so it survives the theme pass that
/// clobbers `Node::padding`). Relative units ONLY — `Vw` on the horizontal axis, `Vh` on the
/// vertical — so the gutter scales with the window on resize (the responsive-units ruling), never
/// a fixed `Px`. Layout plumbing, not a domain value (the `CELL_PX`-class carve-out).
pub(in crate::scenes::running::game::battlescape) const fn bottom_bar_padding() -> UiRect {
    UiRect {
        left:   Val::Vw(BOTTOM_BAR_PAD_X_VW),
        right:  Val::Vw(BOTTOM_BAR_PAD_X_VW),
        top:    Val::Vh(BOTTOM_BAR_PAD_Y_VH),
        bottom: Val::Vh(BOTTOM_BAR_PAD_Y_VH),
    }
}
