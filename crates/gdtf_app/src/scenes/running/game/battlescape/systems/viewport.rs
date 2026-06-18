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
    WeaponPanelRoot, action_bar::ActionBarRoot, hover_panel::HoverPanelRoot,
    status_panel::StatusPanelRoot,
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
/// - **LEFT** by the WIDER of the status-panel ([`StatusPanelRoot`], top-left) and the
///   weapon-panel ([`WeaponPanelRoot`], bottom-left) widths — both occupy the left margin
///   column, so the inset must clear whichever is wider (GTW-275 AC9),
/// - **BOTTOM** by the TALLER of the action-bar ([`ActionBarRoot`], bottom-centre) and the
///   weapon-panel ([`WeaponPanelRoot`], bottom-left) heights — both occupy the bottom margin
///   row (GTW-275 AC9),
/// - **RIGHT** by the hover-panel ([`HoverPanelRoot`]) width (GTW-274 — it is anchored
///   top-right; `0` when the panel is absent / hidden-and-zero-sized, the same panel-absent
///   allowance as the others),
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
    hover_panels: Query<&ComputedNode, With<HoverPanelRoot>>,
    weapon_panels: Query<&ComputedNode, With<WeaponPanelRoot>>,
) {
    // The primary window's PHYSICAL size — the surface the viewport rect is carved from. No
    // window → nothing to confine.
    let Ok(window) = windows.single() else {
        return;
    };
    let win = window.physical_size();

    // Margins from the panels' PHYSICAL `ComputedNode` sizes (0 when a panel is absent — not
    // spawned yet). `.x`/`.y` are physical px; round to integer pixels for the UVec2 viewport
    // (`u32` saturates a negative/NaN to 0, never panics).
    let status_w = status_panels
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().x));
    let action_h = action_bars
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().y));
    // The bottom-left weapon panel (GTW-275 AC9) occupies BOTH the left column and the bottom
    // row, so its width feeds the LEFT inset and its height the BOTTOM inset.
    let weapon_w = weapon_panels
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().x));
    let weapon_h = weapon_panels
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().y));
    // LEFT = wider of the top-left status panel and the bottom-left weapon panel (AC9).
    let left = status_w.max(weapon_w);
    // BOTTOM = taller of the bottom-centre action bar and the bottom-left weapon panel (AC9).
    let bottom = action_h.max(weapon_h);
    // RIGHT = hover-panel width (GTW-274 — anchored top-right; 0 when absent/zero-sized).
    let right = hover_panels
        .iter()
        .next()
        .map_or(0, |node| physical_px(node.size().x));
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
