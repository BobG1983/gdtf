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
//! two devices. Both click surfaces read [`Res<PlayerFaction>`](gdtf_battle_sim::PlayerFaction)
//! and are inert while a `gdtf_app` modal captures the pointer ([`WorldClickSuppressed`],
//! GTW-254 clause 6b).

mod auto_select;
mod decision;
mod highlight;
mod resources;
mod systems;

pub use auto_select::auto_select_first_player_ganger;
pub use decision::{
    LeftClickOutcome, LeftClickReads, TurnReads, apply_left_click, decide_left_click, decide_turn,
};
pub use highlight::update_selection_highlight;
pub use resources::{SelectedShooter, SelectionHighlight, WorldClickSuppressed};
pub use systems::{left_click_act, right_click_turn_to_face};
