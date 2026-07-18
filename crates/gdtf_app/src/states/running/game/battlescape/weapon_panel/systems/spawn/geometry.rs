//! The weapon cluster's responsive layout geometry — every sizing constant of the
//! GTW-298 authoritative layout in one place (one change-reason: the layout tune).
//! Split out of the monolithic `spawn.rs` (GTW-583); the layout rationale lives on
//! each constant.

use crate::states::running::game::battlescape::bottom_bar::{BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_Y_VH};

/// The WIDTH of the Overall Weapon Panel, as a fraction of the WINDOW WIDTH
/// ([`Val::Vw`](bevy::ui::Val) — GTW-295 responsive ruling).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node) (the `CELL_PX`-class carve-out, not a
/// domain value). The panel occupies the LEFT region of the bottom bar in the mockup; `vw` so
/// it scales with the window. A FIXED fraction (item 7): the renderable map area does not
/// pop/shift when the panel contents change.
pub(super) const PANEL_W_VW: f32 = 22.0;

/// The WIDTH of the separate Stance Panel, as a fraction of the WINDOW WIDTH
/// ([`Val::Vw`](bevy::ui::Val)).
///
/// A `const`, layout plumbing fed to a [`Node`](bevy::ui::Node). The Stance panel sits to the RIGHT of the
/// Overall Weapon Panel (a narrow stacked-toggle column in the mockup); a fixed `vw` fraction
/// so the map area is stable (item 7).
pub(super) const STANCE_W_VW: f32 = 8.0;

/// The LEFT edge of the separate Stance Panel, as a fraction of the WINDOW WIDTH — placed
/// immediately to the RIGHT of the Overall Weapon Panel (a small gap beyond [`PANEL_W_VW`]).
///
/// A `const`, layout plumbing fed to a [`Node`](bevy::ui::Node): `PANEL_W_VW` + a hairline gap so the Stance
/// panel does not overlap the Overall panel (item 8 — no panel overlaps another).
pub(super) const STANCE_LEFT_VW: f32 = PANEL_W_VW + 0.5;

/// The weapon cluster's stacking order ([`GlobalZIndex`](bevy::ui::GlobalZIndex)) — one ABOVE the bottom bar's, so the
/// panels (which sit IN the bar, item 6) draw on TOP of the bar's opaque fill.
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`](bevy::ui::GlobalZIndex) (the framework carve-out, not
/// a domain value). Kept in lockstep with the bar's z (`bottom_bar` = 10) — the panels must be
/// strictly greater.
pub(super) const PANEL_Z: i32 = 11;

/// The HEIGHT of the weapon cluster, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val)).
///
/// Sized to the bar's CONTENT box (the bar height [`BOTTOM_BAR_H_VH`] MINUS its top + bottom
/// padding [`BOTTOM_BAR_PAD_Y_VH`] each) so the cluster insets off BOTH vertical edges of the bar
/// (D5 — 2026-06-18 screenshot review). The cluster is an ABSOLUTE overlay anchored to the window,
/// not an in-flow child of the bar, so the bar's `Node::padding` does NOT inset it — the bottom /
/// top breathing room has to live in the cluster's OWN geometry (anchored `bottom:
/// `[`BOTTOM_BAR_PAD_Y_VH`] with this reduced height), or the bottom row (Firemode toggles / Aim)
/// sits flush against the window's bottom edge (P1/P2). It still cannot bleed out the top of the
/// bar (GTW-275 layout overhaul item 6) — it now stops a padding short of BOTH edges. Gives the
/// grid cells a concrete parent to take their `Percent` shares of (a `Column` with `height: auto`
/// would not resolve child `Percent` heights). Responsive (`vh`), NOT a fixed px height.
pub(super) const PANEL_H_VH: f32 = BOTTOM_BAR_H_VH - 2.0 * BOTTOM_BAR_PAD_Y_VH;

/// The COMBINED / ITEM cell height as a `Percent` of its column — the top band (the authoritative
/// height split). A `const` layout plumbing value, NOT a fixed px.
///
/// GTW-303 clip fix (2026-06-19): shrunk from 75 → 65 % (and [`BOTTOM_CELL_PCT`] grown to match)
/// so the bottom firemode / aim row gets a larger share of the column. The firemode segments are
/// now TWO lines (the mode name over its `"{n} TU"` cost sub-line — GTW-303); the larger bottom
/// cell gives the second line the vertical room it needs so it is not clipped. The Combined / Item
/// cells stay comfortably tall for the image + name/mag/Reload row at 65 %.
pub(super) const TOP_CELL_PCT: f32 = 65.0;

