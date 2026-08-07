//! Battle input: keyboard, pointer, gamepad, and act intent bus.

/// Act intent bus, keybinds, keyboard, and contextual acts.
pub mod act_bus;

/// Pointer, selection, fire mode, and gamepad cursor.
pub mod pointer;

mod plugin;

pub use act_bus::{
    contextual,
    contextual::{
        ContextualAct, ContextualActAppExt, ContextualActSystems, EnterEmplacementAct, ExecuteAct,
        ExitEmplacementAct, MeleeAct, OpenDoorAct, PendingContextualIntents, ShoveAct, SlotRank,
        StabilizeAct, ThrowGrenadeAct, drain_contextual_intents,
    },
    cycle,
    cycle::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance},
    focus_bridge,
    focus_bridge::{PanelNavOrder, focused_panel_button},
    intent,
    intent::{
        ActIntent, ActWriters, LevelStep, PendingActIntent, SelectionCycleReads, ShownLevel,
        dispatch_act_intents, step_level,
    },
    keybinds,
    keybinds::{BoundKey, Keybinds},
    keyboard,
    keyboard::{cycle_selection_keys, full_view_key, level_keys, posture_keys, select_clear_key},
    sets,
    sets::InputSystems,
};
pub use plugin::{GdtfBattleInputActive, GdtfBattleInputPlugin, battle_act_gate};
pub use pointer::{
    fire_mode,
    fire_mode::{SelectedFireMode, mode_spec_for, ranged_weapon_of, sync_fire_mode_on_select},
    fire_surface,
    fire_surface::{ShooterArms, try_fire_request},
    gamepad,
    gamepad::{
        ActivePointer, CURSOR_SPEED, CURSOR_STICK_DEADZONE, CursorSpeed, CursorStickDeadzone,
        GamepadCursor, emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn,
        mouse_reclaims_pointer, move_cursor, move_gamepad_cursor,
    },
    picking,
    picking::{
        InspectMode, InspectTarget, emit_highlight_request, pick_hovered_cell, world_to_cell,
    },
    selection,
    selection::{
        CellOrderKey, CycleDirection, FireTargetReads, LeftClickOutcome, LeftClickReads,
        PathPreviewTarget, PinOutcome, PointerSelection, PreviewGrids, SelectedShooter,
        SelectionHighlight, TurnReads, apply_left_click, apply_pin,
        auto_select_first_player_ganger, cell_order_key, cycle_player_selection, decide_left_click,
        decide_pin, decide_turn, left_click_act, populate_fire_target, populate_path_preview,
        reset_move_target_on_fire_mode_change, right_click_turn_to_face,
        update_selection_highlight,
    },
};
