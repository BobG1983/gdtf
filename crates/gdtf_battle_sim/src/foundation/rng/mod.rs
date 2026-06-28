//! Injected seeded-RNG harness: independent per-subsystem streams derived from
//! one root [`BattleSeed`].
//!
//! `docs/combat/resolution.md` is unambiguous — the resolution math is
//! **deterministic** and "**every draw comes from the model-owned seeded RNG,
//! injected once at setup**". GTW-14 replaces the former single `SimRng` with
//! INDEPENDENT per-subsystem stream [`Resource`](bevy::prelude::Resource)s,
//! each derived from the same [`BattleSeed`] root via a stable label-hash:
//!
//! ```text
//! per_stream_u64 = fnv1a64( root.to_le_bytes() ++ LABEL_bytes )
//! stream_rng     = ChaCha12Rng::seed_from_u64(per_stream_u64)
//! ```
//!
//! Using [`rand_chacha::ChaCha12Rng`] (not `StdRng`, which is "non-portable / output
//! may be platform-dependent") makes the full root → draw chain byte-stable for
//! cross-build, cross-platform replay. Adding a stream cannot perturb any other
//! stream's seed (each depends only on `(root, its_own_label)`).
//!
//! ## Resource types (insert at setup, remove at teardown)
//!
//! | [`Resource`](bevy::prelude::Resource) | Draw sites |
//! |---|---|
//! | [`ShotRng`] | §1 trajectory sample + §4 body-part roll |
//! | [`SeverityRng`] | §6 roll term |
//! | [`LootRng`] | reserved |
//! | [`InjuryRng`] | in-battle injury roll (GTW-438) |
//! | [`ProcgenRng`] | reserved |
//! | [`ReactionRng`] | reserved — reaction-fire (GTW-466 substrate) |
//!
//! All six are inserted at battle setup (from [`BattleSeed`]) and removed at
//! teardown. Reserved streams draw nothing and cannot perturb active streams.
//!
//! ## Draw surface — `impl_sim_stream!`
//!
//! Every stream type exposes the same surface, stamped by the
//! `impl_sim_stream!` macro (the single source of truth): `from_root`,
//! `rng`, `next_u64`, `random_range`. Adding a draw method means editing
//! the macro.
//!
//! ## Binding constraint
//!
//! No system may take `Res<ShotRng>` / `Res<SeverityRng>` (etc.) for read-only
//! access — see [`streams`] module doc for the full rationale.

mod seeded;
pub(super) mod streams;
#[cfg(test)]
mod test;

pub use seeded::BattleSeed;
pub use streams::{InjuryRng, LootRng, ProcgenRng, ReactionRng, SeverityRng, ShotRng};
