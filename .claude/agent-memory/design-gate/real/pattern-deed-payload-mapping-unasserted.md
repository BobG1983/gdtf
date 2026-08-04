---
name: pattern-deed-payload-mapping-unasserted
description: A new act-log deed is pinned by NAME only — the recorder's field mapping from the sim message is asserted nowhere, so a zeroed or re-derived payload stays green.
metadata:
  type: feedback
---

When a ticket adds an `ActDeed` variant carrying payload fields, check the three hops
separately: sim message to recorded deed
(`crates/gdtf_battle_sim/src/act_log/record/consequence/messages.rs`), deed to `Played<M>`
(`crates/gdtf_battle_presenter/src/playback/apply.rs`), and `Played<M>` to the drawn pop.

**Why:** the sim suite's probe carries the deed NAME only —
`crates/gdtf_battle_sim/tests/act_log/harness.rs:119-125`, where `LoggedFact` holds `seq`,
`actor`, `provenance` and `deed: &'static str` — and drops the payload entirely. Hop 2 asserts
the fields; hop 1, read through that probe, asserts only actor, provenance and name. Writing a
zero amount or a constant cell in the recorder leaves every test green and ships a "-0" pop at
the wrong cell.

**The shape to require:** a `deeds_of(app, name) -> Vec<ActDeed>` helper that clones the WHOLE
deed (`harness.rs:174-185`), plus a fixture whose message anchor is a cell the ganger is NOT
standing in, so a recorder that re-derives the anchor from the live `Query<&Position>` fails.
`crates/gdtf_battle_sim/tests/act_log/affliction_drains.rs` does both: `:33-34` puts the ganger
at `ground(2, 2)` and the drain at `ground(9, 9)`, and `:61-69` and `:98-106` assert the whole
`ActDeed` value. Accept nothing weaker.

**How to apply:** grep the new deed's field names in `crates/*/tests`. If the only hits are the
constructor sites and a `deed_name` arm, the mapping is unpinned. The name-only probe is the
tell — it makes every payload claim in a sim act-log suite vacuous.

Related: [[pattern-tautological-assertion-replacing-a-pin]].
