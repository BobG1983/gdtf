# Glossary (game vocabulary)

Necromunda vocabulary is the house style. **Code identifiers must follow it** — no generic terms (no "paddle"/"ball"/"unit"/"soldier") where a glossary term exists. In Rust these terms become `snake_case` for fields/functions/modules and `CamelCase` for types/components/enums (e.g. `Ganger`, `GangId`, `bottle_check`) — the *word* is fixed, the casing follows Rust convention.

| Term | Meaning |
| ------ | --------- |
| **Gang** | A faction / the player's roster as a unit. |
| **Ganger** | An individual combatant. *Not* "unit" or "soldier". Named, persistent, mortal. |
| **Turf** | A controlled region on the geoscape. |
| **Grudge** | Recorded enmity between gangs/fighters with memory. |
| **Bottle** | A gang voluntarily routing/withdrawing from a fight (Necromunda morale). |
| **Injury** | A lasting condition rolled (location × severity) when a fighter takes a Wound; docks an attribute. The named condition + attribute dock (Injury Tables) is not yet built — today a Wound carries its tier + struck location and spends the Wounds pool. |
| **Damage / Armor Types** | The shared 7-type wheel — armor: Plated, Refractive, Flak, Void, Hazard, Reinforced, Ceramic; damage: Kinetic, Las, Plasma, Chem, Shock, Blast, Rend (see [combat/matchup.md](combat/matchup.md)). |
| **Visible** | A (cell, level) some conscious squad fighter currently sees — the union of the squad's eyes, the only tier the player may target into (see [combat/visibility.md](combat/visibility.md)). |
| **Explored** | A (cell, level) seen at some point this mission but not currently — mission memory, rendered as live terrain dimmed; never shrinks. |
| **Unseen** | A (cell, level) never seen this mission — fully hidden, reveals nothing (not even walkability). |
| **HiveScape** | The strategic layer — managing the gang and its turf in the hive between fights (the campaign / turf-war layer; the project's name for the "geoscape"-equivalent). Sub-state `GameState::HiveScape`. |
| **BattleScape** | The tactical layer — a single fight resolved on the battle grid. Sub-state `GameState::BattleScape`, with its own `BattleScapeState` flow: generation -> animate-in -> running -> animate-out -> aftermath. |
| **AfterMath** | The post-fight phase that surfaces a battle's results and consequences on the survivors before returning to the HiveScape (pillar: every fight leaves a mark). Sub-state `BattleScapeState::AfterMath`, with its own `AfterMathState` flow. |

**TBD (design):** the Serious Injury table entries and any meta-currency name.
