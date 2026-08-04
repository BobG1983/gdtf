# Glossary (game vocabulary)

Necromunda vocabulary is the house style. **Code identifiers must follow it** — no generic terms (no "paddle"/"ball"/"unit"/"soldier") where a glossary term exists. In Rust these terms become `snake_case` for fields/functions/modules and `CamelCase` for types/components/enums (e.g. `Ganger`, `GangName`, `bottle_check`) — the *word* is fixed, the casing follows Rust convention.

| Term | Meaning |
| ------ | --------- |
| **Gang** | A faction / the player's roster as a unit. One gang is the **player faction** (the gang the human controls); every other fielded gang is an **enemy**. |
| **Ganger** | An individual combatant. *Not* "unit" or "soldier". Named, persistent, mortal. |
| **Out of the fight** | A ganger who is `Downed`, `Dead`, or **`Bottled`** — no longer counting toward keeping their gang in the battle (body: [wounds-and-roster.md](combat/wounds-and-roster.md); mind: [morale.md](combat/morale.md)). A gang is defeated when all its gangers are out of the fight. |
| **Turf** | A controlled region on the geoscape. |
| **Grudge** | Recorded enmity between gangs/fighters with memory. |
| **Bottle** | (1) Psych life pool (psych Wounds) on a ganger — empty → **Bottled**. (2) Historical Necromunda sense: a side routing from a fight — **gang-wide bottle is not v0**. See [combat/morale.md](combat/morale.md). |
| **Morale** | Psych HP pool — stress buffer derived from Grit + Cool; damaged by shocks; not the same as suppression. See [combat/morale.md](combat/morale.md). |
| **Bottled** | Ganger terminal psych state: out of this fight (no acts), still on the roster after. Parallel to Downed/Dead on the body track. |
| **Nerve effect** | Named battle psych condition (hesitant, reckless, …) rolled from data when a shock bites — analogous to an Injury, for the mind. |
| **Injury** | A lasting named condition rolled (location × severity × source) when a fighter takes a non-graze Wound; docks attributes via the ledger. Rolled **in battle**; post-action carries the ledger (no second table roll for MVP — see [mvp/post-action.md](mvp/post-action.md)). |
| **Damage / Armor Types** | The shared 7-type wheel — armor: Plated, Refractive, Flak, Void, Hazard, Reinforced, Ceramic; damage: Kinetic, Las, Plasma, Chem, Shock, Blast, Rend (see [combat/matchup.md](combat/matchup.md)). |
| **WeaponName** | A weapon's human-facing name (e.g. "autogun") — a newtype component on the armed ganger, taken from the `.weapon.ron` filename stem (also the registry key). UI-only; combat-math never reads it (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **FireMode** | A weapon's fire-mode **selector** — an authored `Vec` of the modes it offers (any subset of Single / Burst / Full, in order); a component on the armed ganger (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **FireModeSpec** | One fire mode's numbers — its `kind` (ModeKind), cone multiplier, TU%, and shot count; the per-mode entry inside a `FireMode` (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **ModeKind** | A fire mode's closed kind — `Single` / `Burst` / `Full`. Its `Display` is the human label ("single"/"burst"/"full-auto"); there is no stored name string (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **WeaponRegistry** | The name-keyed set of all authored weapons, loaded from `assets/content/weapons/ranged/` at battle setup; `setup_battle` resolves each ganger's weapon key against it (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **WeaponSpec** | The authored form of a weapon — the fields a `.weapon.ron` carries (stats + `fire_mode` list), deserialised and resolved into a `WeaponBundle` at setup (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **TerrainDef** | The authored form of a terrain piece — the fields a `.terrain_def.ron` carries: a stable `TerrainUuid` key, a display name, a SIM half (`sim_kind`: `Wall` / `Cover` / `Slab` / `Emplacement` structural stats), a PRESENTER half (`presenter_kind`: graphic role key + optional slab footfall), optional sim-owned `tags`, and an optional `on_death` effect (see [authoring/terrain-authoring.md](authoring/terrain-authoring.md)). |
| **TerrainUuid** | A terrain def's stable UUID key — what themes, prefabs, and the registry reference a piece by (the file is payload-keyed; its filename is NOT the key). The theme mirror is **ThemeUuid**. |
| **TerrainDefRegistry** | The UUID-keyed set of all authored terrain defs (`TerrainUuid` → `TerrainDef`), folder-loaded from `assets/content/terrain/<theme>/` and resolved at battle setup. Mirrors `WeaponRegistry` / `ArmorRegistry`; its theme sibling is the `UuidThemeRegistry` (`ThemeUuid` → `UuidThemeDef`, the per-theme palette + `default_floor`). |
| **Visible** | A (cell, level) some conscious squad fighter currently sees — the union of the squad's eyes, the only tier the player may target into (see [combat/visibility.md](combat/visibility.md)). |
| **Explored** | A (cell, level) seen at some point this mission but not currently — mission memory, rendered as live terrain dimmed; never shrinks. |
| **Unseen** | A (cell, level) never seen this mission — fully hidden, reveals nothing (not even walkability). |
| **HiveScape** | The strategic layer — managing the gang and its turf in the hive between fights (the campaign / turf-war layer; the project's name for the "geoscape"-equivalent). Sub-state `GameState::HiveScape`. |
| **BattleScape** | The tactical layer — a single fight resolved on the battle grid. Sub-state `GameState::BattleScape`, with its own `BattleScapeState` flow: generation -> animate-in -> running -> animate-out -> aftermath. |
| **AfterMath** | The post-fight phase that surfaces a battle's results and consequences on the survivors before returning to the HiveScape (pillar: every fight leaves a mark). Sub-state `BattleScapeState::AfterMath`, with its own `AfterMathState` flow. |
| **Armor pierced** | The FCT/combat-log verdict when a hit's penetrating damage is `> 0` — the round punched through the target's armor. Displayed in neutral GREY (a status note, not a damage number). Contrast: **Armor held**. |
| **Armor held** | The FCT/combat-log verdict when a hit's penetrating damage is `<= 0` — the target's armor absorbed the round. Displayed in AMBER (a favorable defensive outcome). Contrast: **Armor pierced**. |

**TBD (design):** the Serious Injury table entries and any meta-currency name.
