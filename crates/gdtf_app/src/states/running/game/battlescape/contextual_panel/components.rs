//! The root marker + layout consts for the battlescape CONTEXTUAL PANEL (GTW-294;
//! generic per-act machinery since GTW-571).
//!
//! The contextual panel is the bottom-RIGHT cluster of the HUD (per the
//! `docs/ui_mockups/battlescape_mockup.png` bottom-right corner — "Contextual Buttons go
//! Here", to the right of the Stance column). It hosts the situational acts a selected
//! ganger can take: one themed button per registered contextual act (Execute / Stabilize /
//! Melee / Shove / Open Door / Enter / Exit Emplacement / Throw Grenade — see
//! [`acts`](super::acts)). The panel + every button spawn
//! [`Visibility::Hidden`](bevy::camera::visibility::Visibility): the spawn systems only
//! build the tree; each act's OFFER scan fills its
//! [`ContextualOffer`](super::seam::ContextualOffer) and the generic toggles flip the
//! `Visibility` IN PLACE (never despawn — `ui-mutate-not-respawn`). The per-act button
//! markers live in each act's own module under [`acts`](super::acts) (GTW-571 — one
//! vertical act module per crate layer).

use bevy::prelude::*;

crate::support_item! {
    /// Marks the **root** node of the contextual panel box (the bottom-right cluster holding
    /// the per-act contextual buttons — GTW-294).
    ///
    /// It is a [`spawn_panel`](gdtf_ui::spawn_panel) themed box parented as a CHILD of the
    /// [`BottomBarRoot`](super::super::bottom_bar::BottomBarRoot) container (GTW-726), spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by
    /// the act-agnostic `sync_panel_root_visibility` pass (visible iff ANY act button is).
    /// As a child of the bar it is laid out INSIDE the bottom panel (never floating over the
    /// map) and renders in the bar's own stacking context, on top of the bar's fill, with NO
    /// [`GlobalZIndex`](bevy::ui::GlobalZIndex) of its own (the stance-panel precedent — see
    /// `CONTEXTUAL_PANEL_Z` for the surviving defensive-fallback use). The
    /// `OnExit(BattleRunning)` despawn tears the whole subtree down by THIS marker. Widened
    /// through `support_item!` so the AC tests can assert the box's presence + parent. A unit
    /// marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContextualPanelRoot;
}

/// The contextual panel's DEFENSIVE-fallback stacking order
/// ([`GlobalZIndex`](bevy::ui::GlobalZIndex) — the higher, the nearer the viewer), used ONLY when
/// the panel cannot be parented under the bottom bar.
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`](bevy::ui::GlobalZIndex) (the
/// framework carve-out, not a domain value). Since GTW-726 the panel is normally a CHILD of the
/// [`BottomBarRoot`](super::super::bottom_bar::BottomBarRoot), which renders it in the bar's own
/// stacking context on top of the bar's fill with NO z of its own (the stance-panel precedent). If
/// the bar root is somehow absent when the panel spawns (defensive — `spawn_contextual_panel` is
/// ordered `.after(spawn_bottom_bar)`, so it should not be), the panel stays a top-level root and
/// falls back to carrying this z so the opaque bar (`GlobalZIndex(10)`) does not paint over it.
/// This value is `20` — comfortably above the bar (`10`) and the on-bar weapon + stance cluster
/// (`11`), with headroom.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_Z: i32 = 20;

/// The contextual panel's INSET from the bottom bar's RIGHT edge, as a fraction of the window WIDTH
/// ([`Val::Vw`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO fixed px,
/// so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `right` (the
/// `CELL_PX`-class carve-out, not a domain value). Anchors the panel off the bar's right side
/// so it sits in the bottom-right corner, matching the mockup's contextual cluster to the right
/// of the Stance column. The panel's `bottom` inset reuses the bar's own
/// [`BOTTOM_BAR_PAD_Y_VH`](super::super::bottom_bar::BOTTOM_BAR_PAD_Y_VH) (the sibling weapon /
/// stance panels' bottom anchor), so no dedicated bottom const is needed.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_RIGHT_VW: f32 = 1.5;

/// The contextual panel's WIDTH, as a fraction of the window WIDTH ([`Val::Vw`](bevy::ui::Val) —
/// the responsive-units ruling: relative units only, NO fixed px, so it scales with the window
/// on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `width` (the
/// `CELL_PX`-class carve-out, not a domain value). Sized to the bottom-right contextual cluster
/// region of the mockup (the empty space to the right of the Stance column).
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_WIDTH_VW: f32 = 22.0;

/// The vertical gap between the contextual panel's stacked buttons, as a fraction of the window
/// HEIGHT ([`Val::Vh`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO
/// fixed px, so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `row_gap` (the
/// `CELL_PX`-class carve-out, not a domain value). Separates the per-act contextual buttons
/// stacked in the panel's column.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_ROW_GAP_VH: f32 = 1.0;
