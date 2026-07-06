//! The GTW-249 camera framing: the battle-start frame-on-units one-shot, the
//! viewport-bounds clamp, and their pure geometry helpers.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::{Cell, Faction, Level, Position},
};

use super::{
    marker::WorldCamera,
    tuning::{BoundsMarginWorld, PanTuning},
};
use crate::cell_to_world;

/// The centroid (mean) of the supplied world points, or [`None`] when empty.
///
/// GTW-249 AC1: the camera frames on the player gangers by snapping to the MEAN of
/// their world-space positions, so all the player's units sit around the centre of the
/// view. Returns [`None`] when there are no points (no player gangers) so the caller
/// leaves the camera where it is rather than snapping to the origin.
///
/// Pure / total — no `App`, no `World`, no side effects (unit-tested AC1). `Vec2` is
/// framework-math plumbing here (the `no-bare-types.md` carve-out the contract calls out
/// for raw world coords), not a domain newtype; it mirrors how the presenter already
/// passes world coords (`cell_to_world` returns a bare `Vec3`).
#[must_use]
pub fn camera_focus(centers: impl IntoIterator<Item = Vec2>) -> Option<Vec2> {
    let mut count: u32 = 0;
    let mut sum = Vec2::ZERO;
    for center in centers {
        sum += center;
        count += 1;
    }
    if count == 0 {
        return None;
    }
    // `count >= 1` here, so the divisor is non-zero; `u32 -> f32` is lossless for any
    // realistic ganger count (`f32::from` rejects `u32`, so the documented cast is the
    // total path — there is no panic).
    #[expect(
        clippy::cast_precision_loss,
        reason = "ganger counts are tiny (a handful per gang); u32->f32 is exact in range"
    )]
    let divisor = count as f32;
    Some(sum / divisor)
}

/// Clamps an orthographic camera `translation` so its viewport `[t - half, t + half]`
/// stays within `[world_min, world_max]` on each axis; centres on the map midpoint for an
/// axis where the map is SMALLER than the viewport.
///
/// GTW-249 AC2: the bounds clamp keeps the battlefield filling (or centred within) the
/// view so the camera can never show beyond the battlefield edge. Per axis: if the map
/// span (`world_max - world_min`) is at least the viewport span (`2 * half_viewport`), the
/// translation is clamped into `[world_min + half, world_max - half]`; otherwise (the map
/// is narrower than the view on that axis) the result is the map midpoint, so the smaller
/// map sits centred and does not jitter against the two opposing clamps.
///
/// Pure / total — no `App`, no `World` (unit-tested AC2). `Vec2` is framework-math
/// plumbing (the contract's carve-out), not a domain newtype.
#[must_use]
pub fn clamp_camera(
    translation: Vec2,
    half_viewport: Vec2,
    world_min: Vec2,
    world_max: Vec2,
) -> Vec2 {
    Vec2::new(
        clamp_axis(translation.x, half_viewport.x, world_min.x, world_max.x),
        clamp_axis(translation.y, half_viewport.y, world_min.y, world_max.y),
    )
}

/// One axis of [`clamp_camera`]: clamp into `[min + half, max - half]`, or centre on the
/// axis midpoint when the map span is narrower than the viewport span.
///
/// Pure / total scalar helper (the per-axis core of the AC2 relation).
fn clamp_axis(value: f32, half: f32, min: f32, max: f32) -> f32 {
    let span = max - min;
    if span < 2.0 * half {
        // Map narrower than the view on this axis: centre on the midpoint.
        return span.mul_add(0.5, min);
    }
    value.clamp(min + half, max - half)
}

