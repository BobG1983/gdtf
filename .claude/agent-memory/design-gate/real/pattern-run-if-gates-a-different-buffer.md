---
name: pattern-run-if-gates-a-different-buffer
description: A run_if must name the buffers the system's params actually read; when it names only the old one, check every registration path before calling it a violation.
metadata:
  type: feedback
---

When a presenter reader is re-pointed from `MessageReader<S>` to
`MessageReader<Played<S>>`, its `run_if` has to move too — otherwise the gate names a
resource the system no longer touches and does not name the one its param needs.

**Why:** the shape that satisfies both concerns is in
`crates/gdtf_battle_presenter/src/actors/fx/fct/stacked_reader.rs`. The reader drains
`Played<C::Signal>` (`:35`), and `add_consequence_fct` gates on
`Messages<Played<C::Signal>>` (`:91`) — the buffer the param needs — AND on
`Messages<C::Signal>` (`:92`), the sim buffer, kept so a harness that registers no sim
buffers leaves the reader inert. `BattleInProgress` and `FxTuning` are on the same
condition (`:90,93`).

**How to apply:** a gate naming only the sim buffer is not automatically a defect. The
dangerous combination is "sim buffer present, wrapper buffer absent", so find the registrar
and check every call site. Here both live in one plugin build —
`crates/gdtf_battle_presenter/src/plugin/topdown/plugin.rs:57` calls `register_playback`
(which calls `register_played_messages`, `playback/emit.rs:61`) and `:79` calls
`register_consequence_fct_families` (`plugin/topdown/fx.rs:66-75`) — so match the family
list against the `add_message::<Played<…>>()` list and confirm every family is covered. If
it is, rule it a note and say so out loud, because the "inert without the sim buffer" doc
claim stops being true in one direction. If any path can register a family's sim buffer
without its `Played` wrapper, it is a real param-validation failure: Bevy routes those to
the global error handler, which panics unless a harness installed a warn handler.
