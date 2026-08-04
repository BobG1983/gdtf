# Content Families — adding a folder-loaded content family

How GDTF turns a folder of loose `.ron` files into a registry resource, and
what ONE registration line buys you (generic loader; per-file
salvage + integrity report; headless fallback; shared
validation; /634 one-owner path spellings). This is the guide for
engineers adding a NEW content family; each existing family's authoring guide
covers its own schema.

---

## Part 1 — The one-line contract

A new folder-loaded family costs exactly three things:

1. **One marker impl** of the `ContentFamily` trait
   (`crates/gdtf_assets/src/family/def.rs`) in the glue crate
   `crates/gdtf_content_families/src/` — naming the payload `Spec`, the
   registry `Resource`, the `FOLDER`, the dedicated compound `EXTENSION`, and
   how a member keys into the registry (`insert_member`).
2. **One registration line** per host:
   `app.register_content_family::<MyFamily>()` (the `ContentFamilyAppExt`
   extension, `crates/gdtf_assets/src/family/ext.rs`). The game's lines live
   in `crates/gdtf_app/src/states/load/plugin.rs`; the content editor
   registers the families it edits.
3. **A content folder** of `<key>.<infix>.ron` files under `assets/content/`.

That one line yields the WHOLE chain — encoded once, never per-family:

| Guarantee | What it means |
|-----------|---------------|
| Loader + resolve | A `Startup` kick-off folder-loads recursively; the resolve gates on the folder finishing, folds every member through `insert_member`, and inserts the registry EXACTLY once |
| Per-file salvage | A malformed member fails ALONE (a `malformed file:` finding on the integrity report); every well-formed sibling still loads |
| Fail-closed folder | A genuinely un-enumerable folder `warn!`s and publishes an EMPTY registry, so a presence-gated `Load` flow is never stranded |
| Live redrive (hot-reload) | Editing a member under `cargo drun` rebuilds the whole registry in place via the persistent `ContentFolderHandle`, logging the reload |
| `TypeId` member filter | A mixed folder (terrain + theme defs share one tree) never mistypes a member |
| Headless fallback | A `MinimalPlugins` app (no `AssetServer`) registers no chain and seeds `Registry::default()` instead — no panic, and a presence-gated flow still releases |
| Validation window | Reference checks registered per host (`register_reference_check`) run once every registry they read has resolved — see [reference-integrity.md](reference-integrity.md) |

The generic systems live in `crates/gdtf_assets/src/family/systems.rs`; the
salvage in `crates/gdtf_assets/src/family/salvage.rs`; the report vocabulary in
`crates/gdtf_assets/src/family/report/`.

## Part 2 — Why the glue crate, and the two keying shapes

`gdtf_assets` is a deliberate leaf (bevy/ron/serde only) and cannot name sim
registry types; `gdtf_app` cannot host the impls because the content editor
needs the SAME families without depending on the game. So the marker impls
live in the shared glue crate — the module rustdoc of
`crates/gdtf_content_families/src/lib.rs` is the canonical statement of this
mechanism (link, don't fork).

The shipped families vary on ONE axis — where a member's key comes from:

| Keying | Families | Key |
|--------|----------|-----|
| Stem-keyed | `WeaponsFamily`, `MeleeWeaponsFamily`, `ArmorFamily`, `FieldsFamily`, `GangsFamily`, `AttachmentsFamily`, `SpriteDefsFamily` | File stem, infix stripped (`stub_pistol.weapon.ron` → `stub_pistol`) |
| Payload-keyed | `TerrainDefsFamily`, `ThemeDefsFamily` | The UUID inside the def; the filename is a courtesy |

One placement exception: `SpriteDefsFamily` is the one family whose
`Spec`/`Registry` live IN the glue crate
(`crates/gdtf_content_families/src/sprites/`) rather than the sim — sprite
data is presentation-side, and the render-free sim cannot own it.

## Part 3 — Path spellings have ONE owner

Folder and extension strings are declared exactly once and imported
everywhere else — a writer's spelling can never drift from the loader's read
(the bug class: a save-side extension drift made saved gangs silently
invisible to the loader):

- The workspace assets root: `gdtf_assets::WORKSPACE_ASSETS_ROOT`
  (`crates/gdtf_assets/src/workspace.rs`).
- Generic families: `<Family>::FOLDER` / `<Family>::EXTENSION` — the assoc consts
  on each marker impl in `crates/gdtf_content_families/src/`.
- The two bespoke (non-generic) families, which have no `ContentFamily` impl to
  carry consts: `gdtf_content_families::prefabs::{PREFABS_FOLDER,
  PREFAB_EXTENSION}` and `gdtf_content_families::injuries::{INJURIES_FOLDER,
  INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION}` (declared once, imported
  by the game's bespoke Load chain AND the editor's savers).

New code (docs too) cites the consts, never a re-spelled literal.

## Part 4 — The two deliberate exclusions + the single-file loads

Two families stay BESPOKE by design (their shapes don't fit
folder→one-registry):

- **Prefabs** — a nested `content/maps/<theme>/<size>/` tree resolving into a
  bucketed multimap (`crates/gdtf_app/src/states/load/systems/resolve/prefab.rs`).
- **Injuries** — ONE folder, TWO asset types (defs + weightings), TWO
  resources (`crates/gdtf_app/src/states/load/systems/resolve/injuries.rs`).

Single FILES (not folders) load through the hot-RON chain instead:
the authored situation (`content/situations/skirmish.ron`) and the
`assets/core_tuning/*.tuning.ron` tables — same hot-reload discipline, one
file per chain, with a fallback so a bad file never strands `Load`.

## Part 5 — Verify

- **Suite:** `cargo dtest`. Every family binds the ONE generic load suite
  (`crates/gdtf_app/tests/load_suite/`) through a thin
  `FamilyLoadContract` wrapper — one file per family
  (`crates/gdtf_app/tests/load_weapons.rs`, `load_melee_weapons.rs`,
  `load_armor.rs`, `load_fields.rs`, `load_gangs.rs`, `load_attachments.rs`,
  `load_terrain.rs`, `load_themes.rs`, `load_sprites.rs`) pinning: the headless
  `MinimalPlugins` no-op + fallback seed, the Load→Intro gate on the registry,
  and the real-asset folder resolve with the shipped member stems
  (value-agnostic — presence, never magnitudes). A NEW family adds its own
  thin wrapper.
- **Hot-reload:** `cargo drun`, edit a member `.ron`, watch the
  "hot-reload: rebuilt … from …" info line name the registry and folder.
- **Reference integrity:** dangling cross-family keys surface on the
  consolidated end-of-`Load` report ([reference-integrity.md](reference-integrity.md)).
