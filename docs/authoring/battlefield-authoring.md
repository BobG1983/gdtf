# Battlefield Authoring Guide — themes, prefabs, situations, gangs

How a playable battle is authored: a THEME nominates the terrain palette,
PREFABS are the reusable level fragments procgen packs, a SITUATION requests
the battle (theme + board size + roster + hazards), and GANGS are the
reusable rosters the situation fields. Everything is UUID- or key-referenced;
the reference graph is validated at the end of `Load`
([reference-integrity.md](reference-integrity.md)).

---

## Part 1 — Themes (pointer)

The UUID-keyed terrain/theme model — the per-theme file layout under
`assets/content/terrain/<theme>/`, the `TerrainDef` / `UuidThemeDef` schemas,
payload-keyed UUIDs, and the theme palette + `default_floor` — is documented
in [terrain-authoring.md](terrain-authoring.md) (Parts 0–1). Prefabs and
situations reference a theme by its `ThemeUuid`; a theme's pieces by their
`TerrainUuid`s.

## Part 2 — Prefabs (level fragments)

### 2a. Where the `.ron` file goes

Prefabs live under `assets/content/maps/<theme>/<size>/` — one folder per
theme, one subfolder per footprint (e.g. `industrial_hive/3x3/`,
`industrial_hive/12x12/`), one fragment per file, named
`<name>.prefab.ron` (the stem is the `PrefabName`). The folder + extension
spellings are the `gdtf_content_families::prefabs` consts
([content-families.md](content-families.md) Part 3).

### 2b. The `.prefab.ron` schema — by example

From the shipped
`assets/content/maps/industrial_hive/3x3/entry_room.prefab.ron`:

```ron
(
    // The theme this fragment draws from (must match a terrain_theme.ron key).
    theme: "00000000-0000-0000-0000-01840a900001",
    // The fragment's footprint: width × height × storey-count.
    size: (width: 3, height: 3, levels: 1),
    // role: Fill — OMITTED here; serde-defaults to SpawnRole::Fill.
    // The placed pieces, each a theme terrain def by UUID at a (cell, level).
    placements: [
        (piece: "00000000-0000-0000-0000-01840a910002", at: (cell: (x: 0, y: 0), level: 0)),
    ],
)
```

**Field reference** (`PrefabSpec`,
`crates/gdtf_battle_sim/src/level/prefab/spec.rs`):

| Field | Rust type | RON form | Notes |
|-------|-----------|----------|-------|
| `theme` | `ThemeUuid` | UUID string | The theme whose defs the placements reference |
| `size` | `GridSize` | `(width:, height:, levels:)` | Footprint, validated to the 60×60×8 coarse-grid maxima |
| `role` | `SpawnRole` | `Player` \| `Enemy` \| `Fill` | `#[serde(default)]` → `Fill`. A packed level needs one `Player` + one `Enemy` deployment fragment |
| `placements` | `Vec<TerrainPlacementEntry>` | list of `(piece:, at:, facing:)` | Each entry: a `TerrainUuid` + a `(cell, level)` + a `TerrainFacing`. `facing` is `#[serde(default)]` → `North`, so an entry may omit it. May be empty |

There is NO authored-opening / connectivity field: inter-fragment connectivity
is by-construction in the procgen assembler (every placement reserves a 1-cell
`default_floor` border). The level-wide floor is the THEME's `default_floor`,
never a prefab placement.

### 2c. Loading, procgen, and the editor

