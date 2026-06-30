//! Canvas **centering** (GTW-500 C2) — the [`center_canvas`] system that keeps the prefab grid
//! centred in the edit viewport when it is smaller than that viewport.
//!
//! ## Why [`UiTransform::translation`], NOT `justify_content` / `align_items: Center`
//!
//! Centre-aligning a scroll CHILD that OVERFLOWS its scroll viewport pushes the content's START
//! before the scroll origin `0` — and [`ScrollPosition`](bevy::ui::ScrollPosition) clamps to
//! `[0, max]`, so that leading region is UNREACHABLE: the top-left of an overflowing grid scrolls
//! off-screen and can never be brought back. That is the GTW-421 off-screen-overflow trap. So
//! centring here is done by [`UiTransform::translation`] instead: the grid stays
//! flex-start-anchored (reachable by scroll when it overflows), and a per-axis translation only
//! NUDGES it to centre WHEN it is SMALLER than the viewport. Translation composites into the
//! rendered [`UiGlobalTransform`] WITHOUT touching Taffy geometry / the scroll clipping (unlike
//! `UiTransform::scale`), so it is the safe centring lever.
//!
//! When the grid OVERFLOWS an axis (content >= viewport) the translation on that axis is `0`, so
//! scrolling works normally; when it is smaller, the translation is half the slack so the grid
//! sits centred. The system re-runs whenever a [`ComputedNode`] changes (a window/panel resize, a
//! grid re-extent, or a zoom re-layout), so the centring tracks the live layout.

use bevy::{
    prelude::*,
    ui::{ComputedNode, UiTransform, Val2},
};

use super::types::{CanvasRoot, CanvasScrollArea};

/// `Update` (in `Editing`): per-axis CENTRE the [`CanvasRoot`] within the canvas's
/// [`CanvasScrollArea`] viewport when the grid is smaller than the viewport, else leave it
/// flex-start (C2).
///
/// Compares the [`ComputedNode::size`] of the root vs the CANVAS scroll area (both PHYSICAL px) and,
/// per axis, sets the root's [`UiTransform::translation`] to half the viewport-minus-content slack
/// (converted to LOGICAL px via the root's
/// [`inverse_scale_factor`](bevy::ui::ComputedNode::inverse_scale_factor), since a `UiTransform`
/// translation is a [`Val2`] of logical-px values) when the content is SMALLER than the viewport,
/// and to `0` when it overflows (so scrolling stays normal — the GTW-421 trap). Re-runs whenever a
/// `ComputedNode` changed (resize / re-extent / zoom), reading the area size via a separate query
/// (the area's `ComputedNode` is `Without<CanvasRoot>` so the two are disjoint — bevy-traps #7).
/// The viewport is the CANVAS's scroll area specifically ([`CanvasScrollArea`], NOT a bare
/// [`ScrollListArea`](gdtf_ui::ScrollListArea)): the editor spawns three scroll areas (palette /
/// canvas / right panel), so centring against the first arbitrary one would size the canvas to the
/// wrong viewport. The translation is written only when it actually changes (a `!=` guard) so a
/// settled layout does not churn.
pub(crate) fn center_canvas(
    mut roots: Query<(&ComputedNode, &mut UiTransform), With<CanvasRoot>>,
    areas: Query<&ComputedNode, (With<CanvasScrollArea>, Without<CanvasRoot>)>,
) {
    // The canvas hangs inside exactly ONE scroll area — its own (`CanvasScrollArea`), not the two
    // other editor lists; take that viewport's size (physical px).
    let Some(viewport) = areas.iter().next().map(ComputedNode::size) else {
        return;
    };
    for (computed, mut transform) in &mut roots {
        let content = computed.size();
        let inv = computed.inverse_scale_factor();
        // Per axis: half the slack (logical px) when the content is smaller than the viewport, else
        // 0 (overflow -> flex-start so scrolling reaches the whole grid).
        let offset_x = centre_offset(content.x, viewport.x) * inv;
        let offset_y = centre_offset(content.y, viewport.y) * inv;
        let want = Val2::px(offset_x, offset_y);
        if transform.translation != want {
            transform.translation = want;
        }
    }
}

/// The per-axis centring offset (PHYSICAL px) for content of `content` px inside a `viewport` px
/// viewport: half the slack when the content fits, `0` when it overflows.
fn centre_offset(content: f32, viewport: f32) -> f32 {
    let slack = viewport - content;
    if slack > 0.0 { slack * 0.5 } else { 0.0 }
}
