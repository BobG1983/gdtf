//! GTW-271 — confine the world render to a central viewport sub-rect.
//!
//! The world MAP is drawn by the presenter's `WorldCamera`, but the APP owns the UI layout, so
//! the app computes the central map rect (the window MINUS the UI panel margins) and writes it
//! into the camera's [`Camera::viewport`]. `gdtf_app` deps the presenter and `WorldCamera` is
//! `pub`, so the app can query `&mut Camera, With<WorldCamera>` and set its viewport; the
//! presenter never reads `gdtf_app`/`gdtf_ui` types (the one-way crate edge holds — ADR-0001).

use bevy::{camera::Viewport, prelude::*, window::PrimaryWindow};
use gdtf_battle_presenter::WorldCamera;

use crate::scenes::running::game::battlescape::{
    action_bar::ActionBarRoot, status_panel::StatusPanelRoot,
};

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

/// `Update` (battle-gated): set the [`WorldCamera`]'s [`Camera::viewport`] to the central map
/// region — the window MINUS the UI panel margins (GTW-271 AC1).
///
/// MEASURES the app's panel roots' [`ComputedNode`] sizes (physical px — `ComputedNode.size()`
/// is already in physical pixels, so the window's `scale_factor` is already folded in and no
/// further logical→physical conversion is applied; see the flag below) and insets the world
/// map by:
///
/// - **LEFT** by the status-panel ([`StatusPanelRoot`]) width (it is anchored top-left),
/// - **BOTTOM** by the action-bar ([`ActionBarRoot`]) height (it is anchored bottom-centre),
/// - **RIGHT** by the hover-panel width — that panel does not exist yet, so the right inset is
///   `0` (the AC1 "tolerate the hover panel not existing → 0" allowance; when it lands, add its
///   root marker to the right-inset query),
/// - **TOP** minimal (`0`).
///
/// The viewport is in PHYSICAL px ([`Viewport::physical_position`] / [`Viewport::physical_size`],
/// UVec2): `physical_position = (left, top)` and `physical_size = window.physical_size() -
/// (left+right, top+bottom)`, computed with `saturating_sub` so a panel wider/taller than the
/// window can never produce a zero-or-negative size that wraps.
///
/// # Cadence (AC1 / AC7 flag)
///
/// This is the AC1 "idempotent every-frame write" first cut: it runs every `Update` the battle
/// is live (gated `resource_exists::<BattleInProgress>` at registration). Running every frame is
/// a SUPERSET of AC7's "recompute on `WindowResized` AND at battle start" — it necessarily
/// recomputes on the frame(s) after a resize and on the battle-start frames once the camera +
/// panels exist (the viewport is `None` until then, so a panel-less / camera-less early frame
/// simply writes a full-window viewport, corrected the next frame as the panels' `ComputedNode`
/// sizes settle). So no separate `MessageReader<WindowResized>` path is needed; the every-frame
/// write covers it. FLAGGED here per the contract.
///
/// Param-only (`Query` / `Res`), no `&mut World` (`bevy-traps.md` #7); the live-battle gate
/// (`bevy-traps.md` #1) is applied at registration. It does not order against the presenter's
/// pan/clamp — those READ the viewport rect, so this writer simply needs to run in `Update`; the
/// pan/clamp read whatever viewport was last written (a one-frame settle, harmless for a camera
/// rect).
pub(in crate::scenes::running::game::battlescape) fn set_world_viewport(
    mut cameras: Query<&mut Camera, With<WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    status_panels: Query<&ComputedNode, With<StatusPanelRoot>>,
    action_bars: Query<&ComputedNode, With<ActionBarRoot>>,
) {
    // The primary window's PHYSICAL size — the surface the viewport rect is carved from. No
    // window → nothing to confine.
    let Ok(window) = windows.single() else {
        return;
    };
    let win = window.physical_size();

    // Margins from the panels' PHYSICAL `ComputedNode` sizes (0 when a panel is absent — not
    // spawned yet, or the future hover panel). `.x`/`.y` are physical px; round to integer
    // pixels for the UVec2 viewport (`u32` saturates a negative/NaN to 0, never panics).
    let left = status_panels
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().x));
    let bottom = action_bars
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().y));
    // RIGHT = hover-panel width — that panel does not exist yet, so 0 (AC1 allowance).
    let right = 0;
    // TOP minimal.
    let top = 0;

    // Inset: position at the top-left margin corner, size = window minus the margins, clamped at
    // 0 via `saturating_sub` so an oversized panel can never wrap to a huge (or zero) viewport.
    let physical_position = UVec2::new(left, top);
    let physical_size = UVec2::new(
        win.x.saturating_sub(left.saturating_add(right)),
        win.y.saturating_sub(top.saturating_add(bottom)),
    );

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
