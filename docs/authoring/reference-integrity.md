# Content Reference Integrity — the unified dangling-reference contract

How GDTF validates the authored content graph, what a finding looks like, and
what happens to a malformed file or a dangling key (GTW-582, Q3 ruling
2026-07-02). This is the authoring-facing contract; the validator's module docs
live at `crates/gdtf_app/src/states/load/systems/validate/mod.rs` and the
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
