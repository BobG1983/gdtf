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
    /// It is a BARE [`spawn_panel`](gdtf_ui::spawn_panel) themed box (the bottom-bar precedent: a
    /// bare absolute root resolves the UI camera fine, no wrapper needed), spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by
    /// the act-agnostic `sync_panel_root_visibility` pass (visible iff ANY act button is).
    /// It carries [`GlobalZIndex`](bevy::ui::GlobalZIndex)`(`
    /// [`CONTEXTUAL_PANEL_Z`]`)` so the whole panel subtree (this box + the buttons) stacks ABOVE
    /// the opaque bottom bar it overlaps — without it the bar (a higher-z opaque panel) painted
    /// over the panel and nothing rendered (the GTW-294 occlusion bug). The
    /// `OnExit(BattleRunning)` despawn tears the whole subtree down by THIS marker. Widened
    /// through `support_item!` so the AC tests can assert the box's presence. A unit marker:
    /// presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContextualPanelRoot;
}

/// The contextual panel's stacking order ([`GlobalZIndex`](bevy::ui::GlobalZIndex) — the higher,
/// the nearer the viewer), set strictly ABOVE the bottom bar so the panel draws ON TOP of it.
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`](bevy::ui::GlobalZIndex) (the
/// framework carve-out, not a domain value). The contextual panel is anchored bottom-RIGHT,
/// landing INSIDE the bottom bar's full-width opaque footprint; the bottom bar carries
/// `GlobalZIndex(10)` (`BOTTOM_BAR_Z`) and the weapon + stance cluster that sits on the bar carries
/// `GlobalZIndex(11)` (`PANEL_Z`). With NO `GlobalZIndex` (default `0`) the contextual panel drew
/// BEHIND the opaque bar and rendered nothing (the GTW-294 occlusion bug, proven by a runtime
/// probe: correct camera / parent / transform / size / visibility, yet zero pixels). This value is
/// `20` — comfortably above both the bar (`10`) and the on-bar cluster (`11`), with headroom — so
/// the whole panel subtree (root box + act buttons) draws on top of the bar.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_Z: i32 = 20;

/// The contextual panel's INSET from the window's RIGHT edge, as a fraction of the window WIDTH
/// ([`Val::Vw`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO fixed px,
/// so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `right` (the
/// `CELL_PX`-class carve-out, not a domain value). Anchors the panel off the window's right side
/// so it sits in the bottom-right corner, matching the mockup's contextual cluster to the right
/// of the Stance column.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_RIGHT_VW: f32 = 1.5;

/// The contextual panel's INSET from the window's BOTTOM edge, as a fraction of the window HEIGHT
/// ([`Val::Vh`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO fixed px,
/// so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `bottom` (the
/// `CELL_PX`-class carve-out, not a domain value). Lifts the panel off the window's bottom edge
/// so its buttons sit inside the bottom HUD band rather than flush against the window bottom,
/// matching the mockup.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_BOTTOM_VH: f32 = 3.0;

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
