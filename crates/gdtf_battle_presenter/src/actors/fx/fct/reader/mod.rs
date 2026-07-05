//! GTW-302 (slice 3) / GTW-327 (slice 2): the [`ShotFired`] → floating-combat-text
//! CLASSIFICATION — the shared, reusable functions that turn each fired round's
//! already-computed [`HitReport`] into the ordered list of rise-and-fade pops the slice-2
//! primitive ([`spawn_floating_text`](super::text::spawn_floating_text)) draws, plus the
//! `(cell, level)` anchor those pops sit on.
//!
//! GTW-302 originally spawned the pops IMMEDIATELY when draining the [`ShotFired`] buffer (in a
//! `read_shot_fired_text` system here). That dumped a whole burst / full-auto volley's numbers
//! on the single drain frame even though the GTW-306 tracer projectiles
//! ([`spawn_shot_projectiles`](super::super::spawn_shot_projectiles)) fly STAGGERED, so the
//! numbers and the bolts desynced. GTW-327 slice 2 REPURPOSES this module: the immediate-spawn
//! system is gone; [`classify_report`](classify::classify_report) +
//! [`anchor_cell`](anchor::anchor_cell) are now the SHARED classification
//! [`spawn_shot_projectiles`] calls at projectile-spawn time, threading each round's
//! [`ClassifiedPop`](classified::ClassifiedPop) list + anchor THROUGH the staggered projectile
//! → impact pipeline so each shot's pops appear when THAT shot's impact lands (see
//! [`projectile`](super::super::projectile) / [`impact`](super::super::impact)). This also sets
//! up GTW-328's shared event → text layer (the classification is a clean, reusable seam).
//!
//! For each round [`classify_report`](classify::classify_report) CLASSIFIES its
//! [`report`](gdtf_battle_sim::ShotFired::report)
//! — dispatching on the per-kind [`HitVerdict`](gdtf_battle_sim::HitVerdict) (GTW-573) —
//! into the Phase-1 combat events this slice covers, one pop per event. A ganger verdict
//! (its `applied` block + struck `part`) yields the FLESH family:
//!
//! - **HP damage** (`applied.hit.hp_damage > 0`) — the numeric loss, e.g. `-7`, drawn
//!   the damage RED ([`FctValence::Damage`](super::palette::FctValence::Damage)).
//! - **Wound gained** (`applied.severity` is a real wounding tier) — `"<Part> <Tier>"`,
//!   e.g. `"Torso Major"`, drawn the AMBER ramp
//!   ([`severity_color`](super::palette::severity_color)).
//! - **Graze** (`applied.severity == Severity::None`) — `"Grazed"`,
//!   drawn GREY ([`FctValence::Neutral`](super::palette::FctValence::Neutral)): HP loss but no
//!   Wound spent (resolution.md §6).
//! - **Penetration verdict** (`applied.hit.penetrating`) — `"Armor pierced"` (GREY, it
//!   went through the armor) when `> 0`, else `"Armor held"` (AMBER, the armor soaked it).
//! - **DOWN / DEAD** (`applied.life_after`) — `"DOWN"` / `"DEAD"` drawn the lethal RED
//!   in [`FctEmphasis::Bold`](super::text::FctEmphasis::Bold) (the heaviest pop in the blood
//!   family — bold weight + a larger size, the contract's "RED bold"; the all-caps tag is a
//!   complementary cue, not a substitute).
//!
//! A round that struck the WORLD rather than a ganger (GTW-386) reads the STRUCTURAL family
//! instead of the flesh family above — qualitative structural feedback, never flesh damage:
//!
//! - **Cover / slab hit** (a cover / slab verdict) — `"Cover hit"` / `"Slab hit"`
//!   drawn neutral GREY (a chip indicator: the round struck and chipped the structure). The
//!   verdict carries no per-hit structural DAMAGE NUMBER, so the feedback is qualitative.
//! - **Cover / slab DESTROYED** (the verdict's `destroyed` is `Some`)
//!   — `"Cover Destroyed"` / `"Slab Destroyed"` drawn the lethal RED in
//!   [`FctEmphasis::Bold`](super::text::FctEmphasis::Bold), the structural mirror of the
//!   ganger DOWN / DEAD tag (same heaviest-pop weight, but the word reads structural).
//! - **Ground** (a ground-accrual verdict) — a minor neutral-GREY `"Dust"` impact cue (the
//!   ground is damaged, never destroyed — purely cosmetic, `docs/combat/resolution.md` §3.2).
//!
//! The AUX slice (4) covers the rest of the contract's Phase-1 list that is NOT derivable from
//! [`ShotFired`] alone — armor `"Armor -N"` / `"Armor Broken"`, reload `"Reloaded"` / `"Empty"`
//! / `"No TU"`, and bleeding — from the consequence messages
//! ([`ArmorBroken`](gdtf_battle_sim::ArmorBroken) / [`Bleeding`](gdtf_battle_sim::Bleeding)) or
//! a future reload signal. Those pops are NOT staggered (they ride their own one-shot
//! consequence messages, not the per-round projectile pipeline).
//!
//! Pure VIEW (ADR-0001): these functions only READ the message + look up the hit ganger's cell;
//! the SPAWN happens downstream (at the impact) and never reads any raw sim state by polling and
//! never writes the sim.
//!
//! [`ShotFired`]: gdtf_battle_sim::ShotFired
//! [`HitReport`]: gdtf_battle_sim::HitReport
//! [`spawn_shot_projectiles`]: super::super::spawn_shot_projectiles

mod anchor;
mod classified;
mod classify;

#[cfg(test)]
mod test;

pub(in crate::actors::fx) use anchor::anchor_cell;
pub(in crate::actors::fx) use classified::ClassifiedPop;
pub(in crate::actors::fx) use classify::classify_report;
