
pub mod act_bus;

pub mod pointer;


mod plugin;


pub use act_bus::contextual;
pub use act_bus::cycle;
pub use act_bus::cycle::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};
pub use act_bus::focus_bridge;
pub use act_bus::intent;
pub use act_bus::keybinds;
pub use act_bus::keyboard;
pub use act_bus::sets;
pub use act_bus::{
    contextual::{
        ContextualAct, ContextualActAppExt, ContextualActSystems, EnterEmplacementAct, ExecuteAct,
        ExitEmplacementAct, MeleeAct, OpenDoorAct, PendingContextualIntents, ShoveAct, SlotRank,
        StabilizeAct, ThrowGrenadeAct, drain_contextual_intents,
    },
    focus_bridge::{PanelNavOrder, focused_panel_button},
    intent::{
        ActIntent, ActWriters, LevelStep, PendingActIntent, SelectionCycleReads,
        dispatch_act_intents, step_level,
    },
    keybinds::{BoundKey, Keybinds},
    keyboard::{cycle_selection_keys, full_view_key, level_keys, posture_keys, select_clear_key},
    sets::InputSystems,
};
pub use plugin::{GdtfBattleInputActive, GdtfBattleInputPlugin};
pub use pointer::fire_mode;
pub use pointer::fire_surface;
pub use pointer::gamepad;
pub use pointer::picking;
pub use pointer::selection;
pub use pointer::{
    fire_mode::{SelectedFireMode, sync_fire_mode_on_select},
    gamepad::{
        ActivePointer, CURSOR_SPEED, CURSOR_STICK_DEADZONE, CursorSpeed, CursorStickDeadzone,
        GamepadCursor, emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn,
        mouse_reclaims_pointer, move_cursor, move_gamepad_cursor,
    },
    picking::{
        InspectMode, InspectTarget, emit_highlight_request, pick_hovered_cell, world_to_cell,
    },
    selection::{
        CellOrderKey, CycleDirection, FireTargetReads, LeftClickOutcome, LeftClickReads,
        PathPreviewTarget, PinOutcome, PreviewGrids, SelectedShooter, SelectionHighlight,
        TurnReads, apply_left_click, apply_pin, auto_select_first_player_ganger, cell_order_key,
        cycle_player_selection, decide_left_click, decide_pin, decide_turn, left_click_act,
        populate_fire_target, populate_path_preview, reset_move_target_on_fire_mode_change,
        right_click_turn_to_face, update_selection_highlight,
    },
};