/// The FIREMODE / AIM cell height as a `Percent` of its column — the bottom band (the
/// authoritative height split). A `const` layout plumbing value, NOT a fixed px.
///
/// GTW-303 clip fix (2026-06-19): grown from 25 → 35 % (mirroring [`TOP_CELL_PCT`]'s shrink) so
/// the firemode control's now-TWO-line segments (mode name over the `"{n} TU"` cost sub-line —
/// GTW-303) have the vertical room to render BOTH lines fully, instead of the cost line clipping
/// at the cell's bottom edge.
pub(super) const BOTTOM_CELL_PCT: f32 = 35.0;

/// The LEFT column width as a `Percent` of the Overall panel — the 3/4 share (Combined +
/// Firemode). A `const` layout plumbing value, NOT a fixed px.
pub(super) const LEFT_COL_PCT: f32 = 75.0;

/// The RIGHT column width as a `Percent` of the Overall panel — the 1/4 share (Item + Aim). A
/// `const` layout plumbing value, NOT a fixed px.
pub(super) const RIGHT_COL_PCT: f32 = 25.0;

/// The weapon-text block / Reload row's combined height as a `Percent` of the Combined panel —
/// the bottom 1/2 (the [`WeaponImage`](crate::states::running::game::battlescape::weapon_panel::components::WeaponImage) takes the top 1/2). A `const` layout plumbing value, NOT a fixed px.
pub(super) const INFO_ROW_H_PCT: f32 = 50.0;

/// The weapon-text block's height as a `Percent` of the info block — the 65% share above the
/// 35%-height Reload row (GTW-733: the Reload button moved OUT of the name's row into its OWN
/// row below, so the two never share pixels regardless of name length — the same 65/35 split
/// [`TOP_CELL_PCT`]/[`BOTTOM_CELL_PCT`] already uses for the grid bands). A `const` layout
/// plumbing value, NOT a fixed px.
pub(super) const INFO_TEXT_H_PCT: f32 = 65.0;

/// The Reload row's height as a `Percent` of the info block — the 35% share below the
/// 65%-height weapon-text block (GTW-733). A `const` layout plumbing value, NOT a fixed px.
pub(super) const INFO_RELOAD_H_PCT: f32 = 35.0;

/// The minimum height of the weapon-text block, as a fraction of the WINDOW HEIGHT
/// ([`Val::Vh`](bevy::ui::Val)).
///
/// A `const`, layout plumbing fed to a [`Node`](bevy::ui::Node). The GTW-275 overflow floor: it floors the
/// weapon-text height so the absolutely-positioned, auto-sized root cannot mismeasure against
/// near-zero-height text and push the rows below the panel border. GTW-733 doubled it (3.0 ->
/// 6.0): the name line now wraps to a SECOND line rather than clipping when even the FULL-WIDTH
/// block (GTW-733 gave the name the whole panel width, not just 3/4 of it) can't fit a long
/// shipped identifier on one line, so the floor must hold two text lines' worth of height, not
/// one. Responsive (`vh`), NOT a fixed px.
pub(super) const CONTENT_MIN_H_VH: f32 = 6.0;

/// The VERTICAL gap between weapon-cluster sub-nodes (row gaps), as a fraction of the window
/// HEIGHT ([`Val::Vh`](bevy::ui::Val) — GTW-296 responsive ruling). A `const`, layout plumbing
/// fed to a [`Node`](bevy::ui::Node). Calibrated to a 4px gap at the 1280x720 reference window (4 / 720 =
/// 0.55556) so the cluster is visually identical at the default size.
pub(super) const GAP_VH: f32 = 0.55556;

/// The HORIZONTAL gap between weapon-cluster sub-nodes (column gaps), as a fraction of the window
/// WIDTH ([`Val::Vw`](bevy::ui::Val) — GTW-296 responsive ruling). A `const`, layout plumbing fed
/// to a [`Node`](bevy::ui::Node). Calibrated to a 4px gap at the 1280x720 reference window (4 / 1280 = 0.3125) so
/// the cluster is visually identical at the default size.
pub(super) const GAP_VW: f32 = 0.3125;
