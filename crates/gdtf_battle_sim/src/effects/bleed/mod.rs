//! The bleed-out clock + the ganger-bleeding signal — the E3.7 slice (GTW-189).
//!
//! [`tick_bleed`](crate::effects::bleed::tick_bleed) is the per-round drain of `docs/combat/resolution.md` §9 (and
//! `docs/combat/wounds-and-roster.md` §"Downed → death … state machine"): **once
//! per full round** every [`crate::ganger::LifeState::Downed`] ganger carrying the
//! [`BleedingOut`](crate::effects::bleed::BleedingOut) condition
//! gains a "Bleeding Out" stack draining a flat tuning [`crate::tuning::BleedRate`]
//! of [`crate::ganger::Wounds`] (the stack count = turns down = total Wounds lost, a
//! clock you can read), and when its [`crate::ganger::Wounds`] life pool empties the
//! ganger becomes [`crate::ganger::LifeState::Dead`] through the **same once-only
//! terminal gate** as the E3.6 `apply_hit` path (`Wounds ≤ 0` → Dead). Each draining
//! tick also emits a [`Bleeding`](crate::effects::bleed::Bleeding) message carrying the ganger [`bevy::prelude::Entity`]
//! so the presenter can surface the clock on screen.
//!
//! ## Who bleeds (and who is skipped)
//!
//! - **Only gangers who ENTERED the tick [`crate::ganger::LifeState::Downed`]
//!   bleed** (GTW-641: the gate is a pre-tick snapshot, so a ganger the tick's own
//!   injury-HP bleed just downed has been down zero rounds and drains its first
//!   Wound on the NEXT tick). An [`crate::ganger::LifeState::Alive`] ganger is up
//!   and fighting; an [`crate::ganger::LifeState::Dead`] one is already a corpse —
//!   both are skipped, mutate nothing, and emit no
//!   [`Bleeding`](crate::effects::bleed::Bleeding) (the "once-only" property: once
//!   Dead, the next tick skips it).
//! - **A stabilized Downed ganger is skipped.** Once an ally has dressed the wound,
//!   [`stabilize_downed`](crate::acts::downed::stabilize_downed) REMOVES the
//!   [`BleedingOut`](crate::effects::bleed::BleedingOut) condition, so the clock
//!   halts: no drain, no new stack, no [`Bleeding`](crate::effects::bleed::Bleeding) — the Wounds already lost stay
//!   lost and the ganger **remains Downed**. The two down-transition sites INSERT the
//!   condition (the [`crate::apply_hit`] damage down-gate via
//!   [`mark_downed_bleeding`](crate::effects::bleed::mark_downed_bleeding), and this
//!   slice's own injury-HP-bleed down-gate inline); `tick_bleed` only **reads** it.
//!
//! The drain, the emit, and the terminal gate all happen on the **same** draining
//! tick — including the lethal one (the tick that drops Wounds to `0` both emits a
//! [`Bleeding`](crate::effects::bleed::Bleeding) and flips the ganger to [`crate::ganger::LifeState::Dead`]).
//!
//! [`Bleeding`](crate::effects::bleed::Bleeding) is a buffered Bevy **message** (`#[derive(Message)]`), mirroring
//! the [`crate::occupancy_sync::CoverDestroyed`] / [`crate::armor_wear::ArmorBroken`]
//! precedent (`docs/combat/resolution.md` §9 names the analogous ganger-bleeding
//! signal) — **NOT** the targeted/observer `Event` API (`bevy-traps.md` #4: Bevy
//! 0.18 renamed buffered `Event`/`EventReader` to `Message`/`MessageReader`), so it
//! is written with [`bevy::prelude::MessageWriter`] and read with
//! [`bevy::prelude::MessageReader`]. Pure, render-free model logic: no renderer, no
//! pixel; the drain saturates (no underflow, no `unwrap`).

mod downgate;
mod schedule;
mod tick;

#[cfg(test)]
mod test;

pub use downgate::mark_downed_bleeding;
pub use schedule::enemy_phase_started;
pub use tick::{BleedOngoing, BleedStarted, Bleeding, BleedingOut, tick_bleed};
