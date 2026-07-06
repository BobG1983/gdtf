# Testing — the cargo suite

The combat model is render-free by design (the model/view split is recorded in [ADR 0001](decisions/0001-rust-bevy-rewrite.md)), so its rules are pinned by a headless Rust unit suite — no Bevy `App`, no rendering, no scene. The suite is the regression net for the math: tuning is a data edit, but the *forms* (formulas, clearance rules, event contracts) must not drift silently.

## The ONE definition of green

This is the same suite the gate, `/land`, the pre-commit hook, and [verification.md](../.claude/rules/verification.md) all run. Run from the repo root; **green = ALL THREE pass**:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

The workspace `Cargo.toml` denies clippy `all` / `pedantic` / `correctness` / `suspicious` plus the restriction lints (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`) and `missing_docs`, so being **fmt-clean and lint-clean IS part of "done"**, not a separate nicety. Run it yourself — never report a remembered or assumed result.

(`cargo nextest run --workspace` is an optional faster swap for the test step; default to `cargo test`.)

> **Zero tests today.** gdtf currently has no tests, so `cargo test --workspace` passes trivially. That is **NOT red** — there is no "nothing ran = red" rule here (do not port the Godot/GUT logic where an empty run meant failure). But every behavioral ticket MUST add tests that exercise the **real** code path, so the suite grows with the code.

## Suite layout

Two homes, standard Cargo:

- **In-crate unit tests** — `#[cfg(test)] mod tests { … }` inside the module under test, for white-box coverage of private helpers and tight math. This is where most `gdtf_battle_sim` coverage lives.
- **Integration tests** — `crates/<crate>/tests/*.rs`, one file per system or ticket-sized concern, exercising the crate's public surface as a downstream user would (e.g. constructing a battle and driving a volley through the public verbs).

When the design target from [ADR 0001](decisions/0001-rust-bevy-rewrite.md) lands, expect the suite to grow roughly by band (names ported from the Godot GUT suite, re-grounded to Rust modules):

- **Coarse pipeline geometry** — the 3D DDA march (hand-computed cell sequences, entry/exit `t`, z-crossings, grid-exit rules), clearance bands, corner-tie, occupancy, surfaces, height, the stance/cover matrix, build-from-situation, impact, pipeline integration.
- **Combat math** — cone vector, recoil climb, muzzle origin, shot origin, hit/matchup resolution, severity roll, math guards, tuning parity, battle-space metric, stance shot bands, shot outcome.
- **Manager contract** — the battle value's event + id discipline, `apply_hit` terminal gates, the coarse path, TU/aiming, armor degradation, wounds guard, stance-change TU, volley integration.
- **Presenter routing** — fire routing, outcome routing, burst, input gates, battle-over routing, enemy-AI fire, outcome marker, hit data, battle→world projection, pathfinder restore, prop registry, ganger facing.
- **Shell / misc** — battle seed, situation validation, the `AppState` transition guards, app outcome handoff, camera bounds.

> **TBD (Bevy):** these are the *target* test names; none exist yet. They land alongside the code they pin.

## Conventions

- **Idiomatic, warning-free Rust.** Test code lives under the same deny-by-default workspace lints as production code — no `unwrap`/`expect`/`panic!` slipping in via tests either; prefer `assert!`/`assert_eq!` and `?` on `Result`-returning helpers. fmt and clippy gate the tests too (`--all-targets`).
- **Determinism via injected, seeded RNG.** Anything random takes an RNG by parameter (e.g. `&mut impl rand::Rng`); a sim test seeds it by constructing a production stream resource — `ShotRng::from_root(BattleSeed::new(seed))` (or `SeverityRng::from_root(…)`, etc.) — and drawing through its `.rng()` handle, then asserts exact values or distributions (e.g. the body-part roll's weighting; see `crates/gdtf_battle_sim/src/damage_resolution/hit_location/test.rs`). The per-stream `rand_chacha::ChaCha12Rng` streams replace `rand::rngs::StdRng` (documented non-portable). Never a global/thread RNG. Same-seed-same-stream is itself a pinned property.
- **Model tests need no Bevy.** `gdtf_battle_sim` is plain Rust on data — construct the battle value directly (with coded-default tuning and a seeded RNG) and assert. **No Bevy `App`, no `World`, no plugins, no rendering.** This is the whole point of keeping the sim Bevy-free (see the model/view split in [ADR 0001](decisions/0001-rust-bevy-rewrite.md)).
- **Presenter / scene / state tests** need Bevy. The standard way to step a minimal `App` is the headless harness `gdtf_test_utils::GdtfTestAppBuilder`: it wires the real GDTF state stack onto a deterministic `MinimalPlugins` app, so you `GdtfTestAppBuilder::new().starting_in(AppState::…).build()`, drive it with `app.update()` (or `gdtf_test_utils::advance_until`), and assert on `State<…>` / the `World`. Scene / state / app **behavioral** logic — transitions, `OnEnter`/`OnExit` wiring, a system's effect on the `World` — is now a *required* headless integration test, not an optional one. Stub only true externals; never shadow the unit under test. Running the app (`cargo run -p grimdark_turfwar`) and observing behavior (the app can capture its own screenshot) is reserved for the genuinely unautomatable: actual rendering / visual correctness, real input, font / layout — and richer automation of *those* is **TBD (Bevy harness)**.
- **Guard coverage is deliberate.** Null/empty tuning, malformed arrays, out-of-bounds cells, degenerate directions — each hardening rule carries an assert, so a refactor can't quietly drop a guard.

## The `GDTF_*` dev flags

Every dev / QA / test affordance in the workspace is opted into by a `GDTF_*` environment
variable. They are ALL non-shipping: each is additionally gated on a debug build and/or a
dev-only cargo feature (`dev_capture`) at its wiring site, so a release binary ignores the
lot. Since GTW-590 the game binary's `dynamic_linking` dev feature folds `dev_capture` in,
so the standard dev invocation captures with no extra feature flag:

```bash
GDTF_AUTOBATTLE=1 GDTF_CAPTURE_PATH=/abs/out.png GDTF_CAPTURE_FRAMES="10,60,150,300" \
  cargo run -p grimdark_turfwar --features dynamic_linking
```

(a debug build WITHOUT `dynamic_linking` still ignores the capture vars — the affordance
rides the dev feature). Every capture run is LOUD: activation, per-frame trigger, and
per-frame write each log a `capture`-greppable line, and a set-but-ineffective
`GDTF_CAPTURE_*` / trigger var `warn!`s at startup. The capture-gated tests ride the same
fold — `cargo dtest` compiles + runs them; the targeted recipe is
`cargo test -p gdtf_app --features test-support,dev_capture --lib` (plus
`--test capture_quit` for the exit pin), so that combination can never be an invisible
red again. Census command (run from the repo root; re-run it when adding a flag and keep
this table in step):

```bash
grep -rhoE 'GDTF_[A-Z_0-9]+' crates/ bins/ --include='*.rs' | sort -u
```

| Variable | Owner crate | Effect |
| --- | --- | --- |
| `GDTF_ACCORDION_SHOT` | `gdtf_ui` | Accordion demo example: capture a PNG to this path, then exit. |
| `GDTF_AUTOBATTLE` | `gdtf_app` | Truthy (`1`/`true`/`yes`/`on`) drives a debug launch straight into a live battle (the GTW-223 QA affordance). |
| `GDTF_BATTLE_SEED` | `gdtf_app` (consumed by `gdtf_battle_sim`; honored by the `gdtf_test_utils` battle harness) | Pins the root battle RNG seed for a reproducible replay; unset = wall-clock entropy, logged at `info!`. |
| `GDTF_CAPTURE_FRAME` | `gdtf_app` | Battle capture: how many `BattleRunning` frames to wait before capturing ONE frame. |
| `GDTF_CAPTURE_FRAMES` | `gdtf_app` | Battle capture: comma-separated list of `BattleRunning` frames, one PNG each (wins over `GDTF_CAPTURE_FRAME`). |
| `GDTF_CAPTURE_PATH` | `gdtf_app` | Output PNG path; setting it (in a `dev_capture` debug build — `dynamic_linking` implies it since GTW-590) opts into the battle capture affordance. |
| `GDTF_DEBUG_REACHABLE_OVERLAY` | `gdtf_battle_presenter` (mirrored by `gdtf_battle_input` docs) | Truthy renders the reachable-range debug overlay in a debug build (GTW-450; default off — visual noise). |
| `GDTF_DOCS_PATH_ROOT` | `gdtf_test_utils` | Overrides the repo root the docs path-truth guard test scans (guard-test hook). |
| `GDTF_DROPDOWN_SHOT` | `gdtf_ui` | Dropdown demo example: capture a PNG to this path, then exit. |
| `GDTF_EDITOR_MODE` | `gdtf_content_editor` | Content-editor capture: force the Workbench mode (`terrain`/`theme`/`prefab`) before the shot. |
| `GDTF_EDITOR_SHOT` | `gdtf_content_editor` | Content-editor capture: output PNG path; setting it opts the standalone editor into screenshot-then-exit. |
| `GDTF_EDITOR_TERRAIN_KIND` | `gdtf_content_editor` | Content-editor capture: pre-select the TERRAIN form's kind segment (`wall`/`cover`/`slab`/`emplacement`). |
| `GDTF_EDITOR_VIEW` | `gdtf_content_editor` | Content-editor capture: `full` forces the prefab-viewport full view (default down-to-active). |
| `GDTF_EDITOR_ZOOM` | `gdtf_content_editor` | Content-editor capture: force a non-`1.0` prefab canvas zoom (`0.25`–`4.0`, clamped). |
| `GDTF_FALL_AT_FRAME` | `gdtf_app` | Battle capture trigger: the `BattleRunning` frame at which a player ganger is forced to FALL via the real fall path (fall-FX QA). |
| `GDTF_FIRE_AT_FRAME` | `gdtf_app` | Battle capture trigger: the `BattleRunning` frame at which the selected ganger fires at the nearest enemy via the real fire path. |
| `GDTF_FIRE_MODE` | `gdtf_app` | Battle capture trigger: fire-mode override for the fire trigger (`single`/`burst`/`full`). |
| `GDTF_GANG_EDITOR_SHOT` | `gdtf_app` | In-app GANG-editor scene self-screenshot: output PNG path (drives menu → gang editor → capture → quit). GTW-622 renamed it: the old spelling sat one token from the content editor's `GDTF_EDITOR_SHOT`, and setting the wrong one silently no-oped. |
| `GDTF_LOADING_SHOT` | `gdtf_app` | Loading-screen capture: output PNG path (the GTW-419 capture hook). |
| `GDTF_MODULE_LAYOUT_ROOT` | `gdtf_test_utils` | Overrides the repo root the module-layout conformance guard test scans (guard-test hook). |
| `GDTF_PROCGEN_VIZ_SCREEN_SHOT` | `gdtf_app` | Procgen STEP/AUTO visualizer self-screenshot: output PNG path. |
| `GDTF_SCREENSHOT_TEST_DEFINITELY_UNSET_VAR` | `gdtf_screenshot` | Test-only sentinel: a deliberately-never-set name proving `from_env` stays inert when its var is unset. Never set it. |
| `GDTF_SCROLL_SHOT` | `gdtf_ui` | Scroll-list demo example: capture a PNG to this path, then exit. |
| `GDTF_TEST_FORCE_NO_GPU` | `gdtf_test_utils` | Forces the GPU-adapter probe to report Absent, driving the exact no-GPU skip path on a GPU machine (GTW-527). |
| `GDTF_TEXTFIELD_SHOT` | `gdtf_ui` | Text-field demo example: capture a PNG to this path, then exit. |

## What the suite pins (and what it doesn't)

It pins **behavioral contracts**: the march's exact cell walk, the strictly-higher-sails-over clearance rule, the per-hit damage formula, the severity bucket edges' *application* (not their magnitudes), event-emission discipline, the no-target-stop rule, dead-centre at cone 0, and the id-not-value boundary between sim and presenter. It does **not** pin tuning magnitudes — those live in tuning-config data and are expected to change. It does **not** assert on rendered pixels or exact `Transform` values beyond what a behavioral contract requires.
