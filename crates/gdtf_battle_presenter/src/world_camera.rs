//! The SHARED (renderer-independent) world-camera lifecycle for the GTW-48 battle
//! presenter.
//!
//! A single `Camera2d` carrying the [`WorldCamera`] marker is spawned on entry to the
//! battle and despawned on exit. It is configured to render BENEATH the persistent
//! GTW-120 UI camera (a lower `Camera.order`) and on its own non-zero render layer
//! ([`WORLD_RENDER_LAYER`]) so it does not composite onto the UI camera's default
//! layer-0 surface.
//!
//! This camera is the surface S3+ will draw glyph sprites against and the camera S7's
//! input crate (`gdtf_battle_input`) will query (via `With<WorldCamera>`) to unproject
//! the cursor. It deliberately draws no glyph, loads no atlas, and reads no sim
//! state — it is owned by the SHARED presenter layer (`BattlePresenterPlugin`), NOT a
//! specific renderer, so a later iso renderer swap reuses it unchanged.
//!
//! The spawn/despawn systems are `pub` and param-only (`Commands` / `Query`): the app
//! registers them on the `GameState::BattleScape` boundary because the presenter has no
//! `gdtf_app` dependency and so cannot name `GameState`. The crate dependency chain
//! (input -> presenter -> sim, one-way; see `docs/decisions/0001-rust-bevy-rewrite.md`
//! and `CLAUDE.md`) puts `WorldCamera` in the presenter so the input crate can depend
//! on it.

use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};
use gdtf_battle_sim::{Cell, Faction, GRID_HEIGHT, GRID_WIDTH, Level, PlayerFaction, Position};

use crate::cell_to_world;

/// The non-zero render layer the world camera renders.
///
/// Framework plumbing — the inner `Layer` (`usize`) index fed to
/// [`RenderLayers::layer`], an index into a collection Bevy owns, NOT a domain value
/// (the no-bare-types framework-plumbing carve-out, mirroring the `CELL_PX`-is-a-const
/// reasoning). It MUST be non-zero so the world camera's `RenderLayers` does not
/// intersect layer 0 — the default layer the GTW-120 UI camera falls to (a plain
/// `Camera2d` with no explicit `RenderLayers` resolves to `RenderLayers::layer(0)`).
pub const WORLD_RENDER_LAYER: usize = 1;

/// The render order of the world camera — lower than the UI camera so it renders first.
///
/// Framework plumbing — the `isize` written into `Camera.order` (whose default is `0`).
/// Negative so the world camera renders BENEATH the GTW-120 UI camera, which is a plain
/// `Camera2d` at the default order `0`.
const WORLD_CAMERA_ORDER: isize = -1;

/// Marker for the single SHARED world [`Camera2d`] owned by `BattlePresenterPlugin`.
///
/// This is plumbing around the framework camera (the `Camera2d` itself is exempt from
/// the no-bare-types rule, the same justification `UiCamera` uses), not a domain value.
/// It is `pub` so the S7 input crate (`gdtf_battle_input`) can query `With<WorldCamera>`
/// to read the world camera, and so the app can despawn precisely *this* camera on exit
/// without re-querying every `Camera2d` in the world.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct WorldCamera;

/// Spawns the single SHARED world [`Camera2d`] (the [`WorldCamera`]).
///
/// Registered by the app on `OnEnter(GameState::BattleScape)` (the presenter cannot name
/// `GameState`). It spawns a `Camera2d` carrying the [`WorldCamera`] marker, a
/// [`Camera`] at [`WORLD_CAMERA_ORDER`] (`-1`, below the UI camera's default `0` so it
/// renders first / beneath), and [`RenderLayers::layer`]`(`[`WORLD_RENDER_LAYER`]`)` so
/// it renders its own non-zero layer and does not intersect the UI camera's layer 0.
///
/// Param-only (`Commands`): spawning is `Commands::spawn`, never `&mut World`
/// (`bevy-traps.md` #7).
pub fn spawn_world_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        WorldCamera,
        Camera {
            order: WORLD_CAMERA_ORDER,
            ..default()
        },
        RenderLayers::layer(WORLD_RENDER_LAYER),
    ));
}

