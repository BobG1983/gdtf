//! DEV-ONLY loading-screen self-screenshot QA hook (GTW-419; ported onto the
//! `gdtf_screenshot` primitives in GTW-577 C4).
//!
//! This is **not shipping behavior**. It exists so QA (or a coding agent) can drive the app and
//! capture the rendered LOADING SCREEN frame — proving AC2 (no partial-level frame) visually,
//! which the headless tests structurally cannot observe. It mirrors the GTW-297 capture
//! discipline (env-var gated, double-gated on a debug cfg) but fires DURING
//! [`BattleScapeState::Generation`](crate::states::BattleScapeState) rather than `BattleRunning`.
//!
//! ## Two gates, both must hold to activate
//!
//! 1. **Dev cfg.** Wired into [`LoadingScreenPlugin`](super::plugin::LoadingScreenPlugin) only
//!    under `cfg!(all(debug_assertions, feature = "net_qa"))` (GTW-749 retired the
//!    `dev_capture` feature this used to ride); a release / default build never compiles it.
//! 2. **Opt-in env var.** Even when compiled in it is inert until `GDTF_LOADING_SHOT=/abs/out.png`
//!    is set: with it unset the hook registers nothing.
//!
//! ## Holding the (brief) Generation state — capture-and-CONTINUE, no exit
//!
//! Generation auto-advances to `AnimateIn` the moment the sim signals `BattleReady` (a frame or
//! two). So that the screenshot reliably catches the LOADING SCREEN — not the assembled level or
//! a partial frame — the hook's per-scene DRIVE ([`pin_generation_until_shot`], kept bespoke per
//! GTW-577 P9) re-pins [`NextState`] back to `Generation` every frame until the shared
//! [`settle_then_capture`] has REQUESTED the shot (the settle count interleaves with the pin:
//! the pin runs chained ahead, the settle ticks behind it each Generation frame). The pin runs
//! in `Update`, so it deterministically overrides the `BattleReady`-gated `move_on` (which runs
//! in `FixedUpdate`, earlier in the frame). Once the shot is requested the pin RELEASES (stops
//! re-pinning), so the normal `Generation → AnimateIn` flow resumes on the next ready frame —
//! and this scene deliberately gains NO exit: the run CONTINUES into the battle, exactly the
//! documented GTW-419 contract. The pin is a dev-only QA affordance: it never runs in a normal
//! build.

use bevy::prelude::*;
use gdtf_screenshot::{
    CapturePath, CaptureProgress, SettleFrames, parse_shot_path, settle_then_capture,
};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::generation::loading_screen::components::LoadingScreenRoot,
};

/// The `GDTF_LOADING_SHOT` env var: the absolute path of the output PNG. Setting it (in a
/// `net_qa` debug build) opts into the loading-screen capture hook.
const LOADING_SHOT_ENV: &str = "GDTF_LOADING_SHOT";

/// This scene's calibrated settle window: 4 `Generation` frames, so the UI layout has flushed
/// and the loading screen is settled rather than mid-layout (the GTW-297 settle precedent; the
/// per-scene magnitude stays bespoke — GTW-577 P9).
const LOADING_SETTLE: SettleFrames = SettleFrames::new(4);

/// Whether the loading-screen capture hook is enabled, and where it writes.
///
/// `Some(path)` when [`LOADING_SHOT_ENV`] is set to a non-empty (trimmed) value; `None` (the hook
/// stays inert) otherwise. Pure aside from the env read: the trim/empty gate is the shared
/// [`parse_shot_path`] (GTW-577 C4 — the ONE path gate, asserted once in `gdtf_screenshot`).
#[must_use]
pub(in crate::states::running::game::battlescape::generation) fn loading_shot_path()
-> Option<CapturePath> {
    parse_shot_path(std::env::var(LOADING_SHOT_ENV).ok().as_deref())
}

/// PINS the `Generation` state each frame until the screenshot has been REQUESTED, so the shot
/// catches the LOADING SCREEN (not the assembled level) — the kept GTW-419 hold, expressed
/// against the shared [`CaptureProgress`] instead of a local counter (GTW-577 C4).
///
/// While the shared [`settle_then_capture`] (chained after this) is still settling,
/// [`CaptureProgress::is_requested`] is `false` and this re-pins [`NextState`] back to
/// `Generation`, inhibiting the `BattleReady`-gated `move_on` (which wrote its `AnimateIn`
/// earlier in the frame, in `FixedUpdate` — this later `Update` write deterministically wins).
/// On the frame AFTER the shot is requested it stops re-pinning — the pin is RELEASED and the
/// next ready frame advances `Generation → AnimateIn` normally; the run continues into the
/// battle (capture-and-CONTINUE, no exit). Param-only (`bevy-traps.md` #7).
fn pin_generation_until_shot(
    progress: Res<CaptureProgress>,
    mut next: ResMut<NextState<BattleScapeState>>,
) {
    if progress.is_requested() {
        // Released after the shot: never re-pin again, so the normal flow resumes.
        return;
    }
    next.set(BattleScapeState::Generation);
}

/// Register the loading-screen capture hook IF its env-var gate is set.
///
/// Called by [`LoadingScreenPlugin`](super::plugin::LoadingScreenPlugin) only under
/// `cfg!(all(debug_assertions, feature = "net_qa"))`. When [`loading_shot_path`] returns
/// `None` it registers nothing (the hook is fully inert, exactly like a build without it).
///
/// The per-scene DRIVE ([`pin_generation_until_shot`]) is chained AHEAD of the shared
/// [`settle_then_capture`] (GTW-577 C4): pin, then settle-tick, every Generation frame. The
/// capture is additionally gated on the [`LoadingScreenRoot`] being present, so it never
/// captures a blank frame — and retries until the screen exists instead of silently skipping
/// (the GTW-577 retirement of the old `!= SETTLE_FRAMES` exact-match skip). There is no
/// capture-exit here — this scene's documented contract is capture-and-CONTINUE (the pin
/// releases and the app proceeds `Generation → AnimateIn` into the battle). NOTE: the shared
/// [`CapturePath`] / [`SettleFrames`] resources mean ONE scene-capture env var per run (the QA
/// workflow's existing shape).
pub(in crate::states::running::game::battlescape::generation) fn register_loading_capture(
    app: &mut App,
) {
    let Some(path) = loading_shot_path() else {
        return;
    };
    info!("loading-screen capture: ON (dev) -> {}", path.display());
    app.insert_resource(path)
        .insert_resource(LOADING_SETTLE)
        .init_resource::<CaptureProgress>()
        .add_systems(
            Update,
            (
                pin_generation_until_shot,
                settle_then_capture.run_if(any_with_component::<LoadingScreenRoot>),
            )
                .chain()
                .run_if(
                    in_state(BattleScapeState::Generation).and_then(resource_exists::<CapturePath>),
                ),
        );
}

#[cfg(test)]
mod tests {
    use super::LOADING_SHOT_ENV;

    /// The thin per-scene pin (GTW-577 C6): the scene KEEPS its own env var — the QA-facing
    /// activation contract — while the trim/empty gate logic is asserted ONCE in
    /// `gdtf_screenshot` (`parse_shot_path`'s own tests), which `loading_shot_path` delegates
    /// to.
    #[test]
    fn env_var_name_is_the_scene_contract() {
        assert_eq!(LOADING_SHOT_ENV, "GDTF_LOADING_SHOT");
    }
}
