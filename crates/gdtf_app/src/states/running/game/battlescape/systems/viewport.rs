//! GTW-271 / GTW-275 layout overhaul — confine the world render to leave room for the bottom
//! bar ONLY.
//!
//! The world MAP is drawn by the presenter's `WorldCamera`, but the APP owns the UI layout, so
//! the app computes the map rect (the window MINUS the bottom-bar height) and writes it into the
//! camera's [`Camera::viewport`]. `gdtf_app` deps the presenter and `WorldCamera` is `pub`, so
//! the app can query `&mut Camera, With<WorldCamera>` and set its viewport; the presenter never
//! reads `gdtf_app`/`gdtf_ui` types (the one-way crate edge holds — ADR-0001).
//!
//! GTW-275 layout overhaul (items 1 / 4): the map is the FULL-WINDOW background. The status
//! panel (top-left) and hover panel (top-right) are absolute UI OVERLAYS on top of the map (the
//! UI camera draws over the world camera) — they contribute NOTHING to the viewport inset. The
//! ONLY UI that reduces the map is the opaque BOTTOM BAR: the map is full width, full height
//! MINUS the bottom bar (no left / right / top inset — this reverses the GTW-271 side insets).

use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::WorldCamera;

use crate::states::running::game::battlescape::BottomBarRoot;

/// Round a physical-px [`ComputedNode`] dimension (`f32`, always `>= 0`) to integer physical
/// pixels for the [`Viewport`] (`UVec2`).
///
/// `ComputedNode` sizes are non-negative physical pixels, so the rounded value is a small
/// non-negative integer well within `u32`; the localized `#[expect]` (the framing.rs
/// `cast_precision_loss` precedent) keeps the documented, total cast rather than threading a
/// fallible conversion through. A negative / `NaN` input (which `ComputedNode` never produces)
/// saturates to `0`, never panics.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "ComputedNode sizes are small non-negative physical px; rounding to u32 is total \
              and in-range (a negative/NaN saturates to 0, never panics)"
)]
const fn physical_px(value: f32) -> u32 {
    value.round() as u32
}

/// `Update` (battle-gated): set the [`WorldCamera`]'s [`Camera::viewport`] to the map region —
/// the FULL window MINUS only the bottom-bar height (GTW-275 layout overhaul items 1 / 4).
///
/// MEASURES the app's [`BottomBarRoot`] [`ComputedNode`] height (physical px — `ComputedNode.
/// size()` is already in physical pixels, so the window's `scale_factor` is already folded in
/// and no further logical→physical conversion is applied) and insets the world map by:
///
/// - **BOTTOM** by the bottom-bar height ([`BottomBarRoot`]) — the ONE opaque strip that
///   reduces the map; `0` when the bar is absent / zero-sized (the same panel-absent allowance
///   as before — a full-window viewport, corrected the next frame as the bar settles),
/// - **LEFT / RIGHT / TOP** minimal (`0`): the status + hover panels are corner OVERLAYS that do
///   NOT inset the map (the GTW-271 side insets are reversed — items 2 / 3 / 4).
///
/// The viewport is in PHYSICAL px ([`Viewport::physical_position`] / [`Viewport::physical_size`],
/// UVec2): `physical_position = (0, 0)` (top-left, no left/top inset) and `physical_size =
/// window.physical_size() - (0, bottom)`, computed with `saturating_sub` so a bar taller than
/// the window can never produce a zero-or-negative size that wraps.
///
/// # Cadence (AC1 / AC7 flag)
///
/// This is the AC1 "idempotent every-frame write": it runs every `Update` the battle is live
/// (gated `resource_exists::<BattleInProgress>` at registration). Running every frame is a
/// SUPERSET of "recompute on `WindowResized` AND at battle start" — it necessarily recomputes on
/// the frame(s) after a resize and on the battle-start frames once the camera + bar exist (the
/// viewport is `None` until then, so a bar-less / camera-less early frame simply writes a
/// full-window viewport, corrected the next frame as the bar's `ComputedNode` height settles).
/// So no separate `MessageReader<WindowResized>` path is needed; the every-frame write covers it.
///
/// Param-only (`Query` / `Res`), no `&mut World` (`bevy-traps.md` #7); the live-battle gate
/// (`bevy-traps.md` #1) is applied at registration. It does not order against the presenter's
/// pan/clamp — those READ the viewport rect, so this writer simply needs to run in `Update`; the
/// pan/clamp read whatever viewport was last written (a one-frame settle, harmless for a camera
/// rect).
pub(in crate::states::running::game::battlescape) fn set_world_viewport(
    mut cameras: Query<&mut Camera, With<WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    bottom_bars: Query<&ComputedNode, With<BottomBarRoot>>,
) {
    // The primary window's PHYSICAL size — the surface the viewport rect is carved from. No
    // window → nothing to confine.
    let Ok(window) = windows.single() else {
        return;
    };
    let win = window.physical_size();

    // The ONLY inset: the bottom-bar height from its PHYSICAL `ComputedNode` size (0 when the
    // bar is absent — not spawned yet). `.y` is physical px; round to integer pixels for the
    // UVec2 viewport (`u32` saturates a negative/NaN to 0, never panics).
    let bottom = bottom_bars
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().y));

    // Inset: position at the window top-left (no left / top inset — the corner panels are
    // overlays), size = full width × (height − bottom-bar height), clamped at 0 via
    // `saturating_sub` so an oversized bar can never wrap to a huge (or zero) viewport.
    let physical_position = UVec2::ZERO;
    let physical_size = UVec2::new(win.x, win.y.saturating_sub(bottom));

    for mut camera in &mut cameras {
        // Write only on a real change so an idempotent rewrite does not spuriously trip the
        // camera's change detection (`DerefMut`) every frame. `Viewport` is not `PartialEq`, so
        // compare the position + size fields directly (depth is constant `0.0..1.0`).
        let unchanged = camera.viewport.as_ref().is_some_and(|current| {
            current.physical_position == physical_position && current.physical_size == physical_size
        });
        if unchanged {
            continue;
        }
        camera.viewport = Some(Viewport {
            physical_position,
            physical_size,
            depth: 0.0..1.0,
        });
    }
}
