# Combat

The battlescape design canon — the authoritative, render-free combat model
(`gdtf_battle_sim`) and the rules + math the game resolves fights with. The
presenter (`gdtf_battle_presenter`) mirrors this; it never owns the rules.

- [combat.md](combat.md) — battlescape rules: grid, arena sizes, Time Units, cover, line of sight.
- [resolution.md](resolution.md) — the full attack pipeline: dispersion accuracy, projectile travel, destructible cover, hit-location, melee, reaction fire, bleed-out.
- [battle-space.md](battle-space.md) — the cubic-voxel sim metric the shot pipeline flies in: one sim unit on all three axes (cell = cell = level), the 60×60×8 grid, the level-fraction / band z-datums, and how the presenter projects it to pixels.
- [visibility.md](visibility.md) — squad fog-of-war: the three states (Visible / Explored / Unseen), per-ganger FOV and squad union, asymmetric sight, rendered-only planning, pay-per-step TUs, and the fog/slice composition.
- [stats.md](stats.md) — ganger stats: direct attributes, computed combat stats, the HP×Wounds damage model, Time Units.
- [matchup.md](matchup.md) — the 7-type weapon/armor matchup system (the gameplay rules).
- [two-paradox-tournament.md](two-paradox-tournament.md) — the underlying two-paradox / Paley math and how it maps to game systems.
- [weapons-and-armor.md](weapons-and-armor.md) — weapon & armor stats, the per-hit damage/penetration formula, and how the matchup wheel hooks in.
- [wounds-and-roster.md](wounds-and-roster.md) — the wound table and roster persistence (the heart of the generator).
- [morale.md](morale.md) — Morale / Bottle pools, nerve effects, Bottled (proposed; GTW-40).

See also: [../pillars/index.md](../pillars/index.md) · [../glossary.md](../glossary.md) · [../litmus-tests.md](../litmus-tests.md) · [../decisions/0001-rust-bevy-rewrite.md](../decisions/0001-rust-bevy-rewrite.md) (the model/view split) · [../authoring/injury-authoring.md](../authoring/injury-authoring.md) (injury content guide) · [../authoring/weapon-authoring.md](../authoring/weapon-authoring.md) (weapon content guide) · [../authoring/armor-authoring.md](../authoring/armor-authoring.md) (armor content guide) · [../authoring/terrain-authoring.md](../authoring/terrain-authoring.md) (terrain content guide).
