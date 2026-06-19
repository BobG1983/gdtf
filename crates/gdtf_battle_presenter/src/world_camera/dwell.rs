//! GTW-299 DWELL gate: the per-cursor-linger accumulator + the pure dwell decision the
//! edge-pan systems read so a brief graze near a map edge does NOT pan.
//!
//! Edge-pan no longer fires the instant the cursor crosses into the edge band — the cursor must
//! REST there continuously for at least [`DwellDelaySeconds`](super::tuning::DwellDelaySeconds)
//! first (the user's `0.3` s default, live-tunable through `assets/tiles/pan_tuning.ron`). So a
//! click-release near the border, or a cursor that brushes the edge while travelling elsewhere,
//! never yanks the camera; only a deliberate edge-rest pans.
//!
//! The linger is tracked in a presenter-owned [`PanEdgeDwellState`] resource (VIEW state — a
//! [`Resource`], NOT a `Local`, so it is testable): the mouse-cursor edge and the gamepad-cursor
//! edge each carry their OWN accumulator ([`DwellElapsed`]) so the two sources never interfere.
//! Each accumulator is grown by `Res<Time>` delta the frame its cursor is in-band and RESET to
//! zero the frame it is not, and the edge contribution is applied only once the accumulator
//! reaches the dwell threshold ([`should_edge_pan_after_dwell`]).

use bevy::prelude::*;

/// How long ONE cursor has rested continuously inside the edge band, in SECONDS.
///
/// GTW-299: the live per-source dwell accumulator — grown by `Res<Time>` delta each frame the
/// cursor sits in the edge band and reset to zero the frame it leaves (or on a fresh
/// click-release that moves the cursor out of band). Once it reaches
/// [`DwellDelaySeconds`](super::tuning::DwellDelaySeconds) the edge contributes its pan
/// ([`should_edge_pan_after_dwell`]); below it, the edge is ignored.
///
/// A named newtype over the `f32` seconds (`.claude/rules/no-bare-types.md`): the inner is
/// PRIVATE, read through [`Deref`](std::ops::Deref) and changed only through
/// [`accumulate`](Self::accumulate) / [`reset`](Self::reset) — accumulated time has invariants
/// (never negative, only ever grows-by-delta or snaps to zero), so it is mutated through named
/// methods rather than a leaky `DerefMut`.
#[derive(Debug, Clone, Copy, PartialEq, Deref)]
pub struct DwellElapsed(f32);

impl DwellElapsed {
    /// A fresh accumulator with zero elapsed linger (the cursor has not yet rested in-band).
    pub const ZERO: Self = Self(0.0);

    /// Add this frame's `delta` seconds of continued in-band linger.
    ///
    /// Called the frame the cursor is inside the edge band; the accumulated total is what
    /// [`should_edge_pan_after_dwell`] compares against the dwell threshold.
    pub fn accumulate(&mut self, delta: f32) {
        self.0 += delta;
    }

    /// Snap the accumulator back to zero — the cursor left the edge band (or no cursor), so the
    /// linger restarts from scratch the next time it returns.
    pub const fn reset(&mut self) {
        self.0 = 0.0;
    }
}

impl Default for DwellElapsed {
    fn default() -> Self {
        Self::ZERO
    }
}

/// The presenter-owned edge-pan dwell state: one linger accumulator per edge-pan SOURCE.
///
/// GTW-299: VIEW state (a [`Resource`], not a `Local`, so the dwell logic is testable through the
/// real systems). The mouse-cursor edge ([`pan_camera`](super::pan::pan_camera)) and the
/// gamepad-cursor edge ([`pan_camera_on_gamepad_cursor_edge`](super::pan::pan_camera_on_gamepad_cursor_edge))
/// hold SEPARATE accumulators so dwelling at the edge with one pointer never advances (or resets)
/// the other's gate — the two sources are independent.
///
/// Inserted with [`Default`] (both accumulators zero) wherever the pan systems are wired, so a
/// headless app with no `AssetServer` still has the resource present and behaves correctly
/// (`bevy-traps.md` #1). Derives [`Default`] for that headless / fresh-battle insert.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct PanEdgeDwellState {
    /// The mouse-cursor edge-band linger (driven by [`pan_camera`](super::pan::pan_camera)).
    pub mouse:   DwellElapsed,
    /// The gamepad-cursor edge-band linger (driven by
    /// [`pan_camera_on_gamepad_cursor_edge`](super::pan::pan_camera_on_gamepad_cursor_edge)).
    pub gamepad: DwellElapsed,
}

/// Whether the edge has been dwelt on long enough to start panning: `accumulated >= threshold`.
///
/// GTW-299 AC1/AC6: the PURE dwell decision (no `App`, no `World` — unit-tested directly,
/// mirroring the `pan.rs` pure-helper style). A cursor that has lingered in the edge band for at
/// least the dwell delay pans; one that has lingered less (a graze, a click-release) does not. The
/// comparison is inclusive at the threshold (linger exactly equal to the delay pans), so the gate
/// opens promptly once the deliberate rest reaches the configured time.
///
/// Both arguments are SECONDS read through [`Deref`](std::ops::Deref): the live
/// [`DwellElapsed`] accumulator and the
/// [`DwellDelaySeconds`](super::tuning::DwellDelaySeconds) threshold.
#[must_use]
pub fn should_edge_pan_after_dwell(
    accumulated: DwellElapsed,
    threshold: super::tuning::DwellDelaySeconds,
) -> bool {
    *accumulated >= *threshold
}
