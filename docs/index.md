# Design

Pointer index for the game's design and engineering docs. **GDTF (GrimDark TurF war)** — a turn-based tactics *situation generator* (Necromunda campaign × XCOM), built in Rust + Bevy.

- [pillars/](pillars/index.md) — the thesis, the core loop, the 8 design pillars (one file each), and the two layers.
- [litmus-tests.md](litmus-tests.md) — quick questions to validate any design decision against the pillars.
- [glossary.md](glossary.md) — game vocabulary (required reading for code identifiers).

## MVP

- [mvp/mvp.md](mvp/mvp.md) — v0 scope, the defer list, and the bar that decides whether the game is real.
- [mvp/post-action.md](mvp/post-action.md) — post-battle injury carry (accepted) + XP / use-based advancement (proposed).
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
- [combat/morale.md](combat/morale.md) — Morale / Bottle, nerve effects (accepted), and the suppression pin with its break-away rule.

## Authoring

- [authoring/](authoring/index.md) — how to author EVERY data-driven surface: weapons (ranged + melee), armor, attachments, injuries, on-death effects, fields, terrain & themes, prefabs/situations/gangs, floating combat text, the combat log, the generic content-family loader, and the reference-integrity contract — plus pointers to the engineer-facing rustdoc recipes (contextual acts, FCT families, effect palettes, test harness, scene scaffolds).

## Engineering

- [architecture.md](architecture.md) — the model/view split (the render-free authoritative sim `gdtf_battle_sim`, the presenter that mirrors it, the one-way input → presenter → sim chain), the rule that a `can_*` / `*_refusal` gate in the sim is the only answer to whether an act may happen, the `bevy_ui` / `egui` boundary, and the `AppState` tree.
- [testing.md](testing.md) — the Rust test suite: how to run it, suite layout, conventions (injected seeded RNG, render-free model tests), and what it pins vs. what it deliberately doesn't.
- [ui-picking-arbitration.md](ui-picking-arbitration.md) — how a mouse press is arbitrated between the UI and the battle world: Bevy's UI picking backend is already installed by `UiPlugin`, how it coexists with the battle `cursor_over_ui` gate, the minimum arbitration rule ("one pointer position, one owner per frame"), the wiring spec for the picking rollout, and the keyboard / gamepad activation state.

## Tooling

- [tooling/agent-qa.md](tooling/agent-qa.md) — the agent QA control channel: how the MCP host launches the game, the debug-build gate that opens the channel, the five MCP tools an agent calls, and the JSON-RPC-stdio / framed-RON wire shape.
- [tooling/qa-commands.md](tooling/qa-commands.md) — how to add a QA command: the file, the one line in a host's list, and the test — written from `app.phase`, the simplest command the game publishes. It also records why the shape is what it is.
