//! Ganger selection + the unified click control surface (GTW-225 / GTW-227 / GTW-238 /
//! GTW-255): the [`SelectedShooter`] resource, the [`SelectionHighlight`] sprite, the ONE
//! disambiguated left-click decision ([`left_click_act`]), and the right-click turn-to-face
//! surface ([`right_click_turn_to_face`]) — all gated to the player's own faction.
//!
//! # The unified left-click decision (GTW-238 — replaces the two-system race)
//!
//! Before GTW-238 a left-click ran TWO systems that only avoided colliding by ordering, never a
//! real precedence. GTW-238 REPLACES that pair with ONE decision, [`decide_left_click`] /
//! [`apply_left_click`], resolving a single Left-press edge through a STRICT precedence chain —
//! first match wins (the user-confirmed precedence): FIRE (a fire mode + an ENEMY occupant + a
//! player-faction selection + the shared `can_fire` guard) → SELECT (your own ganger) → MOVE (a
//! player-faction selection over an empty, in-bounds, unblocked cell) → CLEAR. A fire mode over
//! an EMPTY / your-own / non-enemy cell falls THROUGH to MOVE (it does not lock out move).
//!
//! # Shared with the gamepad (GTW-259)
//!
//! [`decide_left_click`] / [`apply_left_click`] / [`decide_turn`] are the SHARED decisions the
//! mouse ([`left_click_act`] / [`right_click_turn_to_face`]) and the gamepad
//! ([`gamepad_click_act`](crate::gamepad::gamepad_click_act) /
//! [`gamepad_turn`](crate::gamepad::gamepad_turn)) both call — ONE precedence implementation,
//! two devices. Both click surfaces read [`Res<PlayerFaction>`](gdtf_battle_sim::PlayerFaction).
//! (The GTW-254 `WorldClickSuppressed` modal click-through guard was REMOVED in GTW-265 once
//! the fire-mode popup picker — the only modal — was replaced by an always-visible 3-toggle
//! sub-panel.)
//!
//! # Don't act through the UI (GTW-286)
//!
//! Every click branch keys off the LIVE hovered cell ([`InspectTarget::hovered`](crate::InspectTarget::hovered)), which the picker
//! ([`pick_hovered_cell`](crate::pick_hovered_cell)) now resolves to [`None`] for any cursor
//! OUTSIDE the map viewport rect (a margin / a UI panel) — the GTW-286 viewport gate in
//! `resolve_hovered_cell`. So a click over a UI button never reaches a cell: nothing hovered →
//! the decision is [`LeftClickOutcome::NoOp`] (GTW-288), which leaves the selection UNTOUCHED.
//! It does NOT clear: clearing on every bottom-UI click flickered the status panel and broke
//! Mode/Stance (which need the selection to resolve the weapon). CLEAR is reserved for a valid
//! in-grid cell (see below). The GTW-271 camera-pan gate had been pan-path only; GTW-286 adds
//! the SAME gate to the click/pick path so one chokepoint covers move-on-UI AND the
//! reticle-under-panels, for mouse and gamepad alike.
//!
//! # Clicking an enemy is a no-op on your selection (GTW-287)
//!
//! When you have a player ganger selected and click an ENEMY-occupied cell you can't FIRE on,
//! the decision is [`LeftClickOutcome::NoOp`] — it leaves [`SelectedShooter`] untouched (no
//! clear, no transient `None`, no "No ganger selected" flash, no auto-select revert). Enemies
//! are inspected via the GTW-274 hover panel, never selected/cleared as your shooter. CLEAR
//! still fires for the genuine "nothing to act on" cases (an empty / blocked cell with a
//! non-player or stale selection).
//!
//! # Clicking ALSO pins the inspect panel (GTW-300)
//!
//! Every left-click (mouse OR gamepad) resolves a SECOND, ORTHOGONAL effect: the inspect-panel
//! pin ([`decide_pin`] / [`apply_pin`], a [`PinOutcome`]). Clicking COVER or an ENEMY pins the
//! panel on that cell (it freezes there, ignoring hover); clicking an EMPTY tile unpins (hover
//! resumes); clicking your OWN ganger (a SELECT) or FIRING on an enemy KEEPS the pin. The pin
//! lives on a DIFFERENT resource ([`InspectTarget`](crate::InspectTarget)) than the
//! selection/act effect, so the two compose on one click with no double-dispatch.

mod auto_select;
mod decision;
mod fire_target;
mod highlight;
/// The SHARED player-faction selection ordering + the GTW-458 Prev/Next cycle decision —
/// the one `(z, y, x)` order the auto-select and the cycle both step.
mod order;
mod path_preview;
/// The reachable-range DEBUG overlay POPULATE half (GTW-450) — render-only, so the
/// whole module compiles ONLY in a debug build (`#[cfg(debug_assertions)]`, C1). In a
/// release build `populate_reachable_overlay` / `ReachableGrids` do not exist.
#[cfg(debug_assertions)]
mod reachable;
mod resources;
mod systems;

pub use auto_select::auto_select_first_player_ganger;
pub use decision::{
    LeftClickOutcome, LeftClickReads, PinOutcome, TurnReads, apply_left_click, apply_pin,
    decide_left_click, decide_pin, decide_turn,
};
pub use fire_target::{FireTargetReads, populate_fire_target};
pub use highlight::update_selection_highlight;
pub use order::{CellOrderKey, CycleDirection, cell_order_key, cycle_player_selection};
pub use path_preview::{
    PathPreviewTarget, PreviewGrids, populate_path_preview, reset_move_target_on_fire_mode_change,
};
// GTW-450 — the reachable-overlay POPULATE half is DEBUG-only (C1); re-exported only
// under `#[cfg(debug_assertions)]` so the release build never names it.
#[cfg(debug_assertions)]
pub use reachable::{ReachableGrids, populate_reachable_overlay};
pub use resources::{SelectedShooter, SelectionHighlight};
pub use systems::{left_click_act, right_click_turn_to_face};
