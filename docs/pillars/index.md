# Design Pillars

Source of truth for the game's identity. These are **locked** unless explicitly revised. Anything marked **TBD (design)** elsewhere is open work requiring a deliberate, signed-off decision before implementation.

## The thesis: a situation generator

The game is a *unique situation generator*, not a *consumable game* (Jason Rohrer, GDC 2019) — built to produce stories the player carries, not a single authored path to an endpoint. The lineage is direct: old Necromunda's campaign rules (persistent named gang, permadeath, Injuries, turf/income, XP/advancement, captures, grudges) are arguably the purest tabletop situation generator ever printed, and XCOM productized exactly that loop.

## The core loop (the engine of fun)

fight  →  consequences on the survivors  →  carry the scarred roster forward  →  fight again, changed

Everything else — geoscape, economy, grudges, procgen maps, verticality — is a **layer on top of this loop**. If the loop isn't fun, no layer saves it. Build and prove the loop first (see [../mvp/mvp.md](../mvp/mvp.md)).

## The pillars

1. [A situation generator, not a consumable](1-situation-generator.md) — produce stories the player carries, not an authored ending.
2. [The roster is the story](2-the-roster-is-the-story.md) — gangers earn identity from wounds, XP, and grudges.
3. [Every fight leaves a mark](3-every-fight-leaves-a-mark.md) — consequences are permanent and compound; a won fight can still cost you.
4. [Permanent stakes make decisions matter](4-permanent-stakes.md) — tension from consequence, not a clock.
5. [Readability over fidelity](5-readability-over-fidelity.md) — compete on legible consequence, not production values.
6. [Matchups are modifiers, never auto-wins](6-matchups-are-modifiers.md) — counters felt but never decisive.
7. [Balanced by construction](7-balanced-by-construction.md) — structure the math so no dominant strategy emerges.
8. [Memory is the moat](8-grudges-with-memory.md) — gangs that remember who crippled whom; the long-game differentiator, and it's data not art.

See also: [litmus-tests.md](../litmus-tests.md) — quick questions to validate any design decision.

## Two layers

| Layer | Grid | Status |
| ------- | ------ | -------- |
| **Battlescape** (TBS combat) | **Square** — cover, blast radii, building footprints, destructible terrain, and the whole genre's muscle memory assume square. | MVP |
| **Geoscape** (turf war) | **Hex** — in a turf war, adjacency *is* the conflict; hex gives six clean equal neighbours with no diagonal ambiguity. A Necromunda underhive turf map (Civ-lite), not an XCOM globe. Procgen per campaign. | Deferred — see [../mvp/campaign.md](../mvp/campaign.md) |