In the game, prefabs load through a bespoke resolve (a nested tree into a
bucketed registry — `crates/gdtf_game/src/states/load/systems/resolve/prefab.rs`,
building the `PrefabRegistry` bucketed per `(theme, size, role)` key,
`crates/gdtf_battle_sim/src/level/prefab/registry.rs`). At
battle generation, procgen packs fragments for the requested theme/size into
the board and DEPLOYS the situation's roster members into the resulting
player/enemy deployment zones. The content editor (`cargo edrun`) reads the same
tree through `PrefabsFamily` (`crates/gdtf_content_families/src/prefabs.rs`) into
a `PrefabRegistry` of its own, and authors and saves prefabs back to the tree with
the same one-owner path consts. Prefab mode's picker opens one onto the canvas:
its extent and theme onto the session, its cells into the painted map, and the
edit storey clamped into the loaded extent (`open_prefab` in
`crates/gdtf_editor/src/open.rs`).

## Part 3 — Situations (the battle request)

### 3a. The one authored situation

Each host loads exactly ONE situation file:
`assets/content/situations/skirmish.ron` — a single-file hot-RON load (NOT a
folder family; a new `.ron` dropped beside it loads nothing), with a fallback
so a bad file never strands `Load`. The path, the mapping and the fallback live
in `crates/gdtf_content_families/src/situation.rs`; the game's `LoadScenePlugin`
and the editor's `register_load` both call them, so the editor checks the
situation's references too.

### 3b. The schema — by example (abridged from the shipped file)

```ron
(
    theme: "00000000-0000-0000-0000-01840a900001",   // ThemeUuid — the terrain family to generate from
    grid_size: (width: 30, height: 30, levels: 4),   // the procgen board (≤ 60×60×8)
    rosters: [                           // the combatants — gang + member + side refs, NO cells
        (gang: "gang_0", member: "Alex Mercer", faction: 0),  // gang ROSTER ref + member + SIDE
        (gang: "gang_0", member: "Kira Vann",   faction: 0),  // faction 0 → the player deployment zone
        (gang: "gang_1", member: "Vex 1",       faction: 1),  // faction 1 → the enemy deployment zone
        // … more roster members …
    ],
    fields: [ // optional initial hazards
        (at: (cell: (x: 9, y: 8), level: 0), field: "toxic_waste_pool"),
    ],
)
```

The situation authors ONLY theme + board size + roster (+ initial fields):
now the TERRAIN is procgen-generated from `theme` + `grid_size`
against the loaded prefab library, and now each roster member's SPAWN
CELL / facing / stance is procgen-DERIVED too — `deploy_rosters` places each
member into its side's deployment zone (faction == `player_faction` → the
player zone, else the enemy zone) DETERMINISTICALLY by the battle's seed. So a
shipped situation authors NO placement cells and NO inline terrain (both are
`#[serde(default)]`). A roster member is a REFERENCE — the roster (identity,
attributes, equipment) resolves against the `GangRegistry` at setup; the
situation assigns only which member and which side.

A `gangers:` list of fully-placed `PlacedGanger`s (each with `at` / `facing` /
`stance` / …) is STILL accepted (`#[serde(default)]`) for TEST fixtures that
need exact cells — but shipped content authors `rosters`.

Note the gang path's TWO key schemes: `gang:` is a FILE-STEM key, `member:` is
that roster's DISPLAY-NAME — the reference report names which scheme failed
([reference-integrity.md](reference-integrity.md)).

## Part 4 — Gangs (reusable rosters)

### 4a. The `.gang.ron` schema

Gangs live under `assets/content/gangs/` — one roster per file,
`<name>.gang.ron`; the file stem is the `GangName` key (there is no `name:`
field). A roster is faction-agnostic and placement-free: the same gang can be
fielded on any side. Abridged from the shipped
`assets/content/gangs/gang_0.gang.ron`:

