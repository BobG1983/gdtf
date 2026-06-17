//! The 7-type weapon×armor matchup lookup — the Paley-tournament wheel and the
//! punch-&-shred multiplier it yields.
//!
//! This is the E3.2 slice (`docs/combat/matchup.md`, `docs/combat/resolution.md`
//! §5, `docs/combat/two-paradox-tournament.md`). It is **pure logic over the E3.1
//! enums** ([`crate::weapon::DamageType`] / [`crate::armor::ArmorType`]) plus the
//! tuning scalars — it touches neither [`crate::weapon::Weapon`] nor
//! [`crate::armor::ArmorPiece`], and carries no pixel.
//!
//! ## The wheel
//!
//! The seven types are a **2-paradox tournament** (a directed Paley graph, n=2,
//! OEIS A362137; `docs/combat/two-paradox-tournament.md`). Each type maps to a
//! [`WheelNode`] `0..=6`. The shipped game orientation
//! (`docs/combat/matchup.md` §"Drop-in data") is the quadratic **non-residues**:
//! weapon node `w` penetrates armor nodes `{(w+3)%7, (w+5)%7, (w+6)%7}`. So a
//! matchup resolves in one lookup:
//!
//! - same node (the mirror) → [`Matchup::Neutral`];
//! - armor node in the weapon node's strong set → [`Matchup::Favorable`];
//! - else → [`Matchup::Resisted`].
//!
//! Each type is favorable vs exactly 3, resisted vs exactly 3, neutral vs 1 (its
//! own mirror) — a balanced regular tournament, balanced *by construction* rather
//! than by hand-authoring 49 cells.
//!
//! ## Modifier, never auto-win
//!
//! The matchup is a **multiplier on weapon `punch` and `shred` only**
//! (`docs/combat/matchup.md` §"Modifier never auto-win", resolution.md §5) — it
//! does NOT touch `damage`, `floor`, `protection`, `integrity`, or `hardness`.
//! [`matchup_multiplier`] yields that scalar from [`crate::tuning::CombatTuning`]
//! (favorable ×1.33 / neutral ×1.0 / resisted ×0.34 are TUNING defaults). Its
//! signature can see *only* a [`Matchup`] and the tuning, so no other stat is
//! reachable from it — the actual application to punch & shred is a later E3 slice.

#[cfg(test)]
mod test;
mod wheel;

pub use wheel::{Matchup, MatchupMultiplier, WheelNode, matchup, matchup_multiplier};
