# Architecture — the model / view split

How the battle code is layered. One rule above all: **the sim is render-free and authoritative; the presenter mirrors it.** The model (`crates/gdtf_battle_sim`) is plain Rust on data — no Bevy `World`, no rendering, no iso math, no ECS scheduling — so every combat rule is headless-testable ([testing.md](testing.md)) with an injected seeded RNG. The view (`crates/gdtf_battle_presenter`) draws, animates, and gathers input as Bevy systems; it never owns a gameplay fact the model also holds.

This thesis is ported wholesale from the Godot original; the crate names and the engine primitives below are re-grounded onto Bevy 0.18. Where a piece is named here but not yet built, it is marked **TBD (Bevy):** rather than invented.

## The crates

| Crate | Role |
| ------- | ------ |
| `crates/gdtf_battle_sim` | the render-free authoritative combat sim — **the model** |
| `crates/gdtf_battle_presenter` | the Bevy view/presenter that mirrors the sim — **the view** |
| `crates/gdtf_app` | the Bevy `App`: `AppState` + scene-plugins (`states/app_state.rs`, `scenes/<scene>/plugin.rs`), the composition root |
| `bins/grimdark_turfwar` | the binary entry point (`GdtfApp::new().run()`) |

The model crate carries **no `bevy` dependency on its gameplay types** — it stays a plain-Rust library so the sim can be constructed and stepped in a unit test with no Bevy `App`. The presenter is a Bevy `Plugin`; the app composes the scene-plugins.

## The model — `crates/gdtf_battle_sim`

> **TBD (Bevy):** this crate today is a single doc-comment stub (`src/lib.rs`). The structure below is the design target ported from the Godot `src/combat`; each named type lands per ticket. Treat the names as the intended contract, not as existing code.

The logical battle is a plain Rust value (the Godot `BattleManager` `RefCounted`), built from a **situation** (the generated/authored battlefield: gangers, walls, scatter, upper-floor slabs, and stair/ladder vertical links — every placement an optional storey, position always the pair (cell, level)). It owns:

- one **ganger state** per ganger, keyed by an **integer id** = the ganger's index in the situation list. The state carries (cell, level), facing, stance, aiming, faction, the HP / Wounds / TU pools, life state, the wounds taken — and the battle-local **worn armor** (per-part copies of the roster armor, seeded at construction, that mid-battle wear mutates). The roster data is never touched.
- the **coarse occupancy** (a 3D grid, e.g. 60×60×8) — rebuilt **fresh per shot** from the situation's static terrain plus live ganger state: always correct by construction, no stale-sync seam. A destroyed-cover set keeps smashed walls/props from resurrecting on the rebuild.
- the **cover-HP ledger** — ONE ledger for walls *and* scatter props, keyed (cell, level), lazily seeded from each piece's max HP. Spending it destroys through one writer that feeds the destroyed-cover set, emits a `CoverDestroyed { cell, level }` event exactly once, and recomputes squad visibility.
- the **persistent surface grid** — floor/roof slabs + ground records are battle *state*, carried across every rebuild, so a destroyed slab stays destroyed and ground damage accrues.

Position uses `glam::IVec2` for the cell and a small integer for the level (Bevy re-exports `glam`); battle-space vectors are `glam::Vec3`. None of these touch screen/iso coordinates — that projection is the presenter's job.

**Model-authoritative facts** (the presenter carries none of these):

