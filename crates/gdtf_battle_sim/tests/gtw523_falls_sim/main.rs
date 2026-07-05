//! GTW-523 — the fall mechanic end-to-end through the REAL wiring (`apply_falls`).
//!
//! A slab destroyed under a standing ganger makes the WIRED `apply_falls` (the production
//! system, NOT a reimplementation) drop the faller, apply weight-free fall damage + injury
//! through the shared resolve/apply + injury pipeline, and emit `FallOccurred`. This test
//! builds a real headless app with the production plugins (`SimActsPlugin` +
//! `OccupancyMaintenancePlugin` + `FallsPlugin`), writes a buffered `SlabDestroyed`, drives
//! `app.update()`, and asserts the whole clause contract:
//!
//! - **QA(1)** a 3-storey column, ganger on level 2, `SlabDestroyed` at (cell,2) → `Position`
//!   drops to the highest Present/ground + storeys == expected; the faller keys off
//!   `level == 2`, NOT level+1 (an EXPLICIT roof-decoy ganger at level 3 does NOT fall);
//! - **QA(2)** a multi-storey drop through an `Absent` intermediate → the first Present/ground;
//! - **QA(3)** a ganger on a DIFFERENT level does not move;
//! - **QA(4)** a stair lower-endpoint occupant is BRACED — it does not fall;
//! - **QA(5)** Hp dropped + a seeded injury + `InjuryInflicted` fired + damage monotone in
//!   storeys;
//! - **QA(6)** determinism — the same seed twice yields identical outcomes;
//! - **QA(7)** a hot-edit of `per_storey_damage` → different damage (the FORMULA, not a
//!   shipped magnitude);
//! - **QA(8)** regression — a destroyed slab is NON-pathable (the `VerticalLinkGraph` is
//!   untouched) and LOS flies THROUGH the hole (the shared `march_vector`).
//!
//! Render-free, zero pixels; the one `World` mutation is in a TEST BODY (`bevy-traps.md`
//! #7 carve-out) — no helper here takes `&mut World` / `&World`.

mod damage;
mod harness;
mod terrain_after;
mod who_falls;
