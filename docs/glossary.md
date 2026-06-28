# Glossary (game vocabulary)

Necromunda vocabulary is the house style. **Code identifiers must follow it** — no generic terms (no "paddle"/"ball"/"unit"/"soldier") where a glossary term exists. In Rust these terms become `snake_case` for fields/functions/modules and `CamelCase` for types/components/enums (e.g. `Ganger`, `GangId`, `bottle_check`) — the *word* is fixed, the casing follows Rust convention.

| Term | Meaning |
| ------ | --------- |
| **Gang** | A faction / the player's roster as a unit. One gang is the **player faction** (the gang the human controls); every other fielded gang is an **enemy**. |
| **Ganger** | An individual combatant. *Not* "unit" or "soldier". Named, persistent, mortal. |
| **Out of the fight** | A ganger who is `Downed` or `Dead` — incapacitated, no longer counting toward keeping their gang in the battle (see [combat/wounds-and-roster.md](combat/wounds-and-roster.md) for the two-pool downing/death model). A gang is defeated when all its gangers are out of the fight; the player wins / loses on this (see the battle-outcome beat in [combat/combat.md](combat/combat.md)). |
| **Turf** | A controlled region on the geoscape. |
| **Grudge** | Recorded enmity between gangs/fighters with memory. |
| **Bottle** | A gang voluntarily routing/withdrawing from a fight (Necromunda morale). |
| **Injury** | A lasting condition rolled (location × severity) when a fighter takes a Wound; docks an attribute. The named condition + attribute dock (Injury Tables) is not yet built — today a Wound carries its tier + struck location and spends the Wounds pool. |
| **Damage / Armor Types** | The shared 7-type wheel — armor: Plated, Refractive, Flak, Void, Hazard, Reinforced, Ceramic; damage: Kinetic, Las, Plasma, Chem, Shock, Blast, Rend (see [combat/matchup.md](combat/matchup.md)). |
| **WeaponName** | A weapon's human-facing name (e.g. "autogun") — a newtype component on the armed ganger, taken from the `.weapon.ron` filename stem (also the registry key). UI-only; combat-math never reads it (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **FireMode** | A weapon's fire-mode **selector** — an authored `Vec` of the modes it offers (any subset of Single / Burst / Full, in order); a component on the armed ganger (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **FireModeSpec** | One fire mode's numbers — its `kind` (ModeKind), cone multiplier, TU%, and shot count; the per-mode entry inside a `FireMode` (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **ModeKind** | A fire mode's closed kind — `Single` / `Burst` / `Full`. Its `Display` is the human label ("single"/"burst"/"full-auto"); there is no stored name string (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **WeaponRegistry** | The name-keyed set of all authored weapons, loaded from `assets/content/weapons/` at battle setup; `setup_battle` resolves each ganger's weapon key against it (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **WeaponSpec** | The authored form of a weapon — the fields a `.weapon.ron` carries (stats + `fire_mode` list), deserialised and resolved into a `WeaponBundle` at setup (see [combat/weapons-and-armor.md](combat/weapons-and-armor.md)). |
| **TerrainName** | A terrain piece's registry key — the filename stem of its `.terrain.ron` file (e.g. `"deck_floor"` from `deck_floor.terrain.ron`). The `TerrainRegistry` keys specs by it; the combat path never reads it. Mirrors `WeaponName` / `ArmorName`. |
| **TerrainRegistry** | The name-keyed set of all authored terrain pieces, loaded from `assets/content/terrain/` at battle setup; future battle-grid setup will resolve each cell's `TerrainName` against it (GTW-394; dormant until a downstream consumption ticket). Mirrors `WeaponRegistry` / `ArmorRegistry`. |
| **TerrainSpec** | The authored form of a terrain piece — the fields a `.terrain.ron` carries: a graphic key, a footfall sound key, and a `kind` payload (`Floor` / `Wall` / `Cover` / `Scatter` / `Slab`) carrying the kind-specific stats (move cost for floors; HP + armor + height band for Wall/Cover/Scatter; HP + armor for Slab). Deserialised into the `TerrainRegistry` by the `Load` flow (GTW-394). |
| **Visible** | A (cell, level) some conscious squad fighter currently sees — the union of the squad's eyes, the only tier the player may target into (see [combat/visibility.md](combat/visibility.md)). |
| **Explored** | A (cell, level) seen at some point this mission but not currently — mission memory, rendered as live terrain dimmed; never shrinks. |
| **Unseen** | A (cell, level) never seen this mission — fully hidden, reveals nothing (not even walkability). |
| **HiveScape** | The strategic layer — managing the gang and its turf in the hive between fights (the campaign / turf-war layer; the project's name for the "geoscape"-equivalent). Sub-state `GameState::HiveScape`. |
| **BattleScape** | The tactical layer — a single fight resolved on the battle grid. Sub-state `GameState::BattleScape`, with its own `BattleScapeState` flow: generation -> animate-in -> running -> animate-out -> aftermath. |
| **AfterMath** | The post-fight phase that surfaces a battle's results and consequences on the survivors before returning to the HiveScape (pillar: every fight leaves a mark). Sub-state `BattleScapeState::AfterMath`, with its own `AfterMathState` flow. |
| **Armor pierced** | The FCT/combat-log verdict when a hit's penetrating damage is `> 0` — the round punched through the target's armor. Displayed in neutral GREY (a status note, not a damage number). Contrast: **Armor held**. |
| **Armor held** | The FCT/combat-log verdict when a hit's penetrating damage is `<= 0` — the target's armor absorbed the round. Displayed in AMBER (a favorable defensive outcome). Contrast: **Armor pierced**. |

**TBD (design):** the Serious Injury table entries and any meta-currency name.
