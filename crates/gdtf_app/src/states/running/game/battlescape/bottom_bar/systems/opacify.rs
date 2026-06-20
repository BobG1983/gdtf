//! [`opacify_bottom_bar`] + [`repad_bottom_bar`] — keep the bottom-bar panel reading correctly
//! AFTER the theme pass (GTW-298, screenshot review 2026-06-18).
//!
//! The bottom bar is a `Themed(Panel)`, so `gdtf_ui::apply_theme` repaints it every run and
//! CLOBBERS two fields the bar needs:
//!
//! - **fill alpha** — the theme's panel fill is `alpha 0.55` (a SEMI-TRANSPARENT panel over the
//!   backdrop, the right look for the small corner HUD panels that float over the near-black
//!   menu/screen). But the bottom strip is the ONE solid bottom panel the weapon + stance cluster
//!   sit IN, hovering over the DOTTED world map: at 0.55 the map dots bleed through, and the strip
//!   stops reading as a single solid bordered panel (R3). [`opacify_bottom_bar`] forces ONLY the
//!   bottom-bar root's fill to FULL opacity (same RGB, alpha → 1.0).
//! - **content padding** — `apply_theme`'s `box_node` overwrites `Node::padding` from the panel
//!   sub-theme margin (a px value, no breathing room) every pass. [`repad_bottom_bar`] restores the
//!   four-sided relative-unit inset ([`bottom_bar_padding`]) so the weapon cluster + stance column
//!   are not flush against ANY edge (especially the bottom row), matching the mockup.
//!
//! Both pass through WITHOUT touching the theme (which would change every panel app-wide) and
//! WITHOUT weakening the themed border / radius (the theme still owns those, re-applied every run).
//! They run `.after(UiSystems::ApplyTheme)` (`bevy-traps.md` #3) under the bottom-bar plugin's
//! live-battle gate, and write only on a real change (change-detection hygiene), so once the
//! freshly-spawned bar settles they are no-ops; a theme repaint that re-applies the 0.55 fill / the
//! px padding is re-corrected the next frame. UI/view only; no [`Commands`], no `&mut World`
//! (`bevy-traps.md` #7).

use bevy::{color::Alpha, prelude::*, ui::BackgroundColor};

use crate::states::running::game::battlescape::bottom_bar::components::{
    BottomBarRoot, bottom_bar_padding,
};

/// Forces the bottom-bar panel's [`BackgroundColor`](bevy::ui::BackgroundColor) to FULL opacity
/// (GTW-298) so the bottom strip reads as ONE solid bordered panel over the dotted map, instead of
/// letting the theme's `alpha 0.55` panel fill bleed the map dots through (R3).
///
/// For each [`BottomBarRoot`] it raises its background color's alpha to `1.0` (keeping the
/// theme-painted RGB), writing only when the alpha is not already `1.0` (change-detection
/// hygiene). It does NOT touch the theme or any other panel — only the bottom-bar root's own fill.
///
/// Param-only (`bevy-traps.md` #7): one `&mut BackgroundColor` write query over the bottom-bar
/// root — no [`Commands`], no `&mut World`.
pub(in crate::states::running::game::battlescape) fn opacify_bottom_bar(
    mut bars: Query<&mut BackgroundColor, With<BottomBarRoot>>,
) {
    for mut fill in &mut bars {
        // Alpha is in `[0.0, 1.0]`, so "not already fully opaque" is `< 1.0` — a directional
        // compare (no `float_cmp` strict-equality lint) that keeps the change-detection guard:
        // once the fill is opaque this is a no-op, so a settled bar never re-writes.
        if fill.0.alpha() < 1.0 {
            fill.0.set_alpha(1.0);
        }
    }
}

/// Restores the bottom-bar panel's four-sided content [`padding`](bevy::ui::Node::padding) after
/// the theme pass (GTW-298) so the weapon cluster + stance column inset off EVERY edge — left,
/// right, top, bottom — and the bottom row (Prone / firemode buttons / Aim) is not jammed against
/// the window's bottom edge, matching the mockup.
///
/// `apply_theme`'s `box_node` overwrites a `Themed(Panel)`'s whole `Node::padding` from the panel
/// sub-theme margin (a px value with no breathing room) on every repaint, so the inset set at spawn
/// would be clobbered. For each [`BottomBarRoot`] this re-applies the shared relative-unit inset
/// ([`bottom_bar_padding`] — `Vw` left/right, `Vh` top/bottom), writing only when the current
/// padding differs (change-detection hygiene), so a settled bar never re-writes.
///
/// Param-only (`bevy-traps.md` #7): one `&mut Node` write query over the bottom-bar root — no
/// [`Commands`], no `&mut World`.
pub(in crate::states::running::game::battlescape) fn repad_bottom_bar(
    mut bars: Query<&mut Node, With<BottomBarRoot>>,
) {
    let want = bottom_bar_padding();
    for mut node in &mut bars {
        if node.padding != want {
            node.padding = want;
        }
    }
}
