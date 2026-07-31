# Floating Combat Text (FCT) — the consequence-family palette

How the battlescape's floating combat text works, what the color model MEANS,
and where to go to add a new consequence family (GTW-572). FCT is pure VIEW —
everything here lives in the presenter
(`crates/gdtf_battle_presenter/src/actors/fx/fct/`); the sim only emits fact
messages.

There is no RON authoring surface for FCT today — the palette is code-owned
(a later ticket may migrate the swatches to a hot-reloadable `.ron` the way
the FX tuning table did). This guide documents the model and POINTS at the
rustdoc recipe; it does not fork it.

---

## Part 1 — The color model: valence, then severity

Every pop is classified into a **valence** — the presenter's own neutral
category that decides its color (`FctValence` / `valence_color` in
`crates/gdtf_battle_presenter/src/actors/fx/fct/palette.rs`):

| Valence | Swatch | Meaning |
|---------|--------|---------|
| `Damage` | saturated red | An HP-loss number a hit dealt |
| `Lethal` | the same red, drawn BOLD + larger | A ganger went Downed/Dead — heaviest pop in the blood family |
| `Status` | flat amber | A status change with no severity tier |
| `Neutral` | light grey | A clean miss / no-consequence marker |
| `Suppressed` | muted blue-grey | Pinned by fire — matches the desaturated suppressed sprite tint |
| `Dot` | toxic green | A damage-over-time tick (a carried affliction's per-round drain) |
| `Field` | hazard orange | An area-damage-field tick (a zone you STAND in, not an affliction you carry) |

A severity-bearing WOUND does not use the flat `Status` amber — it picks its
exact swatch from the **severity ramp** (`severity_color`, same file): the
wounding tiers `Minor → Critical` interpolate from light amber toward hot
orange-red (a worse wound reads hotter), a `None` graze reads neutral grey (HP
loss, not a wound), and a `Fatal` jumps to the lethal red (a death, not a
wound). The ramp keys off `Severity::rank`, so it tracks the sim's ladder
without pinning any score magnitude.

The palette is deliberately presenter-owned and separate from the `gdtf_ui`
theme: combat valences are not UI chrome.

---

## Part 2 — The pipeline (what a family plugs into)

- **Signal** — a sim fact message (e.g. `SuppressionApplied`, `FieldTicked`).
  The reader drains it wrapped as `Played<Signal>` — the fact at the moment the
  playback cursor SHOWS it, not the moment the sim produced it (GTW-889). The
  sim resolves a whole exchange in one tick, so a family that drained the raw
  buffer popped its tag ahead of the shots that caused it. Every family signal
  therefore needs an act-log deed behind it; the DOT and field per-round drains
  gained theirs in GTW-889.
- **Classify** — the family's pure signal → pop mapping (`ConsequenceFct`
  impl): text, valence/color, emphasis, position.
- **Spawn/stack** — the generic stacked reader spawns the pop and claims its
  stacking slot from the ONE lifetime-aware allocator
  (`crates/gdtf_battle_presenter/src/actors/fx/fct/slot_allocator.rs` /
  `stacked_reader.rs`): the allocator counts the pops still ALIVE on the cell
  and lands the new pop one slot above them, so pops on one cell fan out
  instead of overdrawing — spanning frames (a pop from a prior frame that has
  not despawned is still counted), not just within a single frame. The
  primitive animates rise + fade
  (`crates/gdtf_battle_presenter/src/actors/fx/fct/text.rs`).

Two readers stay OUTSIDE the palette dispatch by design: the multi-pop shot
classifier (`crates/gdtf_battle_presenter/src/actors/fx/fct/reader/`) and the
fall FX reader — see the palette rustdoc for why. Both now claim their stacking
slot from the SAME lifetime-aware allocator as the palette families, but they
remain their own separate readers — they are not merged into the generic
consequence-family dispatch.

---

## Part 3 — Adding a consequence family (the pointer)

The add-a-family recipe lives in the module rustdoc of
`crates/gdtf_battle_presenter/src/actors/fx/fct/families/mod.rs` — one family
file (marker + `ConsequenceFct` impl + its classify unit tests in the SAME
file), one `app.add_consequence_fct::<YourFamily>()` registrar line in
`crates/gdtf_battle_presenter/src/plugin/topdown/fx.rs`
(`register_consequence_fct_families`), and — if the event also logs — one
combat-log source in the same crate
([combat-log-authoring.md](combat-log-authoring.md)). That rustdoc is the
source of truth; do not duplicate it here.

The seven shipped families (`crates/gdtf_battle_presenter/src/actors/fx/fct/families/`):
`BleedingFct`, `ArmorBrokenFct`, `InjuryFct` (severity-ramp color),
`SuppressionFct`, `DotFct`, `FieldFct`, and `OnDeathFct` (the one BOLD
family — the "BOOM" marker).

---

## Part 4 — Verify

- **Suite:** `cargo dtest`. Each family's classify tests live in its own
  family file; the palette/stack behavior is covered in
  `crates/gdtf_battle_presenter/src/actors/fx/fct/families/test.rs`,
  `crates/gdtf_battle_presenter/src/actors/fx/fct/reader/test.rs`, and
  `crates/gdtf_battle_presenter/src/actors/fx/fct/test.rs`.
- **In game:** `cargo drun` — fire on a ganger: red damage pops on hits, grey
  "miss" on clean misses, amber wound pops that read hotter as severity climbs,
  bold red on a kill (plus "BOOM" at an on-death detonation), blue-grey
  "SUPPRESSED", green DOT ticks, and orange field ticks on the shipped toxic
  pool.