| Fact | Owner / writer |
| ------ | ---------------- |
| HP, Wounds, life (DOWNED/DEAD) | `apply_hit` — terminal gates: Wounds ≤ 0 → DEAD (trumps Downed), else HP ≤ 0 → DOWNED |
| Position — (cell, level); a storey change is legal only over an authored vertical link | the sim-driven move verb emits `id + cell + level`; the presenter tweens/reparents. A player-commit catch-up path lets the model follow a move the view already animated. |
| Stance | `set_stance` — the resolver reads live stance off the ganger state |
| TU pool + Aim Mode | the TU verbs (`spend_tu` / `reset_tu` / `can_spend_tu` / `set_aiming`), all gated on the ganger being active |
| Facing | view-driven but model-mirrored: every turn routes through `set_facing` so the zero-delta aim fallback never reads a stale spawn facing |
| Armor wear | `apply_hit` persists each hit's integrity onto the worn copy; a piece at ≤ 0 stops protecting and emits an armor-broken event once |
| Shot resolution | `resolve_coarse` — the whole coarse pipeline, returning a `ShotOutcome` |
| Cover HP — walls and props, one ledger keyed (cell, level) | `apply_cover_hit` — depletion destroys: destroyed-cover set + a single `CoverDestroyed` event + visibility recompute |
| Surfaces / ground | the surface-hit verb (slab destroyed at zero) / the ground-hit verb (damaged, never destroyed) |

### Events carry integer ids, never state references

In Godot these were signals; in Bevy they are **`Event`s written by the sim and read by presenter systems via `EventReader`**. Each event carries the **integer ganger id**, never a borrow of the ganger state: `GangerMoved`, `StanceChanged`, `TuChanged`, `AimingChanged`, `HpChanged`, `GangerWounded`, `ArmorBroken`, `GangerDowned`, `GangerDied`, `GroundDamaged { cell }`, `CoverDestroyed { cell, level }`. A presenter system maps the id back to its own entity (via the id→entity map below) without ever holding sim values.

> **TBD (Bevy):** the exact event flavor — Bevy `Event` (buffered, read in a system) vs. a returned report struct the presenter drains — is a per-ticket decision. The contract that holds either way: **ids cross the boundary, sim values do not.** A render-free sim cannot emit Bevy events while staying Bevy-free, so the likely shape is: the sim returns plain report values; a thin presenter-side adapter re-publishes them as Bevy `Event`s. Confirm at implementation time.

The one deliberate exception: a **`ShotOutcome` carries the resolved values themselves** (the struck ganger state / object / surface), because the consumer must *act* on exactly what the sim resolved, not re-resolve an ambiguous id. Ids ride along for logging only. Positions in the outcome are battle-space units, never screen coords.

Beside the manager sit the pure pieces: the combat math (static, deterministic, every coefficient from a tuning config value), the 3D DDA march, the part-roll verdict, the setup-time and per-shot occupancy pours (taking per-call arguments only), the vertical-link graph, and the data types (`GangerState`, `ShotOutcome`, `HitResult`, `Wound`, `Matchup`, …).

## The view — `crates/gdtf_battle_presenter`

> **TBD (Bevy):** this crate today is a doc-comment stub (`src/lib.rs`). The design below is the porting target; it lands as a Bevy `Plugin` exposing systems and components per ticket.

The presenter owns what is genuinely presentational or input-side, expressed as **Bevy ECS** rather than a Godot scene tree:

- the cell↔iso bridge — the *only* place a (cell, level) becomes a world `Transform`, level-aware (a storey lifts the world position). This is a presenter system/helper, never the model's.
- the prop/wall entity registry (presentation and reaction wiring only — cover HP lives in the model).
- the multi-level pathfinder (per-storey grids stitched by the authored links).
- the grid cursor, the **active view level** (the X-COM slice the updater applies), and turn **phases** (PLAYER ↔ ENEMY alternation, enemy-AI driver).
- the HUD.
- the **battle-seed resolution** — the composition root resolves the seed (an env pin for replay, randomized by default) and seeds the **model's** RNG, handed over once at setup.

### Firing a shot — the division of labor

The combat acts are **model acts** (fire economy, damage + severity application, the battle RNG, and cone composition all belong to the sim; the presenter keeps targeting flow, the player-only fog gate, and FX staging):

