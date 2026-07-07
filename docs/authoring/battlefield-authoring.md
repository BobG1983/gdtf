# Battlefield Authoring Guide — themes, prefabs, situations, gangs

How a playable battle is authored: a THEME nominates the terrain palette,
PREFABS are the reusable level fragments procgen packs, a SITUATION requests
the battle (theme + board size + placed gangers + hazards), and GANGS are the
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
| `placements` | `Vec<TerrainPlacementEntry>` | list of `(piece:, at:)` | Each entry: a `TerrainUuid` + a `(cell, level)`. May be empty |

There is NO authored-opening / connectivity field: inter-fragment connectivity
is by-construction in the procgen assembler (every placement reserves a 1-cell
`default_floor` seam). The level-wide floor is the THEME's `default_floor`,
never a prefab placement.

### 2c. Loading, procgen, and the editor

Prefabs load through a bespoke resolve (a nested tree into a bucketed
registry — `crates/gdtf_app/src/states/load/systems/resolve/prefab.rs`,
building the `PrefabRegistry` bucketed per `(theme, size, role)` key,
`crates/gdtf_battle_sim/src/level/prefab/registry.rs`). At
battle generation, procgen packs fragments for the requested theme/size into
the board and merges the situation's gangers over the result. The content
editor (`cargo edrun`) authors and saves prefabs to the same tree with the
same one-owner path consts.

## Part 3 — Situations (the battle request)

### 3a. The one authored situation

The game loads exactly ONE situation file:
`assets/content/situations/skirmish.ron` — a single-file hot-RON load (NOT a
folder family; a new `.ron` dropped beside it loads nothing), with a fallback
so a bad file never strands `Load`
(`crates/gdtf_app/src/states/load/plugin.rs`).

### 3b. The schema — by example (abridged from the shipped file)

```ron
(
    theme: "00000000-0000-0000-0000-01840a900001",   // ThemeUuid — the terrain family to generate from
    grid_size: (width: 30, height: 30, levels: 4),   // the procgen board (≤ 60×60×8)
    gangers: [
        (
            gang:       "gang_0",       // gang ROSTER ref — a assets/content/gangs/ file stem
            member:     "Alex Mercer",  // member within that gang, by roster display name
            at:         (cell: (x: 5, y: 6), level: 0),  // spawn (cell, level)
            faction:    0,              // SIDE for this battle (placement, not roster)
            facing:     East,           // one of the 8 grid directions
            stance:     Standing,       // Standing | Crouching | Prone
            aiming:     false,          // aim-mode flag
            life_state: Alive,          // terminal life state
        ),
        // … more placed gangers …
    ],
    fields: [                            // optional initial hazards (GTW-545)
        (at: (cell: (x: 9, y: 8), level: 0), field: "toxic_waste_pool"),
    ],
)
```

The situation authors ONLY theme + board size + placements (+ initial
fields): since GTW-433 the TERRAIN is procgen-generated from `theme` +
`grid_size` against the loaded prefab library, driven by the battle's
deterministic seed; the legacy inline-terrain fields are `#[serde(default)]`
and stay unauthored. A placed ganger is a REFERENCE — the roster (identity,
attributes, equipment) resolves against the `GangRegistry` at setup; the
situation assigns only placement + faction.

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
            speed:     3.0,            // the EIGHT direct attributes (GTW-384 two-layer stat model):
            aim:       3.0,            //   raw potential, from which the computed combat stats
            strength:  4.0,            //   (Shooting / HP / Wounds / TU …) are DERIVED at setup
            toughness: 12.0,           //   via assets/core_tuning/stat.tuning.ron
            reflexes:  3.0,
            cool:      6.0,
            grit:      19.0,
            luck:      1.0,            // severity-roll tail (not a computed stat)
            armor:     "flak_vest",    // armor KEY — a assets/content/armor/ file stem
            weapon:    "volatile_charge",  // ranged weapon KEY — a assets/content/weapons/ranged/ stem
                                           // (the GTW-547 satchel charge — its on_death Explode fires
                                           // when Alex dies; see on-death-authoring.md)
            // melee_weapon: "chainsword",  // OPTIONAL melee KEY; omitted → the "fists" default
        ),
    ],
)
```

Schema types: `GangRoster` / `GangMember`
(`crates/gdtf_battle_sim/src/combatants/ganger/gang.rs`). `melee_weapon:` is
`#[serde(default)]` — an omitting member resolves to the shipped `fists`
melee weapon at setup, so EVERY ganger can melee
([weapon-authoring.md](weapon-authoring.md)). Attribute magnitudes are
per-ganger DATA, never pinned tuning.

Gangs are a stem-keyed folder family (`GangsFamily`,
`crates/gdtf_content_families/src/gangs.rs`) — folder-loaded, salvaged,
hot-reloadable like every family ([content-families.md](content-families.md)).

### 4b. The CURRENT gang editor surface

Gangs are authored in the CONTENT EDITOR binary (`cargo edrun`), in its GANG
Workbench mode (GTW-636; the 2026-07-06 ruling — gangs are authored OUTSIDE
the game binary, and the old in-game debug gang editor is GONE). The mode
(`crates/gdtf_content_editor/src/gang_form/` — model; the egui form is its
`egui_shell/gang_form_ui/` sibling) loads any gang from the registry (or
starts a new one), edits members — add/remove/rename, the eight attributes,
weapon/armor picks plus the `melee_weapon` key — shows the live derived
stats through the real GTW-384 pipeline, and SAVES back to
`assets/content/gangs/<name>.gang.ron` in the exact loader schema (the
round-trip contract; the write path derives its folder + extension from the
same one-owner spellings the loader reads — GTW-621/634).

## Part 5 — Verify

- **Suite:** `cargo dtest`. Situations:
  `crates/gdtf_app/tests/load_situation.rs` +
  `crates/gdtf_app/tests/migrated_skirmish_theme.rs`. Prefabs:
  `crates/gdtf_app/tests/load_prefab.rs`, `load_prefabs.rs`,
  `migrated_prefab_content.rs`. Gangs: `crates/gdtf_app/tests/load_gangs.rs`
  (family suite), `load_gangs_spawn.rs` (setup resolution), and the GANG-mode
  round-trip (`crates/gdtf_content_editor/tests/gang_mode.rs` — save into a
  TempDir root, reload through the real loader). The whole graph:
  `load_ref_integrity.rs` / `load_ref_salvage.rs`.
- **In game:** `cargo drun` — the shipped skirmish loads, procgen assembles
  the industrial_hive board, both gangs deploy at their prefab corners.
  Editing `skirmish.ron`, a gang file, or a prefab under `cargo drun`
  hot-reloads it; a dangling gang/member/theme/piece reference prints on the
  end-of-`Load` report.
- **Editor:** `cargo edrun` for terrain/theme/prefab/gang authoring (the GANG
  Workbench mode owns rosters — GTW-636).
