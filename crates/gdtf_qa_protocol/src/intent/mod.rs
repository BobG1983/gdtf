//! [`NetIntent`] — the wire mirror of the `gdtf_battle_input` act vocabulary
//! (GTW-734).
//!
//! One buffered intent a QA client can INJECT (the GTW-694 ruling: injection is at the
//! INTENT layer only — the game maps a [`NetIntent`] onto a `PendingActIntent` /
//! `PendingContextualIntents<A>` push, never a raw `*Requested` write). The enum
//! ([`net_intent`]) mirrors the input crate's classic `ActIntent` variants + the eight
//! contextual acts + the ruled-in `Select`; its small payload enums (the posture /
//! melee-target mirrors) live in [`payload`].

pub mod net_intent;
pub mod payload;

pub use net_intent::NetIntent;
pub use payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet};

#[cfg(test)]
mod test;
