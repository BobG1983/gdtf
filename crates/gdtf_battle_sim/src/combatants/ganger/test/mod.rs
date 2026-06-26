//! Relocated unit tests for the ganger per-field state components, split by
//! concern: the component decomposition / defaults / deref proofs ([`components`]),
//! the [`Direction`](crate::ganger::Direction) compass + ring helpers
//! ([`direction`]), and the GTW-384 attribute → computed-stat derivation +
//! hot-reload re-derive ([`derive`]), and the GTW-436 injury-aware derivation +
//! injury-ledger re-derive ([`injury`]).

mod components;
mod derive;
mod direction;
mod injury;
