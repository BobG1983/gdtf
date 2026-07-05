//! GTW-525 — the SHOVE act (the last GTW-39 falls child), proven END-TO-END through the REAL
//! wiring (`dispatch_shove` + the melee / fire auto-shove hooks + the shared GTW-523 fall path).
//!
//! The clause contract (C1-C3 / C6-C7):
//!
//! - **QA(1) displace-away direction (C1)** — a deliberate shove pushes the target ONE cell
//!   directly AWAY from the shover (the attacker->target direction).
//! - **QA(2) unsupported=>fall via 523 + `FallOccurred` (C1)** — a shove off a ledge (no
//!   supporting slab at the destination storey) makes the target FALL through the shared
//!   `resolve_drop` path, emitting `FallOccurred` + dropping Hp.
//! - **QA(3) supported=>move (C1)** — a shove onto a supported cell (ground / a Present slab)
//!   moves the target one cell, no fall, no Hp loss.
//! - **QA(4) blocked=>no-op (C1)** — a shove into a solid (another ganger / a wall) is a no-op:
//!   the target does not move.
//! - **QA(5) deliberate act: any ganger, adjacency-gated, TU spent, NO wound (C2)** — the
//!   deliberate shove gates 8-adjacency + opposing + alive, spends the shove TU, and deals NO
//!   wound of its own (a supported shove leaves Hp/Wounds untouched).
//! - **QA(6) weapon-tag auto-shove on a connecting MELEE strike (C3)** — a `shove`-tagged melee
//!   weapon knocks the target back on a CONNECTING strike (in addition to the damage).
//! - **QA(7) weapon-tag auto-shove on a connecting RANGED shot (C3)** — a `shove`-tagged gun
//!   knocks the target back on a CONNECTING shot.
//! - **QA(8) miss / non-tagged => NO shove (C3)** — a non-`shove` weapon never shoves; a missed
//!   attack never shoves.
//! - **QA(9) determinism (C6)** — the same seed + same message order yields the identical shove
//!   outcome (the shove itself is RNG-free; only the fall draws, deterministically).
//!
//! No pinned tunable magnitude — every assert is a RELATION (moved / fell / no-op / Hp dropped
//! / TU spent), never a shipped balance number (the brittle-test rule). Render-free, zero
//! pixels; the one `World` mutation is in a TEST BODY (`bevy-traps.md` #7 carve-out).

mod deliberate_act;
mod displacement;
mod harness;
