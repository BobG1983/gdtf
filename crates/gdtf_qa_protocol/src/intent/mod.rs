//! [`NetIntent`] — the wire mirror of the `gdtf_battle_input` act vocabulary
//! (GTW-734).
//!
//! One buffered intent a QA client can INJECT. Most variants are at the INTENT layer (the
//! GTW-694 ruling: the game maps a [`NetIntent`] onto a `PendingActIntent` /
//! `PendingContextualIntents<A>` push, never a raw `*Requested` write). The enum
//! ([`net_intent`]) mirrors the input crate's classic `ActIntent` variants + the eight
//! contextual acts + the ruled-in `Select`; its small posture / melee-target payload enums
//! live in [`payload`]. The GTW-783 raw-input family (keypress / hover / focus-set) drives
//! keyboard-shaped behaviour that no act covers; its keyboard vocabulary lives in
//! [`raw_input`], and the game side writes it through the SAME windowing-input messages the
//! backend feeds — still never a direct sim mutation.

pub mod net_intent;
pub mod payload;
pub mod raw_input;

pub use net_intent::NetIntent;
pub use payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet};
pub use raw_input::{KeyNet, KeyPressNet, KeybindActionNet};

#[cfg(test)]
mod test;
