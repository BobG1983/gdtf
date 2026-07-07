# Content Reference Integrity — the unified dangling-reference contract

How GDTF validates the authored content graph, what a finding looks like, and
what happens to a malformed file or a dangling key (GTW-582, Q3 ruling
2026-07-02). This is the authoring-facing contract. The HOST-AGNOSTIC per-edge
checks (gang equipment, weapon attachments, theme/emplacement terrain,
injury weighting, terrain `graphic_name` — shared by game and editor since
GTW-630) live at
`crates/gdtf_content_families/src/validate/`; the game-bespoke edges
(situation, prefabs) and the game's registration at
`crates/gdtf_app/src/states/load/systems/validate/mod.rs`; the editor's
registration at `crates/gdtf_content_editor/src/validate/`; and the shared
report/salvage vocabulary at `crates/gdtf_assets/src/family/report/` /
`crates/gdtf_assets/src/family/salvage.rs`.

---

## The contract in one paragraph

Every authored cross-file reference is validated ONCE, at the END of `Load` —
after every content family's registry has resolved and strictly before the
`Load → Intro` transition. The pass walks the whole graph and emits ONE
consolidated loud `warn!` report listing every dangling reference; the same
findings persist in the `ContentIntegrityReport` resource. Validation is loud,
**never fatal**: `Load` always exits (the no-strand guarantee), and the
existing runtime guards (a battle setup still aborts fail-closed on a bad key)
stay in place. Mistakes become visible at `Load`, not at battle-request time.

The **content editor runs the same pass** (GTW-630) over the edges it loads —
theme → terrain UUIDs, emplacement → mounted-weapon keys, the gang
equipment keys (weapon / armor / melee incl. the implicit `fists` default —
GTW-651), the injury-weighting keys (GTW-654), and the terrain
`graphic_name` → sprite-def keys (GTW-663) — and RE-ARMS it on every
hot-reload of a watched registry: the
report is reset, re-checked against the current content, and re-published.
The watch set spans every registry the registered checks read, so an edit to
EITHER side of an edge — the gang file OR the weapons/armor/melee folder it
references — re-runs every check onto the one consolidated report. A dangling
terrain UUID or equipment key authored in the editor therefore surfaces at
authoring time (at the save/edit), not on the next game launch.

## What is validated (the reference graph)

| Referencing content | Key it authors | Resolves against | Key scheme |
| --- | --- | --- | --- |
| situation `gangers[].gang` (`assets/content/situations/skirmish.ron`) | gang name | gang files' stems (`assets/content/gangs/`) | file stem |
| situation `gangers[].member` | member name | that gang's roster `members[].name` | display name |
| gang member `weapon` | weapon key | `assets/content/weapons/ranged/` stems | file stem |
| gang member `armor` | armor key | `assets/content/armor/` stems | file stem |
| gang member `melee_weapon` (or the implicit `fists` default) | melee key | `assets/content/weapons/melee/` stems | file stem |
| weapon / melee-weapon `attachments[]` | item key | `assets/content/attachments/` stems | file stem |
| injury weighting rows (`assets/content/injuries/weighting/`) | injury key | `assets/content/injuries/<category>/` stems | file stem |
| situation `theme` | theme UUID | theme defs (`assets/content/terrain/<theme>/`) | UUID |
| situation `walls` / `scatter` / `slabs` / `floors` / `default_floor` | terrain UUID | terrain defs | UUID |
| situation `fields[].field` | field key | `assets/content/fields/` stems | file stem |
| theme `default_floor` + `terrain` palette | terrain UUID | terrain defs | UUID |
| terrain def `Emplacement.mounted_weapon` | weapon key | ranged weapon stems | file stem |
| terrain def `presenter_kind.graphic_name` | sprite-def name | `assets/content/sprites/` stems (GTW-663) | file stem |
| prefab `theme` (`assets/content/maps/`) | theme UUID | theme defs | UUID |
| prefab `placements[].piece` | terrain UUID | terrain defs | UUID |

The gang path carries TWO key schemes at once — a gang is referenced by its
FILE STEM, a member by its roster DISPLAY-NAME — and every finding names which
scheme failed, so a "renamed the file but not the reference" mistake and a
"renamed the member" mistake read differently in the report.

## What a finding looks like

Each finding prints the referencing file/key context, the dangling target, the
target family, and the key scheme, consolidated into one `warn!` block:

```text
content reference contract: 2 finding(s) at end of Load:
  - dangling reference: content/situations/skirmish.ron: placed ganger `Vex 9` names `ghost_gang` (file-stem key), not found in GangRegistry
  - dangling reference: gang `gang_0` member `Alex Mercer` names `ghost_vest` (file-stem key), not found in ArmorRegistry
```

A clean graph logs one `info!` line instead. The findings also persist in the
`ContentIntegrityReport` resource (the shipped-content test pins it EMPTY —
graph integrity is CI-enforced, magnitudes never are).

## Malformed files fail per FILE, not per family

A `.ron` that fails to parse used to fail its whole folder — Bevy's
`load_folder` aborts on the first bad member — and the folder then resolved to
an EMPTY registry: every sibling vanished for one typo. Since GTW-582 the
resolve SALVAGES the folder per-file: every well-formed sibling still loads,
and the malformed file alone fails, loudly, as a `malformed file:` finding on
the same report. A folder that cannot be enumerated at all (a missing
directory) still fails closed to the empty registry. The content editor
(`crates/gdtf_content_editor/`) loads through the same seam, so it inherits the
per-file behavior unchanged.

## Last-resort fallbacks are never silent

Two generation-time degradations survive, as last resorts only, and both
`warn!` AND land on the report when they engage:

- **nil-floor pour** — procgen pours a level whose theme resolves no
  `default_floor` with the nil sentinel (setup then falls back to the tuning
  move cost);
- **empty-board fallback** — when procgen cannot assemble a level at all (no
  prefabs for the theme), the battle uses the authored situation's terrain
  as-is.

## Adding a family edge (the recipe — one crate)

A new cross-file reference edge costs ONE check system in ONE crate
(`crates/gdtf_content_families/src/validate/`, beside the family glue impls),
plus one `register_reference_check(...)` hook per host that loads the edge's
registries:

1. Write the check beside its edge family in
   `crates/gdtf_content_families/src/validate/` — a Bevy system over the sim
   registries that appends a `DanglingRef` finding per unresolved key.
2. Hook it in the game's registrar
   (`crates/gdtf_app/src/states/load/systems/validate/`) and, IF the editor
   loads every registry the check reads, in the editor's
   (`crates/gdtf_content_editor/src/validate/`). Never register a check whose
   registries a host doesn't load — the host's `Check` window gates on the
   registries its registered checks read, so an unloaded one would hold the
   whole window shut (and an empty stand-in would false-fail every key).

Only game-bespoke edges (families the editor never loads — situations,
prefabs) live in the game crate's `validate/` instead (the injuries edge
moved to the shared crate in GTW-654 when the editor started loading the
injuries family).

## Fixing a finding

1. Read the finding's target + family: that names the missing file (file-stem
   keys) or the missing definition (UUID keys).
2. File-stem key: create `assets/content/<family>/<key>.<infix>.ron`, or fix
   the referencing spelling.
3. UUID key: the UUID must match an authored def's `key:` field under
   `assets/content/terrain/<theme>/` (terrain/theme defs are payload-keyed —
   see [terrain-authoring.md](terrain-authoring.md)).
4. `malformed file:` findings name a parse error — fix the RON (hot-reload
   rebuilds the registry live for member edits).
