//! GTW-249 / GTW-271: headless camera-LOGIC tests for the battle-start frame-on-units +
//! the bounds clamp (incl. the GTW-271 viewport-aware fallback half-extent).
//!
//! - AC3 (`frame_camera_on_units`): with a `WorldCamera` + >=2 player-faction gangers at
//!   known cells (+ `PlayerFaction` + `BattleInProgress`), one update centres the camera
//!   on the player gangers' world centroid; several MORE updates do NOT re-centre (the
//!   `Local<bool>` latch holds) — proving the one-shot framing won't fight the sibling
//!   pan-nav slice (GTW-250).
//! - AC4 (`clamp_camera_to_bounds`): a camera placed FAR outside the battlefield bounds,
//!   with a known orthographic half-viewport + primary window, is pulled back inside (the
//!   `clamp_camera` relation holds end-to-end through the real system).
//! - GTW-271 AC6 (`clamp_camera_to_bounds` + viewport-aware fallback): a camera with a
//!   DEGENERATE orthographic `area` (forcing the pre-`camera_system` fallback) + an explicit
//!   sub-rect `Camera.viewport` is clamped using the SMALLER viewport-derived half-extent
//!   (the viewport physical size / window scale factor), NOT the full window — so reverting
//!   the `Some(viewport)` fallback branch to `window.size()` flips the asserted result.
//!
//! These prove the camera LOGIC headless; the actual on-screen centring is the user's
//! eyeball (AC5, post-gate QA). The systems are exercised on their REAL registration
//! shape (battle-gated `Update`), not a copy. Every `app.world_mut()` / camera mutation is
//! in a TEST BODY — the accepted headless idiom (`bevy-traps.md` #7 carve-out (a)). No
//! function here takes `&mut World`/`&World`.

mod clamp;
mod frame_on_units;
mod harness;
mod viewport_fallback;
