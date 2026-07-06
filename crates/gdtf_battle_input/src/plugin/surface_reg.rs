//! Act-SURFACE registrations: the GTW-571 contextual-act registrar lines + the GTW-259
//! gamepad cursor/act surfaces.

use bevy::{ecs::message::Messages, prelude::*, window::CursorMoved};
use gdtf_battle_sim::prelude::BattleInProgress;

use super::build::battle_act_gate;
use crate::{
    InputSystems,
    contextual::{
        ContextualActAppExt, EnterEmplacementAct, ExecuteAct, ExitEmplacementAct, MeleeAct,
        OpenDoorAct, ShoveAct, StabilizeAct, ThrowGrenadeAct, configure_contextual_act_drains,
    },
    gamepad::{
        emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn, mouse_reclaims_pointer,
        move_gamepad_cursor,
    },
    intent::dispatch_act_intents,
    picking::pick_hovered_cell,
};

/// Registers the GTW-571 CONTEXTUAL acts: the ONE explicitly-ordered
/// [`ContextualActSystems::Drain`](crate::contextual::ContextualActSystems) set
/// (configured ONCE — inside [`InputSystems::Gather`] and `.before(dispatch_act_intents)`,
/// the Q5 invariant's explicit ordering) plus one compile-time
/// [`add_contextual_act::<A>()`](ContextualActAppExt::add_contextual_act) line per act.
///
/// Each line wires the act's whole input-layer slice — the `*Requested` buffer (IDEMPOTENT
/// with the sim's own registration, `bevy-traps.md` #4), the per-act pending queue, and the
/// per-act generic drain. Adding a contextual act adds exactly ONE line here (plus its
/// descriptor module — see `docs/authoring/contextual-act-recipe.md`). Extracted from
/// [`GdtfBattleInputPlugin::build`](super::GdtfBattleInputPlugin) to keep `build` under the
/// `too_many_lines` lint (the `register_gamepad_systems` precedent).
pub(super) fn register_contextual_acts(app: &mut App) {
    configure_contextual_act_drains(app);
    app.add_contextual_act::<ExecuteAct>()
        .add_contextual_act::<StabilizeAct>()
        .add_contextual_act::<MeleeAct>()
        .add_contextual_act::<ShoveAct>()
        .add_contextual_act::<OpenDoorAct>()
        .add_contextual_act::<EnterEmplacementAct>()
        .add_contextual_act::<ExitEmplacementAct>()
        .add_contextual_act::<ThrowGrenadeAct>();
}

/// Registers the GTW-259 gamepad systems into [`InputSystems::Gather`]: the software-cursor
/// drive + the pointer arbitration, the South / East act surfaces, and the edge-pan emitter.
///
/// Extracted from [`GdtfBattleInputPlugin::build`](super::GdtfBattleInputPlugin) to keep it under the
/// `too_many_lines` lint (the presenter's `register_*` extraction precedent). Every system is
/// battle-gated (`bevy-traps.md` #1) and `InputSystems::Gather`-banded:
///
/// - [`move_gamepad_cursor`] steers the [`GamepadCursor`](crate::gamepad::GamepadCursor) by the LEFT stick and claims
///   [`ActivePointer::Gamepad`](crate::gamepad::ActivePointer) past the deadzone; ordered `.before(pick_hovered_cell)`
///   (`bevy-traps.md` #3) so the generalized picker projects THIS update's cursor.
/// - [`mouse_reclaims_pointer`] flips back to [`ActivePointer::Mouse`](crate::gamepad::ActivePointer) on a [`CursorMoved`]
///   message (last-moved-wins); additionally gated on its `Messages<CursorMoved>` buffer so
///   its [`MessageReader`](bevy::ecs::message::MessageReader) validates under `MinimalPlugins`.
/// - [`gamepad_click_act`] (South) + [`gamepad_turn`] (East) reuse the SHARED decision the
///   mouse uses and the SAME [`PendingActIntent`](crate::PendingActIntent) seam, ordered `.before(pick_hovered_cell)`
///   and `.before(dispatch_act_intents)`.
/// - [`emit_gamepad_cursor_move`] writes [`GamepadCursorMoved`](gdtf_battle_presenter::GamepadCursorMoved) for the presenter's edge-pan
///   when the gamepad is the active pointer.
pub(super) fn register_gamepad_systems(app: &mut App) {
    app.add_systems(
        Update,
        move_gamepad_cursor
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .run_if(resource_exists::<BattleInProgress>),
    )
    .add_systems(
        Update,
        mouse_reclaims_pointer.in_set(InputSystems::Gather).run_if(
            resource_exists::<BattleInProgress>.and_then(resource_exists::<Messages<CursorMoved>>),
        ),
    )
    .add_systems(
        Update,
        (gamepad_click_act, gamepad_turn)
            .in_set(InputSystems::Gather)
            .before(pick_hovered_cell)
            .before(dispatch_act_intents)
            .run_if(battle_act_gate()),
    )
    .add_systems(
        Update,
        emit_gamepad_cursor_move
            .in_set(InputSystems::Gather)
            .run_if(resource_exists::<BattleInProgress>),
    );
}
