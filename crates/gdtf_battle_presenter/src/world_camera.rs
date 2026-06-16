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

use bevy::{
    camera::visibility::RenderLayers, input::gamepad::Gamepad, prelude::*, window::PrimaryWindow,
};
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

// ---------------------------------------------------------------------------------
// GTW-250: pan navigation — mouse-edge + keyboard (WASD/arrows) + gamepad right stick.
// ---------------------------------------------------------------------------------

/// The pan speed of the [`WorldCamera`], in world units per second.
///
/// A VIEW tunable (how fast the camera glides under player navigation), not combat or
/// theme tuning — so it lives as a presenter-level const here with a doc-comment, NOT in
/// a `.ron` data file (the contract's view-config ruling). A newtype with a private inner
/// `f32` + derived [`Deref`](std::ops::Deref) (the house style for a domain value,
/// `no-bare-types.md`): the
/// speed is a domain quantity (world-units/sec), never a bare `f32`.
///
/// FOLLOW-UP (flagged per the contract): if the user later wants pan speed authored / hot-
/// swappable, promote this to a presenter view-config `.ron` resource — it is deliberately
/// a const for the first cut.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct PanSpeed(f32);

impl PanSpeed {
    /// Construct a [`PanSpeed`] from world-units-per-second.
    #[must_use]
    pub const fn new(units_per_second: f32) -> Self {
        Self(units_per_second)
    }
}

/// The mouse-edge band thickness, in logical screen pixels.
///
/// The cursor is "at an edge" (and pans the camera that way) when it sits within this many
/// logical pixels of a window edge. A VIEW tunable, so a presenter const with a doc-comment
/// (not `.ron`), and a newtype over a private `f32` ([`Deref`](std::ops::Deref)) per
/// `no-bare-types.md`.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct EdgeBandPx(f32);

impl EdgeBandPx {
    /// Construct an [`EdgeBandPx`] from a logical-pixel band thickness.
    #[must_use]
    pub const fn new(pixels: f32) -> Self {
        Self(pixels)
    }
}

/// The gamepad-stick deadzone: a stick magnitude at or below this contributes no pan.
///
/// A unitless `[0, 1]` analog-stick magnitude threshold below which the right stick is
/// treated as centred (no drift). A VIEW tunable (a presenter const, not `.ron`) and a
/// newtype over a private `f32` ([`Deref`](std::ops::Deref)) per `no-bare-types.md`.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct StickDeadzone(f32);

impl StickDeadzone {
    /// Construct a [`StickDeadzone`] from a unitless `[0, 1]` magnitude threshold.
    #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// The shipping pan speed: world units the camera glides per second under navigation.
///
/// A view const (see [`PanSpeed`]). Chosen so the whole 60-cell battlefield can be crossed
/// in a couple of seconds at the 16-px cell pitch ([`CELL_PX`](crate::CELL_PX)); FLAGGED for
/// data-driving later.
pub const PAN_SPEED: PanSpeed = PanSpeed::new(600.0);

/// The shipping mouse-edge band: cursor within this many logical pixels of a window edge
/// pans the camera that way (see [`EdgeBandPx`]).
pub const EDGE_BAND_PX: EdgeBandPx = EdgeBandPx::new(24.0);

/// The shipping gamepad right-stick deadzone (see [`StickDeadzone`]): a stick magnitude at
/// or below this is ignored so a resting stick never drifts the camera.
pub const STICK_DEADZONE: StickDeadzone = StickDeadzone::new(0.15);

/// The camera-space pan direction implied by the mouse cursor's position within the edge
/// bands of a `size`-pixel window, with the screen-y → camera-y flip baked in.
///
/// GTW-250 AC1. The window's origin is TOP-LEFT and screen-y grows DOWNWARD (Bevy's
/// `cursor_position`), while the camera's `+Y` is UP — so a cursor near the TOP edge
/// (`cursor.y < edge`) pans the camera UP (`+Y`) and a cursor near the BOTTOM edge
/// (`cursor.y > size.y - edge`) pans it DOWN (`-Y`); the x axis needs no flip (cursor left →
/// `-X`, right → `+X`). A cursor in NO band (the centre region) → [`Vec2::ZERO`]. The result
/// is a small integer-ish combination of unit axes (`-1`/`0`/`+1` per axis); the caller
/// normalises the summed multi-source direction so a corner is not faster than an edge.
///
/// Pure / total — no `App`, no `World` (unit-tested AC1). `Vec2` is framework-math plumbing
/// (the GTW-249 carve-out for raw screen / direction coords), not a domain newtype.
#[must_use]
pub fn mouse_edge_dir(cursor: Vec2, size: Vec2, edge: EdgeBandPx) -> Vec2 {
    let band = *edge;
    let mut dir = Vec2::ZERO;
    if cursor.x < band {
        dir.x -= 1.0;
    } else if cursor.x > size.x - band {
        dir.x += 1.0;
    }
    // Screen-y → camera-y FLIP: top band (small screen y) pans the camera UP (+Y); bottom
    // band (large screen y) pans it DOWN (-Y).
    if cursor.y < band {
        dir.y += 1.0;
    } else if cursor.y > size.y - band {
        dir.y -= 1.0;
    }
    dir
}

/// The camera-space pan direction implied by the WASD / arrow pan keys.
///
/// GTW-250 AC2. `up` (W / ↑) → `+Y`, `down` (S / ↓) → `-Y`, `left` (A / ←) → `-X`,
/// `right` (D / →) → `+X`; opposite keys CANCEL (W+S → no y, A+D → no x). The result is a
/// `-1`/`0`/`+1` combination per axis; the caller normalises the summed direction so a
/// diagonal (W+D) is not faster than a cardinal (W).
///
/// Pure / total — no `App`, no `World` (unit-tested AC2). `Vec2` is framework-math plumbing.
///
/// The four `bool` params are the contract's specified signature
/// (`keyboard_pan_dir(up, down, left, right: bool) -> Vec2`) — the four independent pan-key
/// pressed-states, each a genuine input the system fills from `ButtonInput<KeyCode>` (WASD +
/// arrow aliases). The localized `#[expect]` (the file's `cast_precision_loss` precedent)
/// keeps that exact, documented signature rather than narrowing it; the pedantic
/// `fn_params_excessive_bools` gate is the only thing it suppresses.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the four pan-key pressed states are the contract's specified keyboard_pan_dir \
              signature (up/down/left/right); they are independent inputs, not a flag soup"
)]
#[must_use]
pub fn keyboard_pan_dir(up: bool, down: bool, left: bool, right: bool) -> Vec2 {
    let x = f32::from(right) - f32::from(left);
    let y = f32::from(up) - f32::from(down);
    Vec2::new(x, y)
}