/// Despawns every [`WorldCamera`] entity.
///
/// Registered by the app on `OnExit(GameState::BattleScape)` — the same span as the
/// existing `request_battle_teardown`, so the camera survives the whole
/// `Generation → AnimateIn → BattleRunning → AnimateOut → AfterMath` walk and is torn
/// down only when the battle is left. Tolerates zero or one `WorldCamera` (it despawns
/// whatever the `With<WorldCamera>` query yields).
///
/// Param-only (`Commands` + a `Query<Entity, With<WorldCamera>>`): despawning is
/// `Commands::entity(..).despawn()`, never `&mut World` (`bevy-traps.md` #7).
pub fn despawn_world_camera(mut commands: Commands, cameras: Query<Entity, With<WorldCamera>>) {
    for camera in &cameras {
        commands.entity(camera).despawn();
    }
}

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
/// in world units, already folding in window size + scale), falling back to the primary
/// window's size scaled by the projection `scale` when that area is still its uninitialised
/// `default_2d` unit rect (no `camera_system` has run yet — e.g. the very first frame /
/// a headless app). Returns [`None`] when the camera is not orthographic.
fn viewport_half_extent(projection: &Projection, window: Option<&Window>) -> Option<Vec2> {
    let Projection::Orthographic(ortho) = projection else {
        return None;
    };
    let area_half = ortho.area.half_size();
    if area_half.x > 0.0 && area_half.y > 0.0 {
        return Some(area_half);
    }
    // The projection `area` has not been computed yet; derive the half-extent from the
    // window size (the contract's "primary Window for the viewport size") and the scale.
    let window = window?;
    Some(window.size() * 0.5 * ortho.scale)
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
            let world = cell_to_world(Cell::new(pos.x, pos.y), Level::new(0));
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

/// `Update` (battle-gated): clamp the [`WorldCamera`] so its viewport never shows beyond
/// the battlefield bounds — the FINAL word on camera position each frame.
///
/// GTW-249 AC4: whatever moves the camera (the one-shot framing above; the future pan-nav
/// slice GTW-250 — order this `.after` both so it is the last writer), this pulls the
/// translation back inside via [`clamp_camera`]. It reads the camera [`Transform`] + its
/// [`Projection`] (the orthographic visible half-extent) + the primary [`Window`] (the
/// viewport-size fallback before `camera_system` first computes the projection area) + the
/// battlefield world bounds ([`battlefield_world_bounds`], from the grid extent via
/// [`cell_to_world`] — never a hardcoded literal), and writes back the clamped `xy` (z is
/// kept). With a not-yet-orthographic / zero-size viewport it leaves the camera unchanged
/// rather than snapping it.
///
/// Param-only (`Query` / `Res`), no `&mut World` (`bevy-traps.md` #7); battle-scoped gating
/// is applied at registration (`bevy-traps.md` #1).
pub fn clamp_camera_to_bounds(
    mut cameras: Query<(&mut Transform, &Projection), With<WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let window = windows.iter().next();
    let (world_min, world_max) = battlefield_world_bounds();
    for (mut transform, projection) in &mut cameras {
        let Some(half_viewport) = viewport_half_extent(projection, window) else {
            continue;
        };
        let current = Vec2::new(transform.translation.x, transform.translation.y);
        let clamped = clamp_camera(current, half_viewport, world_min, world_max);
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The configured world render layer does not intersect the UI camera's default
    /// layer 0 (the local guarantee behind AC3) — exercised through the real
    /// `RenderLayers::intersects` so it is genuine runtime behaviour, not a constant.
    #[test]
    fn world_render_layer_misses_layer_zero() {
        let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
        let ui_layer = RenderLayers::layer(0);
        assert!(
            !world_layer.intersects(&ui_layer),
            "the world camera's render layer ({WORLD_RENDER_LAYER}) must not intersect the UI \
             camera's default layer 0",
        );
    }

    /// AC1 — `camera_focus` is the centroid (mean) of the supplied points, and `None`
    /// for an empty iterator (so the framing leaves the camera be when no player gangers
    /// exist). Pure, no `App`.
    #[test]
    fn camera_focus_is_the_centroid_or_none() {
        // Three points -> their mean.
        let three = [
            Vec2::new(0.0, 0.0),
            Vec2::new(6.0, 0.0),
            Vec2::new(0.0, 9.0),
        ];
        let mean = camera_focus(three);
        assert!(mean.is_some(), "three points must have a centroid");
        let Some(mean) = mean else { return };
        assert_eq!(
            mean.x.to_bits(),
            2.0_f32.to_bits(),
            "mean x = (0+6+0)/3 = 2"
        );
        assert_eq!(
            mean.y.to_bits(),
            3.0_f32.to_bits(),
            "mean y = (0+0+9)/3 = 3"
        );

        // One point -> that point.
        let one = camera_focus([Vec2::new(4.0, -7.0)]);
        assert!(one.is_some(), "one point must have a centroid");
        let Some(one) = one else { return };
        assert_eq!(
            one,
            Vec2::new(4.0, -7.0),
            "a single point is its own centroid"
        );

        // Empty -> None.
        assert!(
            camera_focus(std::iter::empty::<Vec2>()).is_none(),
            "an empty iterator has no centroid",
        );
    }

    /// AC2 — `clamp_camera` keeps the viewport inside the bounds and centres on the map
    /// midpoint when the map is smaller than the viewport. Relations, not pinned scene
    /// magnitudes. Pure, no `App`.
    #[test]
    fn clamp_camera_keeps_viewport_inside_and_centres_when_smaller() {
        let half = Vec2::new(10.0, 10.0);
        let world_min = Vec2::new(0.0, 0.0);
        let world_max = Vec2::new(100.0, 100.0);

        // Pushed past world_max -> clamped to world_max - half on that axis.
        let past_max = clamp_camera(Vec2::new(1000.0, 1000.0), half, world_min, world_max);
        assert_eq!(
            past_max,
            world_max - half,
            "a translation past world_max clamps to world_max - half",
        );

        // Pushed past world_min -> clamped to world_min + half on that axis.
        let past_min = clamp_camera(Vec2::new(-1000.0, -1000.0), half, world_min, world_max);
        assert_eq!(
            past_min,
            world_min + half,
            "a translation past world_min clamps to world_min + half",
        );

        // A within-bounds translation is unchanged.
        let inside = Vec2::new(50.0, 40.0);
        assert_eq!(
            clamp_camera(inside, half, world_min, world_max),
            inside,
            "a translation already inside the bounds is left unchanged",
        );

        // Map SMALLER than the viewport on both axes (2*half > span) -> the map midpoint.
        let big_half = Vec2::new(80.0, 80.0); // 2*80 = 160 > 100 span on each axis.
        let centred = clamp_camera(Vec2::new(1000.0, -1000.0), big_half, world_min, world_max);
        let midpoint = (world_min + world_max) * 0.5;
        assert_eq!(
            centred, midpoint,
            "when the map is narrower than the viewport, the camera centres on the midpoint",
        );
    }
}
