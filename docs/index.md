# Design

Pointer index for the game's design and engineering docs. **GDTF (GrimDark TurF war)** — a turn-based tactics *situation generator* (Necromunda campaign × XCOM), built in Rust + Bevy.

- [pillars/](pillars/index.md) — the thesis, the core loop, the 8 design pillars (one file each), and the two layers.
- [litmus-tests.md](litmus-tests.md) — quick questions to validate any design decision against the pillars.
- [glossary.md](glossary.md) — game vocabulary (required reading for code identifiers).

## MVP

- [mvp/mvp.md](mvp/mvp.md) — v0 scope, the defer list, and the bar that decides whether the game is real.
- [mvp/campaign.md](mvp/campaign.md) — the deferred post-MVP strategic layer (hex geoscape, grudges, turf, economy).

## Combat

- [combat/combat.md](combat/combat.md) — battlescape rules: grid, arena sizes, Time Units, cover, line of sight.
- [combat/resolution.md](combat/resolution.md) — the full attack pipeline: dispersion accuracy, projectile travel, destructible cover, hit-location, melee, reaction fire, bleed-out.
- [combat/battle-space.md](combat/battle-space.md) — the cubic-voxel sim metric the shot pipeline flies in: one sim unit on all three axes (cell = cell = level), the 60×60×8 grid, the level-fraction / band z-datums, and how the presenter projects it to pixels.
- [combat/visibility.md](combat/visibility.md) — squad fog-of-war: the three states (Visible / Explored / Unseen), the per-ganger FOV and squad union, asymmetric sight, rendered-only planning, pay-per-step TUs, and the fog/slice composition.
- [combat/stats.md](combat/stats.md) — ganger stats: direct attributes, computed combat stats, the HP×Wounds damage model, Time Units.
- [combat/matchup.md](combat/matchup.md) — the 7-type weapon/armor matchup system (the gameplay rules).
- [combat/two-paradox-tournament.md](combat/two-paradox-tournament.md) — the underlying two-paradox / Paley math and how it maps to game systems.
- [combat/weapons-and-armor.md](combat/weapons-and-armor.md) — weapon & armor stats, the per-hit damage/penetration formula, and how the matchup wheel hooks in.
- [combat/wounds-and-roster.md](combat/wounds-and-roster.md) — the wound table and roster persistence (the heart of the generator).

## Authoring

- [authoring/injury-authoring.md](authoring/injury-authoring.md) — step-by-step guide: creating a new `.injury.ron`, the weighting table, how a per-side `BodyPart` maps to the category pool, how to add a new `InjuryEffect` variant end-to-end, and worked examples (`DisableHand` / `MovementCostMul`).
- [authoring/weapon-authoring.md](authoring/weapon-authoring.md) — step-by-step guide: creating a new ranged `.weapon.ron` (`assets/content/weapons/ranged/<key>.weapon.ron`), the full `WeaponSpec` field schema (the guide's field table mirrors `crates/gdtf_battle_sim/src/equipment/weapon/spec.rs` 1:1 — that file is the authoritative field list), fire-mode authoring, and how to extend the weapon model end-to-end. Melee weapons (`assets/content/weapons/melee/<key>.melee_weapon.ron` — the shared damage model + melee-only `reach` / `fight_mode`, GTW-505) are covered in [combat/weapons-and-armor.md](combat/weapons-and-armor.md) §Melee weapons.
- [authoring/armor-authoring.md](authoring/armor-authoring.md) — step-by-step guide: creating a new `.armor.ron` (`assets/content/armor/<key>.armor.ron`), the six per-location `ArmorPiece` fields (`floor`, `protection`, `integrity`, `hardness`, `armor_type`), the damage formula, and how to extend the armor model end-to-end.
- [authoring/contextual-act-recipe.md](authoring/contextual-act-recipe.md) — the add-one-contextual-act recipe (GTW-571): one descriptor module + one registration line per crate layer (sim `*Requested` + bespoke dispatch, input `ContextualAct` descriptor, app panel-button descriptor + offer scan), the Q5 drain invariant, and the named "AI arm, or documented why-not" station.
- [authoring/reference-integrity.md](authoring/reference-integrity.md) — the unified dangling-reference contract (GTW-582): every authored cross-file key is validated at the END of `Load` into ONE consolidated loud report (loud, never fatal — `Load` always exits); a malformed `.ron` fails per FILE (its well-formed siblings are salvaged), and the procgen nil-floor / empty-board fallbacks survive only as reported last resorts. Includes the full edge table and the two gang-path key schemes (gang = file stem, member = roster display-name).
- [authoring/terrain-authoring.md](authoring/terrain-authoring.md) — step-by-step guide to the UUID-keyed terrain model: the unified authored-asset roots (`assets/content/` for every data family, `assets/sprites/` for art), creating a new `.terrain_def.ron` under `assets/content/terrain/<theme>/` (payload-keyed by its `TerrainUuid`), the theme file (`<theme>.terrain_theme.ron` — palette + `default_floor`), the sim/presenter kind halves, tags, `on_death`, and how to extend the terrain model end-to-end (the `TerrainDef` schema in `crates/gdtf_battle_sim/src/terrain/def/` is the authoritative field list).

## Engineering

- [decisions/0001-rust-bevy-rewrite.md](decisions/0001-rust-bevy-rewrite.md) — the model / view split: the render-free authoritative sim (`gdtf_battle_sim`), the landed top-down 16×16 sprite battle presenter that mirrors it (`gdtf_battle_presenter`, with the iso renderer deferred behind `BattlePresenterMode`), the one-way `gdtf_battle_input → gdtf_battle_presenter → gdtf_battle_sim` chain, and the message-driven sim↔app boundary (recorded in the ADR's Decision / Consequences).
- [testing.md](testing.md) — the Rust test suite: how to run it, suite layout, conventions (injected seeded RNG, render-free model tests), and what it pins vs. what it deliberately doesn't.
- [decisions/](decisions/index.md) — architecture decision records (ADRs): the why behind the structural and engine choices.
