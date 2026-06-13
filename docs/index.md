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
- [combat/battle-space.md](combat/battle-space.md) — the unified px metric the shot pipeline flies in: one unit on all three axes, the two constants (cell pitch, storey height), the z-axis datums, and how the view projects it.
- [combat/visibility.md](combat/visibility.md) — squad fog-of-war: the three states (Visible / Explored / Unseen), the per-ganger FOV and squad union, asymmetric sight, rendered-only planning, pay-per-step TUs, and the fog/slice composition.
- [combat/stats.md](combat/stats.md) — ganger stats: direct attributes, computed combat stats, the HP×Wounds damage model, Time Units.
- [combat/matchup.md](combat/matchup.md) — the 7-type weapon/armor matchup system (the gameplay rules).
- [combat/two-paradox-tournament.md](combat/two-paradox-tournament.md) — the underlying two-paradox / Paley math and how it maps to game systems.
- [combat/weapons-and-armor.md](combat/weapons-and-armor.md) — weapon & armor stats, the per-hit damage/penetration formula, and how the matchup wheel hooks in.
- [combat/wounds-and-roster.md](combat/wounds-and-roster.md) — the wound table and roster persistence (the heart of the generator).

## Engineering

- [architecture.md](architecture.md) — the model / view split: the render-free authoritative sim (`gdtf_battle_sim`), the mirroring battle presenter (`gdtf_battle_presenter`), id-carrying events, and the fire-flow division of labor.
- [testing.md](testing.md) — the Rust test suite: how to run it, suite layout, conventions (injected seeded RNG, render-free model tests), and what it pins vs. what it deliberately doesn't.
- [decisions/](decisions/index.md) — architecture decision records (ADRs): the why behind the structural and engine choices.
