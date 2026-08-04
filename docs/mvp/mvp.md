# MVP — v0

**Goal: prove the core loop and nothing else.** Cut the geoscape entirely. The heart of the situation generator is:

fight  →  consequences on survivors  →  carry the scarred roster forward  →  fight again, changed

If that's fun, we have a game and the rest is layers. If it isn't, no amount of geoscape saves it.

> **Engine note.** This scope is engine-agnostic design; GDTF implements it in Rust + Bevy (ECS), with the authoritative combat sim in `gdtf_battle_sim` and the view in `gdtf_battle_presenter` (see the model/view split in [architecture.md](../architecture.md)). The original prototype proved several of these systems; GDTF re-implements them, so "shipped" status resets — items below note the *design intent* and where the Bevy implementation stands.

## v0 scope

- **One square-grid battlescape** with **working verticality** (multi-storey maps: authored upper-floor slabs, stair links, the 8-level coarse battle-space under the hit model, an X-COM hard-cut view slice — see [combat.md](../combat/combat.md#arena-size)), **up to 60×60 tiles** (common missions target 40–50 — see [combat.md](../combat/combat.md#arena-size)). The first shipped battle view is **DECIDED**: a **top-down 16×16 sprite renderer** drawn over a `TextureAtlasLayout` (the landed presenter, `gdtf_battle_presenter`), selected by the `BattlePresenterMode` enum. The **isometric** look (UFO:EU-style dimetric, fixed camera, pan/zoom, no rotation — see [combat.md](../combat/combat.md#presentation--settled)) stays the *intended eventual* projection, deferred behind that same enum as the alternate renderer. **TBD (Bevy):** the map asset format — the design wants a generated three-storey 60×60 situation (the grimdark `greybox_city` greybox) plus a hand-authored 12×12 fixture (the `greybox_tower`); the Bevy asset/loader representation is open.
- A **squad of named gangers** with stats (the grimdark fixtures: Brak/Krieg/Vex vs. the rusthounds). **TBD (Bevy):** the roster asset/loader — the design wants data-driven `Ganger` definitions; the Bevy representation (asset vs. resource vs. scene) is open.
- **Move + shoot on time units** (a **TU** economy, not AP), **cover**, **line of sight**, **damage**, **death**. There is **no to-hit %** — resolution is a sampled dispersion-cone trajectory ray-marched through the coarse occupancy grid (see [resolution.md](../combat/resolution.md)). This is the render-free sim core (`gdtf_battle_sim`) and the first thing to land with tests.
- The **7-type weapon/armor matchup** (see [matchup.md](../combat/matchup.md)) — this is cheap data and it's the part most likely to need tuning, so wire it early. The matchup multiplies **punch & shred** (favorable/neutral/resisted), not raw damage.
- On mission end, a **post-action screen** that:
  - **carries in-battle named injuries** onto the roster — no second injury-table roll (see [post-action.md](post-action.md); proposed),
  - **applies XP + minimal use-based attribute bumps** — [post-action.md](post-action.md) (proposed),
  - **persists the roster** — **not yet built** (battle wear is battle-scoped; cross-battle carry of the scarred roster is open work).

  The screen itself is a **placeholder** (outcome word + Continue back to the menu) and maps to the transition from the aftermath back out to the main-menu screen (`RunningState::Menu`, under `AppState::Running`) on the way to `AppState::Teardown`.
- **Two or three missions back-to-back** to prove the loop closes — the scarred roster from mission 1 walks into mission 2. **Not yet built** — the post-action screen returns to the menu; nothing carries.

## The bar

> Does walking a one-eyed, limping veteran into the next fight **feel different** and **generate a story**?

If yes — the engine works, build the layers. If no — stop and fix the loop before building anything else.

## Explicitly deferred (do NOT build in v0)

| Deferred | MVP stand-in |
| ---------- | -------------- |
| Geoscape / strategic map | none — missions run back-to-back from a menu |
| Procedural map generation | hand-author 1–2 maps |
| Grudge system | **just log who-killed-whom** as data; surface it later *(log not yet built)* |
| Turf / economy / income | none |
| Captures / rescue missions | none |
| Rubber-band / underdog comeback mechanics | none |

The whole reason to defer is sequencing risk: each deferred item multiplies complexity, and none of them matter if the fight→wound→roster loop isn't already fun.

## Where the risk actually is

Not art (Rohrer's point — being a non-artist is *aligned* with the strategy). The risk is **combat tuning**: a situation generator only generates interesting situations if the math never collapses into a dominant strategy or a degenerate loop. The matchup system ([matchup.md](../combat/matchup.md)) is the first line of defense — it's balanced by construction — but cover, positioning, the TU economy, and the wound layer all have to hold up too. Budget the tuning hyperfocus here.

## The honest filter

Before writing a line: would you actually play 200 hours of this campaign? The MVP exists to answer that as cheaply as possible.
