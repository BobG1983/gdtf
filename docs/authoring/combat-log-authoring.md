# Combat Log Authoring Guide

How the battlescape combat log gets its lines — the sources, the
presenter-side forwarder, the classify layer, the coverage contract, the fog
gate, and how to add a new log source. The forwarder half lives
presenter-side, so the whole family sits in ONE crate.

The log is the bottom-left HUD strip of recent combat events that scroll up
and fade. It is pure VIEW: it reads sim fact messages, ganger names and the
shown fog, owns no combat rule, and writes nothing back.

---

## Part 1 — The pipeline: forwarder → appender

- **Forwarders (presenter)** —
  `crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/`: one thin
  forwarder per log SOURCE drains its sim fact message, asks `PanelSight`
  (`sight.rs`) what the screen may say about it, and writes one buffered
  `CombatLogEvent` (`event.rs`) — or nothing, when the screen never saw the
  act (Part 2). Every `Entity` becomes a name at that same boundary. The
  `CombatLogSource` trait + the generic forwarder live in `forward.rs`; one
  impl per fact message lives in `sources.rs`. The renderer plugin registers
  each source with one `add_combat_log_source::<Source>()` line in
  `crates/gdtf_battle_presenter/src/plugin/topdown/combat_log.rs` and exports
  the `CombatLogSystems::Forward` set for cross-crate ordering.
- **Classify (presenter)** — `classify_log_event` (`classify.rs`) is the ONE
  pure phrasing + palette layer: event → `LogLine` (text + swatch + emphasis).
  The shot outcome REUSES the FCT shot classifier verbatim (never duplicated);
  a `None` shot report yields NO line (only a blast-detonation seed produces
  one on this path), while a genuine clean miss ALWAYS logs
  (`"<name> missed"`).
- **Appender (app)** —
  `crates/gdtf_game/src/states/running/game/battlescape/combat_log/`: the one
  `bevy_ui` appender drains the event buffer (ordered `.after` the exported
  forward set), classifies, and renders each line as a text node that fades
  over a tuned lifetime and FIFO-despawns past the tuned cap. Lines MUTATE in
  place (fade updates alpha; nothing respawns per frame).

**Feel tuning:** max lines / TTL / fade / panel width are the hot-reloadable
`assets/core_tuning/combat_log.tuning.ron` (the `CombatLogTuning` table,
`crates/gdtf_game/src/states/running/game/battlescape/combat_log/tuning/`).
Edit it under `cargo drun` and the panel re-tunes live.

---

## Part 2 — The coverage contract and the fog gate

**The log covers ALL state changes the squad could see.** The shipped sources
(the registrar lines in
`crates/gdtf_battle_presenter/src/plugin/topdown/combat_log.rs`):

| Source message | Line it produces |
|----------------|------------------|
| `FireDeclaration` | who fires at whom, with what |
| `MoveCompleted` / `MoveRejected` | one line per finished walk naming the mover and no cells, and suppressed move refusals |
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
`DotAfflicted`, `FieldAfflicted`, `BleedStarted`, `MoveCompleted` exist for
exactly this.

**The fog gate is the contract's last rule: the log reports nothing the squad
could not perceive.** One classifier decides — `classify_act` in
`gdtf_battle_sim::visibility`, the same one `log.read` calls over the wire, so
panel and command cannot drift apart. It returns three outcomes:

- **named** — the act is shown where the screen is lit and the actor is one
  the screen can identify;
- **unnamed** — the act is shown, and the actor reads `Someone`, the name
  `PanelSight` returns for a ganger it cannot identify;
- **withheld** — no line at all. A blanked line is not an option: printing one
  announces that something happened, which is the leak the gate closes.

The forwarder reads `ShownSquadVisibility` and each ganger's drawn cell, not
live squad visibility, so a line never appears for a cell the screen has not
lit yet (the playback rule in [architecture.md](../architecture.md)). Own-gang
gangers are always named, and so is a ganger the screen cannot place at all.
Which cell a source asks about follows the act: most ask the actor's drawn
cell, and a finished walk asks the cell it ended on. A fire declaration is
observable when the shooter's cell or its target's is lit; a shot outcome when
the shooter's cell is lit, or the round stopped on a ganger, a slab or a patch
of ground the screen is showing — a cover hit and a clean miss are judged from
the shooter's cell alone. Melee is observable when either the attacker's or the
target's cell is lit, and a turn boundary is never withheld.

---

## Part 3 — Adding a log source

Adding a source is a PRESENTER-ONLY change: one `CombatLogSource` impl in
`sources.rs`, one `CombatLogEvent` variant, one `classify_log_event` arm —
co-located in `crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/` —
plus the one `add_combat_log_source` registrar line. The impl is where the new
source picks the cell its act touches and returns `None` when the classifier
withholds it; the existing impls in `sources.rs` are the worked examples. If
the same event also pops floating text, see
[fct-authoring.md](fct-authoring.md) Part 3.

---

## Part 4 — Verify

- **Suite:** `cargo dtest`. Presenter-side event coverage:
  `crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/test/` (events /
  shot_outcomes / state_changes). App-side end-to-end (real battle app, real
  lines): `crates/gdtf_game/tests/game_suite/combat_log/` (lines_from_events, overflow,
  presentation, shot_outcomes, fog_gate) and
  `crates/gdtf_game/tests/game_suite/battle_shell/combat_log_state_changes.rs` (the
  coverage-contract pin — every covered state change gains its line).
- **In game:** `cargo drun` — play a round: fire, move, reload, melee; watch
  the bottom-left strip gain one line per finished walk and per state change,
  afflictions log once at start, and old lines fade + FIFO out past the cap.
  An enemy the squad cannot see adds no line, and one it can see acting
  without being identified reads `Someone`. Edit
  `assets/core_tuning/combat_log.tuning.ron` to watch the feel re-tune live.