/// The camera-space pan direction implied by the gamepad RIGHT stick, after the deadzone.
///
/// GTW-250 AC3. A stick magnitude at or below `deadzone` → [`Vec2::ZERO`] (a resting stick
/// never drifts the camera); past the deadzone the stick passes through unchanged, with its
/// analog magnitude preserved (so a slight push pans slowly, a full push fast). Bevy's
/// `right_stick()` already reports stick-UP as `+Y`, matching the camera `+Y`-up convention,
/// so no flip is applied.
///
/// Pure / total — no `App`, no `World` (unit-tested AC3). `Vec2` is framework-math plumbing.
#[must_use]
pub fn stick_pan_dir(stick: Vec2, deadzone: StickDeadzone) -> Vec2 {
    if stick.length() <= *deadzone {
        return Vec2::ZERO;
    }
    stick
}

/// The per-second pan VELOCITY for a (summed, multi-source) `dir` at `speed`.
///
/// GTW-250 AC4. A [`Vec2::ZERO`] direction → zero velocity (no input → no drift). The
/// combined direction is NORMALISED when its length exceeds 1 so a diagonal keyboard combo
/// (length `√2`) is not faster than a cardinal one — the documented choice: keyboard / mouse
/// edges contribute unit axes and must not let diagonals out-run cardinals, while an analog
/// stick (magnitude < 1) keeps its sub-unit magnitude so a gentle push pans gently. The
/// result is `world-units/sec`; the caller multiplies by `delta_secs()` to get this frame's
/// translation delta.
///
/// Pure / total — no `App`, no `World` (unit-tested AC4). `Vec2` is framework-math plumbing.
#[must_use]
pub fn pan_velocity(dir: Vec2, speed: PanSpeed) -> Vec2 {
    let length = dir.length();
    if length == 0.0 {
        return Vec2::ZERO;
    }
    // Clamp the combined direction to at most unit length: a length > 1 (a diagonal of
    // unit-axis sources) is normalised so diagonals are not faster than cardinals; a length
    // <= 1 (a single axis, or a sub-unit analog stick) passes through so analog magnitude
    // still scales speed.
    let clamped = if length > 1.0 { dir / length } else { dir };
    clamped * *speed
}