1. The presenter's **targeting flow** (input + cursor systems) gathers intent (target cell + level, fire mode) and gates it through the **player-only fog rule** ("unseen — hold your fire" — player policy, so it must NOT enter the shared act), then calls the model's one fire verb.
2. **`fire(shooter_id, mode, target_cell, target_level)`** on the sim owns the economy and the volley: it validates (the same guard set the HUD buttons read), charges TU (a burst clamped by ammo still charges the **full** mode TU), clamps the burst to ammo, and per round composes the cone (stance, brace off *model* cover, aim mode, burst index — the HUD stability readout reads the SAME method so panel and shot can't disagree), resolves the flight (muzzle, aim axis, recoil climb, cone sample, march, part roll), and applies the result as ONE act (armor → severity → application, with corpse-skip draw discipline), returning the **frozen per-round reports** in firing order. Every draw comes from the **sim-owned seeded RNG** (injected once at setup) — replay determinism is a model property, never an emergent property of view call order.
3. The presenter **consumes the reports**: FX staging (a playback system flies the round from the muzzle along the **sampled trajectory** to the projected exit/impact point), popups/flash off the report payload, recoil feedback.

### The bridge

The Godot `BattleSceneUpdater` bridge node becomes a set of **presenter systems plus a resource holding the id→entity map** (the view's hard rule: entities + integer ids only, never a sim value reference). Setup systems build the scene from the situation (floor, walls, scatter, gangers); reader systems consume the sim's id-events and drive each entity's visuals. Shot playback and fog-terrain presentation are their own carved-out systems. The battle-space → iso-world projection lives entirely on the presenter side.

When the presenter needs a model fact (TU affordability, aim flag), it does a **transient read** of the sim keyed by the entity's ganger id — read and drop, never stored in the entity.

## App shell — `crates/gdtf_app`

The two Godot app-wide autoloads (`App` runtime config, `SceneManager` scene routing) and Godot signals re-ground onto Bevy primitives:

- **Godot autoloads → Bevy `Resource`s + the `App`.** Cross-scene handoff (e.g. `last_battle_outcome`) is a `Resource` inserted on the owning `OnEnter`/removed on `OnExit` (per `bevy-traps.md` rule 1 there is no state-scoped resource — insert/remove explicitly and guard readers with `resource_exists` / `Option<Res<_>>`).
- **`SceneManager.goto` → Bevy `AppState` transitions.** Each scene is a `Plugin` registering `OnEnter(state)` / `OnExit(state)` systems. The state enum lives in `crates/gdtf_app/src/states/app_state.rs`:

  Init → Load → Intro → MainMenu → Playing → Teardown

  (defined in that order in `app_state.rs`; `Init` is `#[default]`). `crates/gdtf_app/src/scenes/<scene>/plugin.rs` is one plugin per state, all registered by `ScenesPlugin` (`scenes/plugin.rs`); `GdtfApp` (`app/gdtf_app.rs`) calls `init_state::<AppState>()` and adds `ScenesPlugin`.
- **Godot signals → Bevy `Event`s** read by systems via `EventReader` (see the model section). A thin per-domain event surface lets the HUD react to combat without combat knowing the HUD exists — the Godot `CombatEvents` autoload's role.

Flow today: `Init` advances to the next state once an init-complete marker resource exists, then on through the scenes. The scaffolded scene-plugins currently only print on enter/exit (`scenes/<scene>/systems/print_state.rs`) — the real per-scene content (main menu, the battle presenter mounting the sim) lands per ticket.

> **TBD (Bevy):** the battlescape mount point (which `AppState` hosts `gdtf_battle_presenter`, presumably `Playing`), the after-action report, and the campaign/geoscape layer are not yet built. The geoscape is a designed placeholder, as in the original.
> **TBD (Bevy):** the scaffold's `scenes/init/systems/move_on.rs` targets `AppState::Loading` and uses `.unwrap()`, which does not match the `app_state.rs` enum (`Load`) and violates the no-`unwrap` workspace lint. `app_state.rs` is treated as authoritative here; reconcile the scaffold against it under its own ticket.
