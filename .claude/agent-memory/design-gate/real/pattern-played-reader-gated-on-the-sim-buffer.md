---
name: pattern-played-reader-gated-on-the-sim-buffer
description: A presenter reader draining Played<S> whose run_if names only Messages<S> is the house pattern here — check the registrars and rule NOTE, unless the Played buffer can go missing or a doc's recipe omits it.
metadata:
  type: feedback
---

A `read_*` / `forward_*` system taking `MessageReader<Played<S>>` whose `.run_if` names
`resource_exists::<Messages<S>>` — the sim buffer, not the one the param reads — is the normal
shape in this tree, not a broken gate. Check the registrars, then rule NOTE by default.

**Why:** `Messages<Played<M>>` is presenter-owned and always registered.
`crates/gdtf_battle_presenter/src/plugin/topdown/plugin.rs:57` calls `register_playback`, which
calls `register_played_messages` unconditionally
(`crates/gdtf_battle_presenter/src/playback/register.rs:18`), and that lists a `Played<M>` for all
21 families (`crates/gdtf_battle_presenter/src/playback/emit.rs:62-82`). The same plugin build
installs every reader, so gating on the wrapper buffer would be trivially true — while gating on
the sim buffer keeps the convention that a harness with no sim buffers leaves the reader inert.
The shape is everywhere: `actors/fx/registrar.rs:36` (six FX readers),
`actors/fx/fct/log_event/forward.rs:95` (thirteen combat-log sources),
`plugin/topdown/combat_log.rs:48`, `plugin/topdown/fx.rs:45`. Exactly one registrar names both.

**It is a VIOLATION only when one of these holds:**

- A family's `Played<M>` is missing from `register_played_messages` while its reader is
  registered. The sim-buffer gate passes, the system runs, and param validation fails — Bevy
  routes that to the global error handler, which panics. No production code calls
  `set_error_handler(warn)`; only test harnesses do.
- A doc gives a closed add-a-family recipe that omits the `register_played_messages` line.
  `docs/authoring/fct-authoring.md:77-85` is the live example: it lists one family file plus one
  `add_consequence_fct::<…>()` registrar line, and names "the module rustdoc of
  `crates/gdtf_battle_presenter/src/actors/fx/fct/families/mod.rs`" as the source of truth — that
  file is 18 lines of `mod` and `pub use` with no rustdoc at all.

**The accepted fix names BOTH buffers.** `actors/fx/fct/stacked_reader.rs:35` reads
`MessageReader<Played<C::Signal>>`, and its registrar gates on `Messages<Played<C::Signal>>`
(`:91`), the buffer the param needs, plus `Messages<C::Signal>` (`:92`), the sim buffer, kept so a
reader in a bare harness stays inert instead of panicking.

**How to apply:** grep `resource_exists::<Messages<` in the presenter and read the registrars —
there is no comment explaining the choice. The check worth running is whether every family a
reader covers has a `Played<M>` line in `emit.rs`. Do not argue that registration ORDER inside the
plugin makes it safe: run conditions are evaluated every frame, so what matters is that
`register_playback` is called at all, not where in `build` it sits.

Related: [[pattern-paced-reader-keeps-live-anchor]] (the other half of the same re-pointing move),
[[pattern-partial-falsehood-sweep]].
