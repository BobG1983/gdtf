# Content Reference Integrity — the unified dangling-reference contract

How GDTF validates the authored content graph, what a finding looks like, and
what happens to a malformed file or a dangling key ( Q3 ruling
2026-07-02). This is the authoring-facing contract. The HOST-AGNOSTIC per-edge
checks (gang equipment, weapon attachments, theme/emplacement terrain, injury
weighting, a terrain def's own view coverage, its `leaves_behind`, every
`views[].sprite` key it names, a prefab's theme and placed terrain UUIDs, and
every `on_death` `LeaveField.field` key, shared by game and editor) live at
`crates/gdtf_content_families/src/validate/`, and so do the situation's own
edges; the game's registration at
`crates/gdtf_app/src/states/load/systems/validate/register.rs`; the editor's
registration at `crates/gdtf_content_editor/src/validate/register.rs`; and the
shared report/salvage vocabulary at `crates/gdtf_assets/src/family/report/` /
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

The **content editor runs the same pass** over the edges it loads —
theme → terrain UUIDs, emplacement → mounted-weapon keys, the gang
equipment keys (weapon / armor / melee incl. the implicit `fists` default; a
member leaving one out authors no edge to check),
the injury-weighting keys, each terrain def's own view coverage, its
`leaves_behind` → terrain-def or sprite-def key, every `views[].sprite` →
sprite-def key, the prefab `theme` → theme UUID, every prefab
`placements[].piece` → terrain UUID, every `on_death` `LeaveField.field` key
a terrain def or a ranged weapon spec authors, and the situation's
`gangers[].gang`, `gangers[].member`, `theme`, terrain and `fields[].field` keys — and RE-ARMS it
on every hot-reload of a watched registry: the report is reset, re-checked
against the current content, and re-published.
The watch set spans every registry the registered checks read, so an edit to
EITHER side of an edge — the gang file OR the weapons/armor/melee folder it
references — re-runs every check onto the one consolidated report. The field
defs are watched, so dropping a field key re-runs every check that reads it.
The loaded situation is watched alongside the registries, so rewriting it
re-runs every check the same way. A
dangling terrain UUID or equipment key authored in the editor therefore
surfaces at authoring time (at the save/edit), not on the next game launch.

## What is validated (the reference graph)

| Referencing content | Key it authors | Resolves against | Key scheme |
| --- | --- | --- | --- |
| situation `gangers[].gang` (`assets/content/situations/skirmish.ron`) | gang name | gang files' stems (`assets/content/gangs/`) | file stem |
| situation `gangers[].member` | member name | that gang's roster `members[].name` | display name |
| gang member `weapon`, when authored | weapon key | `assets/content/weapons/ranged/` stems | file stem |
| gang member `armor`, when authored | armor key | `assets/content/armor/` stems | file stem |
| gang member `melee_weapon` (or the implicit `fists` default) | melee key | `assets/content/weapons/melee/` stems | file stem |
| weapon / melee-weapon `attachments[]` | item key | `assets/content/attachments/` stems | file stem |
| injury weighting rows (`assets/content/injuries/weighting/`) | injury key | `assets/content/injuries/<category>/` stems | file stem |
| situation `theme` | theme UUID | theme defs (`assets/content/terrain/<theme>/`) | UUID |
| situation `walls` / `scatter` / `slabs` / `floors` / `default_floor` | terrain UUID | terrain defs | UUID |
| situation `fields[].field` | field key | `assets/content/fields/` stems | file stem |
| theme `default_floor` + `terrain` palette | terrain UUID | terrain defs | UUID |
| terrain def `Emplacement.mounted_weapon` | weapon key | ranged weapon stems | file stem |
| terrain def `views` | the views its kind and tags owe | the def's own `views[].view` rows | — |
| terrain def `leaves_behind` | terrain UUID or sprite-def name | terrain defs, or `assets/content/sprites/` stems | UUID / file stem |
| terrain def `views[].sprite` | sprite-def name | `assets/content/sprites/` stems | file stem |
| terrain def `on_death` `LeaveField.field` | field key | `assets/content/fields/` stems | file stem |
| ranged weapon spec `on_death` `LeaveField.field` | field key | `assets/content/fields/` stems | file stem |
| prefab `theme` (`assets/content/maps/`) | theme UUID | theme defs | UUID |
| prefab `placements[].piece` | terrain UUID | terrain defs | UUID |

A terrain def's `leaves_behind` authors two more references of this kind, one
naming another terrain def and one naming a sprite def. Both are checked, on
both hosts. A `leaves_behind` naming a terrain def no registry holds leaves
nothing behind at the destroyed cell, and one naming a sprite no registry holds
draws the magenta missing-sprite marker. That is what the finding warns about.

The `views` row is the one edge that resolves against the def itself rather
than another family. A def owes a set of views derived from its `sim_kind` and
its `tags`, and the check reports one `MissingViews` finding per def naming
every view that def draws no art for.

The gang path carries TWO key schemes at once — a gang is referenced by its
FILE STEM, a member by its roster DISPLAY-NAME — and every finding names which
scheme failed, so a "renamed the file but not the reference" mistake and a
"renamed the member" mistake read differently in the report.

## What a finding looks like

Each dangling-reference finding prints the referencing file/key context, the
dangling target, the target family, and the key scheme. A missing-views finding
resolves against no other family, so it prints the def and every view it names
no art for instead. Both go into one `warn!` block:

```text
content reference contract: 3 finding(s) at end of Load:
  - dangling reference: content/situations/skirmish.ron: placed ganger `Vex 9` names `ghost_gang` (file-stem key), not found in GangRegistry
  - dangling reference: gang `gang_0` member `Alex Mercer` names `ghost_vest` (file-stem key), not found in ArmorRegistry
  - terrain def `Rusted Barrels` (3d3f9b52-6f6e-4b6a-9a3e-2f8c1d7e4a01) is missing Facing(East), Facing(West)
```

A clean graph logs one `info!` line instead. The findings also persist in the
`ContentIntegrityReport` resource (the shipped-content test pins it EMPTY —
graph integrity is CI-enforced, magnitudes never are).

## Malformed files fail per FILE, not per family

A `.ron` that fails to parse used to fail its whole folder — Bevy's
`load_folder` aborts on the first bad member — and the folder then resolved to
an EMPTY registry: every sibling vanished for one typo. The resolve now SALVAGES
the folder per-file: every well-formed sibling still loads,
and the malformed file alone fails, loudly, as a `malformed file:` finding on
the same report. A folder that cannot be enumerated at all (a missing
directory) still fails closed to the empty registry. The content editor
(`crates/gdtf_content_editor/`) loads through the same loader, so it inherits the
per-file behavior unchanged.

## Last-resort fallbacks are never silent

Two generation-time degradations survive, as last resorts only, and both
`warn!` AND land on the report when they engage:

- **nil-floor pour** — procgen pours a level whose theme resolves no
  `default_floor` with the nil sentinel (setup then falls back to the tuning
  move cost, and every storey-0 cell with no authored piece draws the magenta
  missing-tile marker, because no floor def resolves);
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

The injuries edge moved to the shared crate when the editor started loading the
injuries family, and the situation's four edges moved when the editor started
loading the situation.

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