```ron
(
    members: [
        (
            name:      "Alex Mercer",  // member identity — what a situation's `member:` references
            speed: 3.0, // the EIGHT direct attributes (two-layer stat model):
            aim:       3.0,            //   raw potential, from which the computed combat stats
            strength:  4.0,            //   (Shooting / HP / Wounds / TU …) are DERIVED at setup
            toughness: 12.0,           //   via assets/core_tuning/stat.tuning.ron
            reflexes:  3.0,
            cool:      6.0,
            grit:      19.0,
            luck:      1.0,            // severity-roll tail (not a computed stat)
            armor:     Some("flak_vest"),    // armor KEY — a assets/content/armor/ file stem
            weapon:    Some("volatile_charge"),  // ranged weapon KEY — a assets/content/weapons/ranged/ stem
                                                 // (the satchel charge — its on_death Explode fires
                                                 // when Alex dies; see on-death-authoring.md)
            // melee_weapon: Some("chainsword"),  // OPTIONAL melee KEY; omitted → the "fists" default
        ),
    ],
)
```

Schema types: `GangRoster` / `GangMember`
(`crates/gdtf_battle_sim/src/combatants/ganger/gang.rs`). All three loadout
keys — `armor:`, `weapon:` and `melee_weapon:` — are `#[serde(default)]`
`Option`s, so each is written as `Some("key")` or left out. Ron rejects a bare
value for an `Option` without the `IMPLICIT_SOME` extension, which no
`.gang.ron` in this workspace enables. An omitted or `None` armor means the
member wears none, and an omitted or `None` weapon means it carries no ranged
weapon. A member that omits `melee_weapon` resolves to the shipped `fists`
melee weapon at setup, so EVERY ganger can melee
([weapon-authoring.md](weapon-authoring.md)). Attribute magnitudes are
per-ganger DATA, never pinned tuning.

Gangs are a stem-keyed folder family (`GangsFamily`,
`crates/gdtf_content_families/src/gangs.rs`) — folder-loaded, salvaged,
hot-reloadable like every family ([content-families.md](content-families.md)).

### 4b. The CURRENT gang editor surface

Gangs are authored in the CONTENT EDITOR binary (`cargo edrun`), in its GANG
Workbench mode (the 2026-07-06 ruling — gangs are authored OUTSIDE
the game binary, and the old in-game debug gang editor is GONE). The mode
(`crates/gdtf_editor/src/gang_form/` — model; the egui form is its
`egui_shell/gang_form_ui/` sibling) loads any gang from the registry (or
starts a new one), edits members — add/remove/rename, the eight attributes,
weapon/armor picks, each with a `(none)` row that clears it, plus the
`melee_weapon` key — shows the live derived
stats through the real pipeline, and SAVES back to
`assets/content/gangs/<name>.gang.ron` in the exact loader schema (the
round-trip contract; the write path derives its folder + extension from the
same one-owner spellings the loader reads — /634).

## Part 5 — Verify

- **Suite:** `cargo dtest`. Situations:
  `crates/gdtf_game/tests/game_suite/load_families/load_situation.rs` +
  `crates/gdtf_game/tests/game_suite/migrated_content/migrated_skirmish_theme.rs`. Prefabs:
  `crates/gdtf_game/tests/game_suite/load_families/load_prefab.rs`, `load_prefabs.rs`,
  `crates/gdtf_game/tests/game_suite/migrated_content/migrated_prefab_content.rs`. Gangs:
  `crates/gdtf_game/tests/game_suite/load_families/load_gangs.rs`
  (family suite), `load_gangs_spawn.rs` (setup resolution), and the GANG-mode
  round-trip (`crates/gdtf_editor/tests/editor_suite/mode_shells/gang_mode.rs` — save into a
  TempDir root, reload through the real loader). The whole graph:
  `load_ref_integrity.rs` / `load_ref_salvage.rs`.
- **In game:** `cargo drun` — the shipped skirmish loads, procgen assembles
  the industrial_hive board, both gangs deploy at their prefab corners.
  Editing `skirmish.ron`, a gang file, or a prefab under `cargo drun`
  hot-reloads it; a dangling gang/member/theme/piece reference prints on the
  end-of-`Load` report.
- **Editor:** `cargo edrun` for terrain/theme/prefab/gang authoring (the GANG
  Workbench mode owns rosters —).
