# Combat Log Authoring Guide

How the battlescape combat log gets its lines — the sources, the
presenter-side forwarder, the classify layer, the coverage contract, and
how to add a new log source (GTW-328 / GTW-572; the forwarder half moved
presenter-side in GTW-620 so the whole family lives in ONE crate).

The log is the bottom-left HUD strip of recent combat events that scroll up
and fade. It is pure VIEW: it reads sim fact messages and ganger names, owns
no combat rule, and writes nothing back.

---

## Part 1 — The pipeline: forwarder → appender

- **Forwarders (presenter)** —
  `crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/`: one thin
  forwarder per log SOURCE drains its sim fact message, resolves every
  `Entity` to a name at that boundary, and writes one buffered
  `CombatLogEvent` (`event.rs`). The `CombatLogSource` trait + the generic
  forwarder live in `forward.rs`; one impl per fact message lives in
  `sources.rs`. The renderer plugin registers each source with one
  `add_combat_log_source::<Source>()` line in
  `crates/gdtf_battle_presenter/src/plugin/topdown/combat_log.rs` and exports
  the `CombatLogSystems::Forward` set for cross-crate ordering.
- **Classify (presenter)** — `classify_log_event` (`classify.rs`) is the ONE
  pure phrasing + palette layer: event → `LogLine` (text + swatch + emphasis).
  The shot outcome REUSES the FCT shot classifier verbatim (never duplicated);
  a `None` shot report yields NO line (only a blast-detonation seed produces
  one on this path — GTW-559), while a genuine clean miss ALWAYS logs
  (`"<name> missed"`).
- **Appender (app)** —
  `crates/gdtf_app/src/states/running/game/battlescape/combat_log/`: the one
  `bevy_ui` appender drains the event buffer (ordered `.after` the exported
  forward set), classifies, and renders each line as a text node that fades
  over a tuned lifetime and FIFO-despawns past the tuned cap. Lines MUTATE in
  place (fade updates alpha; nothing respawns per frame).

**Feel tuning:** max lines / TTL / fade / panel width are the hot-reloadable
`assets/core_tuning/combat_log.tuning.ron` (the `CombatLogTuning` table,
`crates/gdtf_app/src/states/running/game/battlescape/combat_log/tuning/`).
Edit it under `cargo drun` and the panel re-tunes live.

---

## Part 2 — The coverage contract (Q2, FINAL)

**The log covers ALL state changes.** The shipped sources (the registrar lines
in `crates/gdtf_battle_presenter/src/plugin/topdown/combat_log.rs`):

| Source message | Line it produces |
|----------------|------------------|
| `FireDeclaration` | who fires at whom, with what |
| `MovementOccurred` / `MoveRejected` | movement, and suppressed move refusals |
| `ShotImpactResolved` | staggered per-shot outcomes (hit / miss / cover) |
| `ReloadResult` | reloads |
| turn boundary (bespoke forwarder) | turn started |
| `InjuryInflicted` | wounds, with severity |
| `FallOccurred` | falls |
| `MeleeStruck` | melee damage |
| `OnDeathOccurred` | terminal deaths |
| `SuppressionApplied` | suppression |
| `ArmorBroken` | armor breaks |
| `DotAfflicted` / `FieldAfflicted` / `BleedStarted` | afflictions — ONCE at affliction start |

The affliction rule is the contract's second half: the DOT / field / bleed
**per-tick drain signals never log** (they would spam a line per round); only
the affliction START logs. Where a sim signal lacked the data a line needs,
the SIM gains a fact message (never a rendered line) — `MeleeStruck`,
`DotAfflicted`, `FieldAfflicted`, `BleedStarted` exist for exactly this.

---

## Part 3 — Adding a log source (the pointer)

Adding a source is a PRESENTER-ONLY change: one `CombatLogSource` impl in
`sources.rs`, one `CombatLogEvent` variant, one `classify_log_event` arm —
co-located in `crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/` —
plus the one `add_combat_log_source` registrar line. The full recipe lives in
the module rustdoc of
`crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/mod.rs`; if the same
event also pops floating text, see
[fct-authoring.md](fct-authoring.md) Part 3. Those rustdocs are the source of
truth; this section is the pointer.

---

## Part 4 — Verify

- **Suite:** `cargo dtest`. Presenter-side event coverage:
  `crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/test/` (events /
  shot_outcomes / state_changes). App-side end-to-end (real battle app, real
  lines): `crates/gdtf_app/tests/combat_log/` (lines_from_events, overflow,
  presentation, shot_outcomes) and
  `crates/gdtf_app/tests/combat_log_state_changes.rs` (the Q2 contract pin —
  every covered state change gains its line).
- **In game:** `cargo drun` — play a round: fire, move, reload, melee; watch
  the bottom-left strip gain one line per state change, afflictions log once
  at start, and old lines fade + FIFO out past the cap. Edit
  `assets/core_tuning/combat_log.tuning.ron` to watch the feel re-tune live.
