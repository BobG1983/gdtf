# Authoring guides

How to author every data-driven surface in GDTF — one guide per content
family, each following the same shape: what it is, where the files live, the
RON schema BY EXAMPLE, how it registers/hot-reloads, and how to verify
in-editor/in-game. Two ground rules everywhere: authored magnitudes are tuning
DATA (guides show schema shape, never "correct" values), and every `.ron`
carries per-line comments.

Start here if you are adding a whole NEW family:
[content-families.md](content-families.md) — the one-line folder→registry
loader, its guarantees (per-file salvage, hot-reload, headless fallback,
validation window), and the one-owner path-spelling rule.

## Equipment

- [weapon-authoring.md](weapon-authoring.md) — ranged `.weapon.ron` (full
  `WeaponSpec` schema, fire modes, magazine/reload TU model, slots) and melee
  `.melee_weapon.ron` (shared damage model, `reach`/`fight_mode`, the fists
  default) — Part 5.
- [armor-authoring.md](armor-authoring.md) — `.armor.ron`: six per-location
  `ArmorPiece` records, the damage formula, and the `ArmorDamaged` /
  `ArmorBroken` wear-signal surface.
- [attachment-authoring.md](attachment-authoring.md) — `.attachment.ron`
  items: the typed effect vocabulary (magnitudes live ON the item), the
  closed slot model, and the palette/mechanics split.

## Consequences & effects

- [injury-authoring.md](injury-authoring.md) — `.injury.ron` +
  `.weighting.ron`: the ONE shared injury pool, per-context category
  weighting, and the add-an-`InjuryEffect` recipe.
- [on-death-authoring.md](on-death-authoring.md) — the `on_death:` field on
  weapons and terrain (`Explode` / `LeaveField`), the every-terminal-gate
  emission contract, and the cascade resolver.
- [field-authoring.md](field-authoring.md) — `.field.ron` area-damage zones:
  drain/immunity/duration, the three placement producers, the round tick.

## Battlefields

- [terrain-authoring.md](terrain-authoring.md) — the UUID-keyed terrain
  model: `.terrain_def.ron` pieces (sim/presenter kind halves, tags,
  `on_death`), the `.terrain_theme.ron` palette, and the
  extend-the-model steps.
- [battlefield-authoring.md](battlefield-authoring.md) — prefabs
  (`.prefab.ron` fragments procgen packs), the situation
  (`skirmish.ron` — theme + board + placed gangers + hazards), and gangs
  (`.gang.ron` rosters + the current in-game gang editor).
- [sprite-defs.md](sprite-defs.md) — `.spritedef.ron` sprite definitions
  (source file or sheet+rect, anchor, optional facings/animation): the
  catalog a terrain def's `graphic_name` foreign-keys into (renderer swap =
  ).
- [reference-integrity.md](reference-integrity.md) — the unified
  dangling-reference contract: every cross-file key validated at the end of
  `Load` into one loud, never-fatal report; per-file salvage; the editor
  re-arm.
- [terrain-art-and-destruction.md](terrain-art-and-destruction.md) — canon,
  decided but not built: a def owns its art in every state it can be in and
  says what it `leaves_behind` when it dies, the renderer owns no content, and
  `on_death` becomes a list.

## Combat text (the view side)

- [fct-authoring.md](fct-authoring.md) — floating combat text: the
  valence/severity color model and the add-a-consequence-family pointer.
- [combat-log-authoring.md](combat-log-authoring.md) — the combat log:
  sources → presenter-side forwarders → classify → the app appender, the
  all-state-changes coverage contract, and the hot-reloadable feel table.

## Engineer-facing recipes (rustdoc is the source of truth — link, never fork)

- [contextual-act-recipe.md](contextual-act-recipe.md) — adding a contextual
  act: one descriptor module + one registration line per crate layer.
- **FCT families** — module rustdoc of
  `crates/gdtf_battle_presenter/src/actors/fx/fct/families/mod.rs`.
- **Combat-log sources** — module rustdoc of
  `crates/gdtf_battle_presenter/src/actors/fx/fct/log_event/mod.rs`.
- **Content-family loader** — trait rustdoc in
  `crates/gdtf_assets/src/family/def.rs` and the glue-crate overview in
  `crates/gdtf_content_families/src/lib.rs`.
- **Effect palettes** — module rustdocs of
  `crates/gdtf_battle_sim/src/effects/attachments/effect.rs`,
  `crates/gdtf_battle_sim/src/effects/on_death/mod.rs`, and
  `crates/gdtf_battle_sim/src/effects/fields/mod.rs` (one file per effect,
  thin delegation enum).
- **Test authoring** — [testing.md](../testing.md) plus the headless-harness
  rustdoc in `crates/gdtf_test_utils/src/lib.rs`.
- **Scene scaffolds** — module rustdoc of
  `crates/gdtf_app/src/states/scaffold/mod.rs` (the four stamped system
  shapes a new scene plugin calls).

Render the rustdoc locally with `cargo doc --workspace --no-deps`.
