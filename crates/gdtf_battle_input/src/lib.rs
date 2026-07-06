//! Battle input layer for GDTF — the HEAD of the `input -> presenter -> sim` chain.
//!
//! This crate gives the landed top-down SPRITE battle (the read-only presenter, S2-S6) its
//! cursor awareness and control surfaces. Every update during a live battle it reads the
//! presenter's [`WorldCamera`](gdtf_battle_presenter::WorldCamera) and the OS cursor, unprojects
//! the cursor into a sim [`Cell`](gdtf_battle_sim::metric::Cell) on the presenter's
//! [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) (the INVERSE of `cell_to_world`), stores it
//! in [`InspectTarget`]'s live hovered cell, and EMITS a presenter-owned `HighlightRequest` for the presenter to draw
//! (GTW-251). The selection / act surfaces (S8) ride on top of that.
//!
//! # The one-way dependency chain (ADR-0001)
//!
//! The dependency edge runs strictly `gdtf_battle_input -> gdtf_battle_presenter ->
//! gdtf_battle_sim` — a CHAIN, never a cycle. This crate reads the presenter's camera/px/level
//! interface and the sim's presentation-agnostic metric ([`Cell`](gdtf_battle_sim::metric::Cell) /
//! [`Level`](gdtf_battle_sim::metric::Level) / [`CellLevel`](gdtf_battle_sim::metric::CellLevel)); the presenter
//! reads only the sim; the sim reads NEITHER. Input speaks cursor + [`Cell`](gdtf_battle_sim::metric::Cell),
//! not pixels — the px boundary lives in the presenter, and the world->cell inverse reuses it.
//!
//! # Module layout (GTW-201 / GTW-385)
//!
//! - [`plugin`] — the [`GdtfBattleInputPlugin`] wiring (the `add_systems` ordering) + its marker.
//! - [`act_bus`] — the act-intent data bus and the key/binding surfaces that feed it:
//!   [`sets`], [`intent`], [`contextual`] (the GTW-571 generic contextual-act seam),
//!   [`keyboard`], [`keybinds`], [`cycle`].
//! - [`mod@pointer`] — the cursor->cell->selection control surface and the fire decision pair:
//!   [`picking`], [`selection`], [`gamepad`], [`fire_mode`], [`fire_surface`].

// ---- concern parents (GTW-385) --------------------------------------------------

/// The act-intent data bus and the key/binding surfaces that feed it.
pub mod act_bus;

/// The cursor->cell->selection control surface and the fire decision pair.
pub mod pointer;

// ---- kept at root ---------------------------------------------------------------

mod plugin;

// ---- crate-root module aliases so existing `crate::<child>::` paths keep working -
//
// plugin/build.rs and test files use `crate::fire_mode::...`, `crate::gamepad::...`,
// `crate::selection::...`, etc. as sub-module paths.  These re-exports preserve every
// `crate::<child>::...` reference without touching the moved source files.

/// Re-export of [`act_bus::contextual`] — the GTW-571 generic contextual-act seam —
/// for `crate::contextual::...` paths.
pub use act_bus::contextual;
/// Re-export of [`act_bus::cycle`] for intra-crate `crate::cycle::...` paths.
pub use act_bus::cycle;
// ---- flat item re-exports (unchanged public API surface) ------------------------
pub use act_bus::cycle::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};
/// Re-export of [`act_bus::intent`] for intra-crate `crate::intent::...` paths.
pub use act_bus::intent;
/// Re-export of [`act_bus::keybinds`] for intra-crate `crate::keybinds::...` paths.
pub use act_bus::keybinds;
/// Re-export of [`act_bus::keyboard`] for intra-crate `crate::keyboard::...` paths.
pub use act_bus::keyboard;
/// Re-export of [`act_bus::sets`] for intra-crate `crate::sets::...` paths.
pub use act_bus::sets;
pub use act_bus::{
    contextual::{
        ContextualAct, ContextualActAppExt, ContextualActSystems, EnterEmplacementAct, ExecuteAct,
        ExitEmplacementAct, MeleeAct, OpenDoorAct, PendingContextualIntents, ShoveAct,
        StabilizeAct, ThrowGrenadeAct, drain_contextual_intents,
    },
    intent::{
        ActIntent, ActWriters, LevelStep, PendingActIntent, SelectionCycleReads,
        dispatch_act_intents, step_level,
    },
    keybinds::{BoundKey, Keybinds},
    keyboard::{cycle_selection_keys, full_view_key, level_keys, posture_keys, select_clear_key},
    sets::InputSystems,
};
pub use plugin::{GdtfBattleInputActive, GdtfBattleInputPlugin};
/// Re-export of [`pointer::fire_mode`] for intra-crate `crate::fire_mode::...` paths.
pub use pointer::fire_mode;
/// Re-export of [`pointer::fire_surface`] for intra-crate `crate::fire_surface::...` paths.
pub use pointer::fire_surface;
/// Re-export of [`pointer::gamepad`] for intra-crate `crate::gamepad::...` paths.
pub use pointer::gamepad;
/// Re-export of [`pointer::picking`] for intra-crate `crate::picking::...` paths.
pub use pointer::picking;
/// Re-export of [`pointer::selection`] for intra-crate `crate::selection::...` paths.
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
