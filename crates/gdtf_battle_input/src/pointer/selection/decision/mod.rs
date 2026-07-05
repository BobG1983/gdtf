//! The shared click/turn DECISIONS (GTW-238 / GTW-259): the read-only [`decide_left_click`] /
//! [`apply_left_click`] split and the [`decide_turn`] geometry — ONE precedence implementation
//! the mouse and the gamepad both use, plus the [`LeftClickReads`] / [`TurnReads`] bundles.

mod apply;
mod left_click;
mod pin;
mod reads;
mod turn;

pub use apply::apply_left_click;
pub use left_click::{LeftClickOutcome, decide_left_click};
pub use pin::{PinOutcome, apply_pin, decide_pin};
pub use reads::{LeftClickReads, TurnReads};
pub use turn::decide_turn;