/// `Update` (battle-gated): pan the [`WorldCamera`] each frame from the three navigation
/// sources, BEFORE the [`clamp_camera_to_bounds`] clamp so the camera can never pan off the
/// battlefield.
///
/// GTW-250: sums the camera-space pan directions from the mouse at a screen edge
/// ([`mouse_edge_dir`]), the keyboard ([`keyboard_pan_dir`] — WASD + arrows), and the gamepad
/// RIGHT stick ([`stick_pan_dir`]), turns the summed direction into a per-second velocity via
/// [`pan_velocity`] (which normalises so diagonals are not faster than cardinals), scales by
/// `time.delta_secs()`, and adds the result to the camera `Transform.translation.xy` (z is
/// kept — top-down ground plane only, no zoom / multi-level). It emits NO sim message — the
/// camera is the VIEW, so pan navigation is presenter-only.
///
/// Reads the three input sources via params: `Res<ButtonInput<KeyCode>>` (keyboard),
/// `Query<&Window, With<PrimaryWindow>>` (the cursor + window size for the mouse edge — the
/// cursor is `None` when off the window, contributing nothing that frame), and `Query<&Gamepad>`
/// (the gamepad is an ENTITY-component in Bevy 0.18, NOT the pre-0.15 `Res<Axis<GamepadAxis>>`).
/// Movement scales by `Res<Time>`'s `delta_secs()`. Param-only (`Res` / `Query`), no `&mut World`
/// (`bevy-traps.md` #7); the battle gate (`bevy-traps.md` #1) and the `.before(clamp)` ordering
/// (`bevy-traps.md` #3) are applied at registration.
pub fn pan_camera(
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut cameras: Query<&mut Transform, With<WorldCamera>>,
) {
    let mut dir = Vec2::ZERO;

    // Keyboard: WASD + arrow aliases.
    dir += keyboard_pan_dir(
        keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp),
        keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown),
        keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft),
        keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight),
    );

    // Mouse edge: only when the cursor is over the primary window.
    if let Some(window) = windows.iter().next()
        && let Some(cursor) = window.cursor_position()
    {
        dir += mouse_edge_dir(cursor, window.size(), EDGE_BAND_PX);
    }

    // Gamepad RIGHT stick (the first connected pad), past the deadzone.
    if let Some(gamepad) = gamepads.iter().next() {
        dir += stick_pan_dir(gamepad.right_stick(), STICK_DEADZONE);
    }

    let velocity = pan_velocity(dir, PAN_SPEED);
    if velocity == Vec2::ZERO {
        return;
    }
    let delta = velocity * time.delta_secs();
    for mut transform in &mut cameras {
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
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

    // -----------------------------------------------------------------------------
    // GTW-250 — pan-navigation pure helpers (AC1–AC4).
    // -----------------------------------------------------------------------------

    /// A test window size for the mouse-edge helper.
    const SIZE: Vec2 = Vec2::new(800.0, 600.0);
    /// A test mouse-edge band.
    const EDGE: EdgeBandPx = EdgeBandPx::new(20.0);

    /// AC1 — `mouse_edge_dir` maps each edge band to a camera direction WITH the
    /// screen-y → camera-y flip: top → `+Y`, bottom → `-Y`, left → `-X`, right → `+X`, a
    /// corner → a diagonal, and the centre → `ZERO`. Relations, not pinned magnitudes.
    #[test]
    fn mouse_edge_dir_maps_each_edge_with_the_y_flip() {
        // TOP band (small screen y) pans the camera UP (+Y) — the explicit flip.
        let top = mouse_edge_dir(Vec2::new(SIZE.x * 0.5, 5.0), SIZE, EDGE);
        assert!(
            top.y > 0.0,
            "cursor near the TOP must pan the camera UP (+Y)"
        );
        assert_eq!(
            top.x.to_bits(),
            0.0_f32.to_bits(),
            "a centred-x top has no x pan"
        );

        // BOTTOM band (large screen y) pans the camera DOWN (-Y).
        let bottom = mouse_edge_dir(Vec2::new(SIZE.x * 0.5, SIZE.y - 5.0), SIZE, EDGE);
        assert!(
            bottom.y < 0.0,
            "cursor near the BOTTOM must pan the camera DOWN (-Y)"
        );

        // LEFT band → -X.
        let left = mouse_edge_dir(Vec2::new(5.0, SIZE.y * 0.5), SIZE, EDGE);
        assert!(left.x < 0.0, "cursor near the LEFT must pan -X");
        assert_eq!(
            left.y.to_bits(),
            0.0_f32.to_bits(),
            "a centred-y left has no y pan"
        );

        // RIGHT band → +X.
        let right = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, SIZE.y * 0.5), SIZE, EDGE);
        assert!(right.x > 0.0, "cursor near the RIGHT must pan +X");

        // CORNER (top-right) → a diagonal: +X and +Y.
        let corner = mouse_edge_dir(Vec2::new(SIZE.x - 5.0, 5.0), SIZE, EDGE);
        assert!(
            corner.x > 0.0 && corner.y > 0.0,
            "the top-right corner must pan diagonally (+X, +Y)"
        );

        // CENTRE (no band) → ZERO.
        assert_eq!(
            mouse_edge_dir(SIZE * 0.5, SIZE, EDGE),
            Vec2::ZERO,
            "a cursor in the centre (no edge band) contributes no pan",
        );
    }

    /// AC2 — `keyboard_pan_dir` maps W/A/S/D to camera axes, opposite keys cancel, and
    /// (via the system) arrows alias WASD. W → `+Y`, S → `-Y`, A → `-X`, D → `+X`,
    /// W+D → up-right, W+S → `ZERO`.
    #[test]
    fn keyboard_pan_dir_combines_keys_and_cancels_opposites() {
        // Single keys (W / S / A / D).
        assert_eq!(
            keyboard_pan_dir(true, false, false, false),
            Vec2::new(0.0, 1.0),
            "W → +Y",
        );
        assert_eq!(
            keyboard_pan_dir(false, true, false, false),
            Vec2::new(0.0, -1.0),
            "S → -Y",
        );
        assert_eq!(
            keyboard_pan_dir(false, false, true, false),
            Vec2::new(-1.0, 0.0),
            "A → -X",
        );
        assert_eq!(
            keyboard_pan_dir(false, false, false, true),
            Vec2::new(1.0, 0.0),
            "D → +X",
        );

        // Combination: W+D → up-right.
        assert_eq!(
            keyboard_pan_dir(true, false, false, true),
            Vec2::new(1.0, 1.0),
            "W+D → up-right (+X, +Y)",
        );

        // Opposite keys cancel.
        assert_eq!(
            keyboard_pan_dir(true, true, false, false),
            Vec2::ZERO,
            "W+S cancel → ZERO",
        );
        assert_eq!(
            keyboard_pan_dir(false, false, true, true),
            Vec2::ZERO,
            "A+D cancel → ZERO",
        );
    }

    /// AC3 — `stick_pan_dir` zeroes a sub-deadzone stick and passes a clearly-past stick
    /// through with the camera-y orientation preserved (stick up → +Y).
    #[test]
    fn stick_pan_dir_respects_the_deadzone() {
        let deadzone = StickDeadzone::new(0.15);

        // A tiny resting drift below the deadzone → ZERO.
        assert_eq!(
            stick_pan_dir(Vec2::new(0.05, -0.05), deadzone),
            Vec2::ZERO,
            "a sub-deadzone stick contributes no pan",
        );

        // A clear push past the deadzone passes through, magnitude preserved, stick-up = +Y.
        let pushed = Vec2::new(0.0, 0.8);
        assert_eq!(
            stick_pan_dir(pushed, deadzone),
            pushed,
            "a clearly-past stick passes through unchanged (stick up → camera +Y)",
        );
        assert!(
            stick_pan_dir(Vec2::new(0.0, 0.8), deadzone).y > 0.0,
            "stick UP must map to camera +Y",
        );
    }

    /// AC4 — `pan_velocity` scales a unit direction by speed, returns ZERO for no input,
    /// and the diagonal-not-faster rule holds for the keyboard combination (a normalised
    /// diagonal is no faster than a cardinal).
    #[test]
    fn pan_velocity_scales_and_diagonal_is_not_faster() {
        let speed = PanSpeed::new(100.0);

        // ZERO direction → ZERO velocity.
        assert_eq!(
            pan_velocity(Vec2::ZERO, speed),
            Vec2::ZERO,
            "no input → no velocity (no drift)",
        );

        // A unit cardinal → speed-scaled.
        let cardinal = pan_velocity(Vec2::new(0.0, 1.0), speed);
        assert_eq!(
            cardinal,
            Vec2::new(0.0, 100.0),
            "a unit direction scales to exactly `speed` world-units/sec",
        );

        // Diagonal-not-faster: the (normalised) diagonal speed equals the cardinal speed.
        let diagonal = pan_velocity(Vec2::new(1.0, 1.0), speed);
        let diag_speed = diagonal.length();
        let card_speed = cardinal.length();
        assert!(
            (diag_speed - card_speed).abs() < 1e-3,
            "a diagonal keyboard combo must not be faster than a cardinal (got diag {diag_speed}, \
             cardinal {card_speed})",
        );

        // A sub-unit analog magnitude scales speed DOWN (a gentle stick push pans gently).
        let gentle = pan_velocity(Vec2::new(0.0, 0.5), speed);
        assert!(
            gentle.length() < card_speed,
            "a sub-unit analog stick magnitude keeps its sub-unit scale (pans slower)",
        );
    }
}
