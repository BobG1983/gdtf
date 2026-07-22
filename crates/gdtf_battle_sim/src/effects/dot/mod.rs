//! The **damage-over-time runtime** (GTW-544, child GTW-41e) — the per-turn [`tick_dot`](crate::effects::dot::tick_dot)
//! drain of every afflicted ganger, the [`DotApplied`](crate::effects::dot::DotApplied) boundary message the fire path
//! bridges a penetrating DOT hit through, and the [`apply_dot`](crate::effects::dot::apply_dot) system that attaches (or
//! REFRESHES) a [`Dot`](crate::weapon::Dot) on the struck ganger.
//!
//! The DOT model splits along the model/runtime line (mirroring the GTW-438 injury split):
//!
//! - The DECISION is PURE + in-fold: [`resolve_and_apply`](crate::resolve_and_apply::resolve_and_apply)
//!   freezes the [`Dot`](crate::weapon::Dot) to attach onto
//!   [`GangerVerdict::dot_applied`](crate::resolve_and_apply::GangerVerdict::dot_applied) when a hit
//!   PENETRATES armor ([`PenetratingDamage`](crate::resolve_hit::PenetratingDamage) `> 0`)
//!   from a weapon carrying a [`DotProfile`](crate::weapon::DotProfile). It owns no attach.
//! - The SIDE EFFECTS live HERE, at the message boundary
//!   ([`apply_dot`](crate::effects::dot::apply_dot), `bevy-traps.md` #7 — query / [`Commands`](bevy::prelude::Commands) /
//!   [`MessageReader`](bevy::prelude::MessageReader), no `&mut World`): the fire path emits
//!   one [`DotApplied`](crate::effects::dot::DotApplied) per penetrating DOT round, and [`apply_dot`](crate::effects::dot::apply_dot) attaches or REFRESHES
//!   the [`Dot`](crate::weapon::Dot) component (refresh-not-stack — a second penetrating DOT
//!   hit RESETS the affliction).
//! - The CLOCK is [`tick_dot`](crate::effects::dot::tick_dot): once per full round (the SAME enemy-phase-start cadence as
//!   the §9 bleed-out clock, [`enemy_phase_started`](crate::effects::bleed::enemy_phase_started)) it
//!   drains each afflicted ganger's [`Hp`](crate::ganger::Hp) DIRECTLY by the DOT's per-turn
//!   damage — no armor matchup, no injury roll, no RNG — decrements the remaining turns,
//!   emits a [`DotTicked`](crate::effects::dot::DotTicked) signal, removes the [`Dot`](crate::weapon::Dot) when the turns run
//!   out, and flips the ganger to [`LifeState::Dead`](crate::ganger::LifeState::Dead) if the
//!   drain empties its HP (the GTW-544 locked design: a DOT tick that brings HP to `0` KILLS).
//!
//! [`DotApplied`](crate::effects::dot::DotApplied) / [`DotTicked`](crate::effects::dot::DotTicked) are buffered Bevy **messages** (`bevy-traps.md` #4 — NOT
//! the observer `Event` API), mirroring [`Bleeding`](crate::effects::bleed::Bleeding) /
//! [`InjuryInflicted`](crate::acts::InjuryInflicted). Pure, render-free model logic: no
//! renderer, no pixel; the drain saturates (no underflow, no `unwrap`).

mod apply;
mod tick;

#[cfg(test)]
mod test;

pub use apply::{DotAfflicted, DotApplied, apply_dot};
pub use tick::{DotTicked, tick_dot};