/// The battlefield's ground-plane world bounds as `(world_min, world_max)`.
///
/// Projects the four corner cells of the `GRID_WIDTH x GRID_HEIGHT` ground extent
/// (`gdtf_battle_sim`'s structural grid constants — never a hardcoded `60`) through
/// [`cell_to_world`] on the ground storey ([`Level`] 0) and takes the component-wise
/// min / max, so the bounds track the SAME projection the ganger / terrain sprites use.
/// Because row 0 maps to the TOP (Bevy `+Y` up; `cell_to_world` negates `cell.y`), the
/// `y` extent runs from the bottom row up to row 0; `min`/`max` is taken component-wise so
/// the result is a well-ordered axis-aligned box regardless of that sign flip.
///
/// `GRID_WIDTH`/`GRID_HEIGHT` are `usize`; the `i32::try_from(..).unwrap_or(i32::MAX)`
/// (the sim's `magazine.rs` `in_bounds` idiom) keeps the cell-coordinate conversion
/// panic-free and wrap-free (`unwrap_or` is not the denied `unwrap`).
fn battlefield_world_bounds() -> (Vec2, Vec2) {
    let max_x = i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX);
    let max_y = i32::try_from(GRID_HEIGHT).unwrap_or(i32::MAX);
    let ground = Level::new(0);

    // The four corner cells of the ground extent, projected to world space. The extent is
    // [0, GRID_WIDTH] x [0, GRID_HEIGHT] cells: the far corner uses the cell count (one
    // past the last index) so the bounds enclose the FULL last cell, not its near edge.
    let corners = [
        cell_to_world(Cell::new(0, 0), ground),
        cell_to_world(Cell::new(max_x, 0), ground),
        cell_to_world(Cell::new(0, max_y), ground),
        cell_to_world(Cell::new(max_x, max_y), ground),
    ];

    let mut world_min = Vec2::new(corners[0].x, corners[0].y);
    let mut world_max = world_min;
    for corner in corners {
        let p = Vec2::new(corner.x, corner.y);
        world_min = world_min.min(p);
        world_max = world_max.max(p);
    }
    (world_min, world_max)
}

/// The half-extent (world units) of the camera's visible viewport.
///
/// Prefers the orthographic projection's computed `area` (Bevy's `camera_system` keeps it
/// in world units, already folding in window size + scale AND the sub-rect [`Camera::viewport`]
/// once one is set — so the STEADY-STATE path already reflects the GTW-271 map viewport),
/// falling back to the viewport / window size scaled by the projection `scale` when that area
/// is still its uninitialised `default_2d` unit rect (no `camera_system` has run yet — e.g. the
/// very first frame / a headless app). Returns [`None`] when the camera is not orthographic.
///
/// GTW-271 AC6: the pre-`camera_system` fallback is now VIEWPORT-AWARE — when a sub-rect
/// [`Camera::viewport`] is set (the app confines the map to a central region), the fallback
/// derives the half-extent from the VIEWPORT's logical size (the viewport physical size divided
/// by the window scale factor), NOT the full window size. The old full-window fallback
/// over-estimated the visible half-extent once the map is a sub-rect, so the bounds clamp let
/// the map drift. With no viewport set yet it keeps the full-window fallback.
fn viewport_half_extent(
    camera: &Camera,
    projection: &Projection,
    window: Option<&Window>,
) -> Option<Vec2> {
    let Projection::Orthographic(ortho) = projection else {
        return None;
    };
    let area_half = ortho.area.half_size();
    if area_half.x > 0.0 && area_half.y > 0.0 {
        return Some(area_half);
    }
    // The projection `area` has not been computed yet; derive the half-extent from the
    // VISIBLE region (the contract's "primary Window for the viewport size") and the scale.
    let window = window?;
    // GTW-271: prefer the sub-rect viewport's LOGICAL size when one is set, so the fallback
    // matches the confined map region; `to_logical` is None until `camera_system` runs, so
    // convert via the window's `scale_factor` (physical viewport px → logical px).
    let logical_size = match &camera.viewport {
        Some(viewport) => viewport.physical_size.as_vec2() / window.scale_factor(),
        None => window.size(),
    };
    Some(logical_size * 0.5 * ortho.scale)
}

