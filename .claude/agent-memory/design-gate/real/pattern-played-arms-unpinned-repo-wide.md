---
name: pattern-played-arms-unpinned-repo-wide
description: Every FX and combat-log test hand-writes Played<M>, so the deed to Played arms in playback/apply.rs are pinned by nothing for the older families.
metadata:
  type: feedback
---

`crates/gdtf_battle_presenter/tests/fx_draw/harness.rs:79` and
`crates/gdtf_app/tests/combat_log/harness.rs:81` both define `play()` as
`app.world_mut().write_message(Played::new(fact))` — they inject the played message
directly. So the `show_entry` match in
`crates/gdtf_battle_presenter/src/playback/apply.rs:48`, the one place an `ActDeed` becomes
a `Played<M>`, is pinned by nothing for the older families. Empty out an arm body and the
suite stays green while the pop or log line dies in the real game.

**Why:** this is the established house style, not something a new diff introduces. The
real-cursor complement is `crates/gdtf_battle_presenter/tests/playback/`, which builds the
app through the real `register_playback` and steps the real cursor — and it contains zero
references to `ArmorBroken`, `SuppressionApplied`, `OnDeathOccurred`, `Bleeding` or
`InjuryInflicted`. Those arms are covered by nothing at all.

**How to apply:** when a diff moves a reader onto `Played<S>` or adds a deed, ask whether it
adds a `tests/playback/` test that drives the real cursor for the NEW deed.
`tests/playback/affliction_ticks.rs:24`
(`a_recorded_dot_drain_is_played_with_its_cell_and_amount`) is the shape to demand: append
an `ActDeed`, step, read the `Played` buffer, assert on entity, cell and amount. If the diff
only adds `fx_draw` tests that call `play()`, the new arm is unpinned and that IS a
violation. Inheriting the pre-existing gap for the older families is not.

Related: [[pattern-mechanism-shipped-instead-of-evidence]].
