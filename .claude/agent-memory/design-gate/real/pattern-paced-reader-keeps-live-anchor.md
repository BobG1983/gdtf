---
name: pattern-paced-reader-keeps-live-anchor
description: A reader moved onto Played<S> keeps reading position from a live query, so the WHEN moves to cursor time while the WHERE stays at sim time.
metadata:
  type: feedback
---

When a presenter reader is re-pointed from `MessageReader<S>` to
`MessageReader<Played<S>>`, check every OTHER world read inside its loop. A live
`Query<&Position>` — or any live sim component — was coherent while the system ran at sim
time and becomes incoherent once the system runs at cursor time, because the sim has moved
on by then. Pacing the WHEN and leaving the WHERE live is half a fix.

**Why:** the consequence-FCT reader drains `Played<C::Signal>`
(`crates/gdtf_battle_presenter/src/actors/fx/fct/stacked_reader.rs:35`) and resolves
`PopAnchor::GangerPosition` to a cell inside the loop. Three families anchor that way
(`families/bleeding.rs:22`, `armor_broken.rs:22`, `injury.rs:22`). Read from a live
`Position`, the tag lands on the cell the sim has reached, not the cell the sprite is on.

**The accepted fix shape** is in that same file: the query is
`(&'static Position, Option<&'static DrawnPosition>)` (`:23`, aliased `AnchorData`) and the
anchor resolves as `drawn.map_or(**position, |drawn| *drawn.position())` (`:44-48`) — the
live `Position` demoted to a fail-closed existence check plus the pre-seed fallback. The
life half is the same shape: `actors/ganger/visibility.rs:95` queries `Option<&DrawnLife>`
and `:105` resolves `drawn.map_or(*life, |drawn| **drawn)`.

**How to apply:** demand an integration test that sets the mirror and the live value to
DIFFERENT values and asserts against the mirror by name — otherwise the test passes whether
the code reads the mirror or the live value. Both exist as models:
`tests/fx_draw/drawn_anchor.rs:41`
(`a_consequence_pop_anchors_at_the_drawn_cell_not_the_cell_the_sim_ran_ahead_to`) and
`tests/ganger_draw/shown_life.rs:15`.

Related: [[pattern-played-reader-gated-on-the-sim-buffer]]