/// `Update` (battle-gated): frame the [`WorldCamera`] on the player gangers' centroid ONCE
/// per battle, the first frame any player-faction ganger exists.
///
/// GTW-249 AC3: at battle start the camera snaps so the player's soldiers are centred and
/// visible. It runs at most once per battle — a `Local<bool>` "already framed" latch flips
/// true on the framing frame and suppresses every later run, so the one-shot framing does
/// NOT fight the sibling pan-nav slice (GTW-250) that will move the camera afterwards. With
/// no player gangers yet (`camera_focus` is [`None`]) the latch stays false, so the framing
/// waits for the first frame they exist rather than snapping to the origin.
///
/// Reads the player-faction gangers' [`Position`] (the ganger-draw `(&Faction, &Position)`
/// read precedent), projects each through [`cell_to_world`], and writes the centroid into
/// the camera `Transform.translation.xy` (z is kept — top-down ground plane only, no
/// multi-level framing). Param-only (`Query` / `Res` / `Local`), no `&mut World`
/// (`bevy-traps.md` #7); battle-scoped gating is applied at registration (`bevy-traps.md`
/// #1).
pub fn frame_camera_on_units(
    mut already_framed: Local<bool>,
    player: Res<PlayerFaction>,
    gangers: Query<(&Faction, &Position)>,
    mut cameras: Query<&mut Transform, With<WorldCamera>>,
) {
    if *already_framed {
        return;
    }
    let player_faction = **player;
    let centers = gangers
        .iter()
        .filter(|(faction, _)| **faction == player_faction)
        .map(|(_, pos)| {
            // The ganger's ground cell via the canonical CellLevel::cell accessor
            // (GTW-565); framing is planar, so the storey is pinned to 0.
            let world = cell_to_world(pos.cell(), Level::new(0));
            Vec2::new(world.x, world.y)
        });
    let Some(focus) = camera_focus(centers) else {
        return;
    };
    for mut transform in &mut cameras {
        transform.translation.x = focus.x;
        transform.translation.y = focus.y;
    }
    *already_framed = true;
}

/// `Update` (battle-gated): clamp the [`WorldCamera`] so its viewport stays within the
/// battlefield bounds RELAXED by the off-level pan margin — the FINAL word on camera position
/// each frame.
///
/// GTW-249 AC4: whatever moves the camera (the one-shot framing above; the pan-nav slice
/// GTW-250 — ordered `.after` both so this is the last writer), this pulls the translation
/// back inside via [`clamp_camera`]. It reads the camera [`Camera`] (the GTW-271 sub-rect
/// [`Camera::viewport`]) + its [`Transform`] + its [`Projection`] (the orthographic visible
/// half-extent) + the primary [`Window`] (the viewport-size fallback before `camera_system`
/// first computes the projection area) + the battlefield world bounds
/// ([`battlefield_world_bounds`], from the grid extent via [`cell_to_world`] — never a
/// hardcoded literal), and writes back the clamped `xy` (z is kept). With a not-yet-orthographic
/// / zero-size viewport it leaves the camera unchanged rather than snapping it.
///
/// GTW-381: the clamp is now RELAXED by a fixed world-space MARGIN
/// ([`PanTuning::bounds_margin_world`]) so the camera CAN pan up to that many world units PAST
/// the level edge — clamped at `bounds + margin`, never further and never at the old hard
/// bounds (bounded + padding). The margin GROWS the world bounds box (`world_min - margin` /
/// `world_max + margin`) BEFORE [`clamp_camera`] runs, so it feeds the SAME zoom-aware per-axis
/// math: the half-viewport still carries the zoom, while the margin is a fixed world distance
/// independent of zoom (the same world-space slack is allowed at any zoom level, C3). The tuning
/// is taken as `Option<Res<PanTuning>>` so a headless app with no `AssetServer` (the resource
/// never loads) falls back to the shipped [`BoundsMarginWorld`](crate::BoundsMarginWorld)
/// default (`bevy-traps.md` #1).
///
/// Param-only (`Query` / `Res`), no `&mut World` (`bevy-traps.md` #7); battle-scoped gating
/// is applied at registration (`bevy-traps.md` #1).
pub fn clamp_camera_to_bounds(
    tuning: Option<Res<PanTuning>>,
    mut cameras: Query<(&Camera, &mut Transform, &Projection), With<WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let window = windows.iter().next();
    // The off-level pan margin (a FIXED world-space distance), or the shipped default when the
    // hot-reloadable tuning table is absent (a headless app without an AssetServer) — GTW-381.
    let margin = tuning
        .as_ref()
        .map_or_else(BoundsMarginWorld::default, |t| t.bounds_margin_world);
    // Grow the hard battlefield bounds by the margin BEFORE clamping: the camera may pan up to
    // `margin` world units past the level edge, and is clamped at `bounds + margin`. The margin
    // is world-space (the half-viewport carries the zoom), so this slack is zoom-independent (C3).
    let (hard_min, hard_max) = battlefield_world_bounds();
    let world_min = hard_min - Vec2::splat(*margin);
    let world_max = hard_max + Vec2::splat(*margin);
    for (camera, mut transform, projection) in &mut cameras {
        let Some(half_viewport) = viewport_half_extent(camera, projection, window) else {
            continue;
        };
        let current = Vec2::new(transform.translation.x, transform.translation.y);
        let clamped = clamp_camera(current, half_viewport, world_min, world_max);
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}
