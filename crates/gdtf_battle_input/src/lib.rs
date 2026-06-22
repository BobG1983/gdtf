//! Battle input layer for GDTF — the HEAD of the `input -> presenter -> sim` chain.
//!
//! This crate gives the landed top-down SPRITE battle (the read-only presenter, S2-S6) its
//! cursor awareness and control surfaces. Every update during a live battle it reads the
//! presenter's [`WorldCamera`](gdtf_battle_presenter::WorldCamera) and the OS cursor, unprojects
//! the cursor into a sim [`Cell`](gdtf_battle_sim::Cell) on the presenter's
//! [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) (the INVERSE of `cell_to_world`), stores it
//! in [`InspectTarget`]'s live hovered cell, and EMITS a presenter-owned `HighlightRequest` for the presenter to draw
//! (GTW-251). The selection / act surfaces (S8) ride on top of that.
//!
//! # The one-way dependency chain (ADR-0001)
//!
//! The dependency edge runs strictly `gdtf_battle_input -> gdtf_battle_presenter ->
//! gdtf_battle_sim` — a CHAIN, never a cycle. This crate reads the presenter's camera/px/level
//! interface and the sim's presentation-agnostic metric ([`Cell`](gdtf_battle_sim::Cell) /
//! [`Level`](gdtf_battle_sim::Level) / [`CellLevel`](gdtf_battle_sim::CellLevel)); the presenter
//! reads only the sim; the sim reads NEITHER. Input speaks cursor + [`Cell`](gdtf_battle_sim::Cell),
//! not pixels — the px boundary lives in the presenter, and the world->cell inverse reuses it.
//!
//! # Module layout (GTW-201)
//!
//! - [`plugin`] — the [`GdtfBattleInputPlugin`] wiring (the `add_systems` ordering) + its marker.
//! - [`sets`] — the [`InputSystems`] ordering anchor.
//! - [`picking`] — cursor->cell picking, the world->cell inverse, the hover-highlight emitter.
//! - [`selection`] — ganger selection + the unified click/turn control surface.
//! - [`gamepad`] — the gamepad software cursor + its act surfaces.
//! - [`intent`] / [`keyboard`] / [`keybinds`] / [`cycle`] / [`fire_mode`] / [`fire_surface`] —
//!   the shared act-intent seam, the keyboard surface, the data-driven keybinds, the cyclic
//!   orders, the fire-mode resource, and the FIRE decision helper.

pub mod cycle;
pub mod fire_mode;
pub mod fire_surface;
pub mod gamepad;
pub mod intent;
pub mod keybinds;
pub mod keyboard;
pub mod selection;

mod picking;
mod plugin;
mod sets;

pub use cycle::{FACING_CYCLE, STANCE_CYCLE, next_facing, next_stance};
pub use fire_mode::{SelectedFireMode, sync_fire_mode_on_select};
pub use gamepad::{
    ActivePointer, CURSOR_SPEED, CURSOR_STICK_DEADZONE, CursorSpeed, CursorStickDeadzone,
    GamepadCursor, emit_gamepad_cursor_move, gamepad_click_act, gamepad_turn,
    mouse_reclaims_pointer, move_cursor, move_gamepad_cursor,
};
pub use intent::{
    ActIntent, ActWriters, LevelStep, PendingActIntent, dispatch_act_intents, step_level,
};
pub use keybinds::{BoundKey, Keybinds, KeybindsHandle, load_keybinds, resolve_keybinds};
pub use keyboard::{level_keys, posture_keys, select_clear_key};
pub use picking::{
    InspectMode, InspectTarget, emit_highlight_request, pick_hovered_cell, world_to_cell,
};
pub use plugin::{GdtfBattleInputActive, GdtfBattleInputPlugin};
pub use selection::{
    LeftClickOutcome, LeftClickReads, PathPreviewTarget, PinOutcome, PreviewGrids, SelectedShooter,
    SelectionHighlight, TurnReads, apply_left_click, apply_pin, auto_select_first_player_ganger,
    decide_left_click, decide_pin, decide_turn, left_click_act, populate_path_preview,
    populate_reachable_overlay, right_click_turn_to_face, update_selection_highlight,
};
pub use sets::InputSystems;
