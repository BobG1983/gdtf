---
name: pattern-ticket-mints-a-type-c1-already-landed
description: A ticket drafted before the wire vocabulary landed still orders the builder to mint types that already exist — grep wire/ before accepting any "this ticket mints X" clause.
metadata:
  type: feedback
---

A ticket drafted against an older branch can order you to create something the tree already has.
Building it ships a duplicate.

**Why:** the game's wire vocabulary landed COMPLETE in one commit (`2a2f79a9`), so any Risk or
clause saying "this type does not exist yet" is stale for everything under
`crates/gdtf_app/src/dev/net_qa/wire/`. `FocusStepNet` is at `wire/key.rs:95` with its four
variants, a round-trip case (`focus_steps_round_trip_every_direction`, `wire/test/drive.rs:15`)
and a schema case (`wire/test/schema.rs:87`) — and a later ticket's clause 1 still ordered a new
`wire/focus.rs` to mint it. The same holds for `StepperCommandNet` (`wire/misc.rs:87`),
`MouseButtonNet` (`wire/pointer.rs:53`), `StanceNet` / `AimNet` / `FacingNet` / `MeleeTargetNet`
(`wire/act_payload.rs:11,23,35,56`), `KeyPressNet` (`wire/key.rs:86`), `CellLevelNet`
(`wire/cell.rs:67`) and `GangerToken` (`wire/token.rs:10`).

**How to apply:** before accepting a clause that says "this ticket mints X", grep
`crates/gdtf_app/src/dev/net_qa/wire/` for `pub enum X` / `pub struct X`. A hit makes the clause
a necessity failure, not work. The landed vocabulary has NO window handle — zero `Window` hits
under `wire/` — so a reply naming a window entity really does need a new token, plus the
round-trip and schema calls the guard in `wire/test/coverage.rs` demands for every wire type.

Related: [[pattern-two-types-same-name-after-vocabulary-move]].
