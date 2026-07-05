//! Camera registration: battle-start framing, mouse/keyboard/gamepad pan, and the
//! bounds clamp.

use bevy::prelude::*;
use gdtf_battle_sim::{BattleInProgress, PlayerFaction};

use crate::{
    GamepadCursorMoved, PanEdgeDwellState, clamp_camera_to_bounds, frame_camera_on_units,
    pan_camera, pan_camera_on_gamepad_cursor_edge,
};

/// Registers the GTW-249 camera-positioning systems plus the GTW-250 pan navigation and the
/// GTW-259 gamepad-cursor edge-pan: the one-shot [`frame_camera_on_units`] (centre the
/// [`WorldCamera`](crate::WorldCamera) on the player gangers at battle start), the every-frame
/// [`pan_camera`] (move the camera under mouse-edge / keyboard / gamepad-right-stick
/// navigation), the every-frame [`pan_camera_on_gamepad_cursor_edge`] (pan when the GTW-259
/// gamepad software cursor reaches a screen edge), and the every-frame [`clamp_camera_to_bounds`]
/// (keep the viewport inside the battlefield extent).
///
/// All are battle-scoped (`bevy-traps.md` #1): gated
/// `run_if(resource_exists::<BattleInProgress>` AND `resource_exists::<PlayerFaction>)` —
/// `PlayerFaction` is the sim's player-gang witness the framing reads, inserted/removed on
/// the same `BattleInProgress` window, so none run (and none panic on a missing `Res`)
/// outside a live battle. The SAME gate keeps the pans and the clamp in the same scheduled
/// band, so the clamp stays the last writer.
///
/// The clamp is ordered `.after` the framing and BOTH pans
/// (`pan_camera.before(clamp_camera_to_bounds)`,
/// `pan_camera_on_gamepad_cursor_edge.before(clamp_camera_to_bounds)`), so it is the LAST
/// writer of the camera position each frame: whatever a pan adds to the translation, the
/// clamp pulls back inside the battlefield bounds — the camera can never be panned off the
/// map (GTW-250 / GTW-259). The pans are view-only: they move the presenter-owned camera
/// `Transform` and emit NO sim message. They run in plain `Update` (camera positioning needs
/// no `PresenterSystems::Draw` membership — it touches no atlas / sprite, only the camera
/// `Transform`).
///
/// The GTW-259 [`GamepadCursorMoved`] message buffer is registered here via
/// [`App::add_message`]: a [`MessageReader<GamepadCursorMoved>`](bevy::ecs::message::MessageReader)
/// panics param validation without its `Messages<GamepadCursorMoved>` buffer
/// (`bevy-traps.md` #4), and `add_message` is IDEMPOTENT — the input crate also registers the
/// same buffer so its `MessageWriter` validates headlessly, and the two coexist (the
/// [`HighlightRequest`](crate::HighlightRequest) precedent: the presenter DEFINES the message;
/// input WRITES it, input→presenter, no cycle).
///
/// The GTW-299 edge-pan DWELL accumulator ([`PanEdgeDwellState`]) is initialised here with
/// [`App::init_resource`] so it is always present for the (battle-gated) pan systems to read — it
/// is lightweight VIEW state with a [`Default`] (both accumulators zero), so a headless app gets
/// it for free and the dwell gate exercises the real path rather than the `Option`-absent
/// fallback. The pan systems still take it as `Option<ResMut<…>>` so they never panic if it is
/// somehow absent (`bevy-traps.md` #1).
pub(super) fn register_camera_framing_systems(app: &mut App) {
    let battle_gate =
        resource_exists::<BattleInProgress>.and_then(resource_exists::<PlayerFaction>);
    app.init_resource::<PanEdgeDwellState>();
    app.add_message::<GamepadCursorMoved>().add_systems(
        Update,
        (
            frame_camera_on_units,
            pan_camera,
            pan_camera_on_gamepad_cursor_edge,
            clamp_camera_to_bounds
                .after(frame_camera_on_units)
                .after(pan_camera)
                .after(pan_camera_on_gamepad_cursor_edge),
        )
            .run_if(battle_gate),
    );
}
