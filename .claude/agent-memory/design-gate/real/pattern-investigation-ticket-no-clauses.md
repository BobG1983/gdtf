---
name: pattern-investigation-ticket-no-clauses
description: Auditing an investigation bug that carries no numbered clauses — enumerate the symptoms from the description plus every comment, and rule each against a code path, not a claim.
metadata:
  type: feedback
---

An investigation ticket ("check whether this is a bug") has no acceptance criteria. Number the
SYMPTOMS from the description and every comment, and give each its own verdict. A symptom ruled
"not a defect" needs a cited code path that explains the observation, not a summary sentence.

**Why:** one such thread reported four symptoms across a description and three comments — log
interleaving, a tag with no shots, corpse text before its shots, a unit stepping backward. Three
were real timing defects; one was a consequence of the one-act-per-tick AI. Treating the thread as
"log-order-only" would have shipped narrower than what was reported.

**How to apply:**

- For a "not a defect" ruling, open the producer and show the observation follows from it.
  `crates/gdtf_battle_sim/src/ai/brain.rs:113` sorts enemies by `cell_order` and the loop `break`s
  at the first enemy to act (`:165`, `:184`), so concurrent walks interleave deterministically —
  that is an explanation; "AI works as designed" is not.
- For a "presentation timing" symptom, check whether the presenter reader takes the raw sim
  message or the cursor-time wrapper `Played<M>`
  (`crates/gdtf_battle_presenter/src/playback/emit.rs:25`). A raw `MessageReader<SimFact>`
  anywhere in `gdtf_battle_presenter` is the whole bug class.
- Report what stays raw after the fix as notes. Terrain tile swaps still read
  `MessageReader<CoverDestroyed>` / `<SlabDestroyed>` directly
  (`crates/gdtf_battle_presenter/src/render/terrain/swaps.rs:57,87`), and floating combat text
  still anchors pops on a live `Query` over `&Position`
  (`crates/gdtf_battle_presenter/src/actors/fx/fct/stacked_reader.rs:23,36`).

Put the verdict on the ticket before it closes. An answer that lives only in the implementer's
summary is not durable.
