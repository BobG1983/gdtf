# Testing — the cargo suite

The combat model is render-free by design (the model/view split is recorded in [ADR 0001](decisions/0001-rust-bevy-rewrite.md)), so its rules are pinned by a headless Rust unit suite — no Bevy `App`, no rendering, no scene. The suite is the regression net for the math: tuning is a data edit, but the *forms* (formulas, clearance rules, event contracts) must not drift silently.

## The ONE definition of green

This is the same suite the gate, `/land`, the pre-commit hook, and [verification.md](../.claude/rules/verification.md) all run. Run from the repo root; **green = ALL FIVE pass**:

```bash
cargo fmt --check
cargo dclippy -- -D warnings
cargo dtest
cargo dbuild
cargo doc --workspace --no-deps
```

`dclippy` / `dtest` / `dbuild` are the `.cargo/config.toml` aliases for the dynamic-linked dev/gate forms — `clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking`, `test --workspace --features grimdark_turfwar/dynamic_linking`, and `build -p grimdark_turfwar --features dynamic_linking,file_watcher`. `dbuild` is the only step that builds + links the actual game binary (`check`/`clippy` never link; `test` links only test binaries). **`--all-features` is deliberately NOT used** — it forces a second full bevy build for no lint gain. `cargo doc` is dev/gate-only: the workspace `broken_intra_doc_links = "deny"` lint surfaces only under `cargo doc`. CI green is the static subset (no `dynamic_linking`): `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.

The workspace `Cargo.toml` denies clippy `all` / `pedantic` / `correctness` / `suspicious` plus the restriction lints (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`) and `missing_docs`, so being **fmt-clean and lint-clean IS part of "done"**, not a separate nicety. Run it yourself — never report a remembered or assumed result.

(`cargo nextest run` is an optional faster swap for the test step; default to `cargo dtest`.)

> Every behavioral ticket MUST add tests that exercise the **real** code path — never a stub or a shadow copy of the unit under test — so the suite grows with the code.

## Suite layout

Two homes, standard Cargo, placed per the module-layout test convention ([module-layout.md](../.claude/rules/module-layout.md), rule 5):

- **In-crate unit tests** — a sibling `test/` directory (or `test.rs` leaf) next to the module under test (an inline `#[cfg(test)] mod test` only while tiny), for white-box coverage of private helpers and tight math. This is where most `gdtf_battle_sim` coverage lives.
- **Integration tests** — `crates/<crate>/tests/`, one file per system or ticket-sized concern (`tests/<suite>/main.rs` dir-form once a suite outgrows one file), exercising the crate's public surface as a downstream user would (e.g. constructing a battle and driving a volley through the public verbs).

Three repo-wide **guard suites** live in `crates/gdtf_test_utils/tests/` and ride every `cargo dtest` run:

- **`module_layout`** — the clause-7 module-layout conformance guard: wiring-only `mod.rs`, the warn>300 / block>400 line bands, and the exemption registry.
- **`docs_path_truth`** — every repo path referenced from `docs/` and `.claude/rules/` must resolve against the live tree, so a module move can't silently strand design canon.
- **`assets_tree_clean`** — the tracked `assets/` tree must be git-clean when the suite runs, so a test that mutates shipped authored content is caught loudly instead of silently corrupting authored work.

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
| `GDTF_ASSETS_CLEAN_ROOT` | `gdtf_test_utils` | Overrides the repo root the assets-tree-clean guard test scans (guard-test hook). |
| `GDTF_AUTOBATTLE` | `gdtf_app` | Truthy (`1`/`true`/`yes`/`on`) drives a debug launch straight into a live battle (the GTW-223 QA affordance). |
| `GDTF_BATTLE_SEED` | `gdtf_app` (consumed by `gdtf_battle_sim`; honored by the `gdtf_test_utils` battle harness) | Pins the root battle RNG seed for a reproducible replay; unset = wall-clock entropy, logged at `info!`. |
| `GDTF_CAPTURE_FRAME` | `gdtf_app` | Battle capture: how many `BattleRunning` frames to wait before capturing ONE frame. |
| `GDTF_CAPTURE_FRAMES` | `gdtf_app` | Battle capture: comma-separated list of `BattleRunning` frames, one PNG each (wins over `GDTF_CAPTURE_FRAME`). |
| `GDTF_CAPTURE_PATH` | `gdtf_app` | Output PNG path; setting it (in a `dev_capture` debug build — `dynamic_linking` implies it since GTW-590) opts into the battle capture affordance. |
| `GDTF_DEBUG_REACHABLE_OVERLAY` | `gdtf_battle_presenter` (mirrored by `gdtf_battle_input` docs) | Truthy renders the reachable-range debug overlay in a debug build (GTW-450; default off — visual noise). |
| `GDTF_DOCS_PATH_ROOT` | `gdtf_test_utils` | Overrides the repo root the docs path-truth guard test scans (guard-test hook). |
| `GDTF_DROPDOWN_SHOT` | `gdtf_ui` | Dropdown demo example: capture a PNG to this path, then exit. |
| `GDTF_EDITOR_MODE` | `gdtf_content_editor` | Content-editor capture: force the Workbench mode (`terrain`/`theme`/`prefab`/`gang`/`armor`/`injury`/`sprite`) before the shot. |
| `GDTF_EDITOR_SHOT` | `gdtf_content_editor` | Content-editor capture: output PNG path; setting it opts the standalone editor into screenshot-then-exit. |
| `GDTF_EDITOR_TERRAIN_KIND` | `gdtf_content_editor` | Content-editor capture: pre-select the TERRAIN form's kind segment (`wall`/`cover`/`slab`/`emplacement`). |
| `GDTF_EDITOR_VIEW` | `gdtf_content_editor` | Content-editor capture: `full` forces the prefab-viewport full view (lifting the GTW-594 Isolate default, which wins over it); `isolate` stages the GTW-594 three-class Isolate shot (edit storey lifted to the painted upper storey). Unset keeps the editor defaults (Isolate on, one onion below). |
| `GDTF_EDITOR_ZOOM` | `gdtf_content_editor` | Content-editor capture: force a non-`1.0` prefab canvas zoom (`0.25`–`4.0`, clamped). |
| `GDTF_FALL_AT_FRAME` | `gdtf_app` | Battle capture trigger: the `BattleRunning` frame at which a player ganger is forced to FALL via the real fall path (fall-FX QA). |
| `GDTF_FIRE_AT_FRAME` | `gdtf_app` | Battle capture trigger: the `BattleRunning` frame at which the selected ganger fires at the nearest enemy via the real fire path. |
| `GDTF_FIRE_MODE` | `gdtf_app` | Battle capture trigger: fire-mode override for the fire trigger (`single`/`burst`/`full`). |
| `GDTF_LOADING_SHOT` | `gdtf_app` | Loading-screen capture: output PNG path (the GTW-419 capture hook). |
| `GDTF_MODULE_LAYOUT_ROOT` | `gdtf_test_utils` | Overrides the repo root the module-layout conformance guard test scans (guard-test hook). |
| `GDTF_PROCGEN_VIZ_SCREEN_SHOT` | `gdtf_app` | Procgen STEP/AUTO visualizer self-screenshot: output PNG path. |
| `GDTF_SCREENSHOT_TEST_DEFINITELY_UNSET_VAR` | `gdtf_screenshot` | Test-only sentinel: a deliberately-never-set name proving `from_env` stays inert when its var is unset. Never set it. |
| `GDTF_TEST_FORCE_NO_GPU` | `gdtf_test_utils` | Forces the GPU-adapter probe to report Absent, driving the exact no-GPU skip path on a GPU machine (GTW-527). |
| `GDTF_TEXTFIELD_SHOT` | `gdtf_ui` | Text-field demo example: capture a PNG to this path, then exit. |

## What the suite pins (and what it doesn't)

It pins **behavioral contracts**: the march's exact cell walk, the strictly-higher-sails-over clearance rule, the per-hit damage formula, the severity bucket edges' *application* (not their magnitudes), event-emission discipline, the no-target-stop rule, dead-centre at cone 0, and the id-not-value boundary between sim and presenter. It does **not** pin tuning magnitudes — those live in tuning-config data and are expected to change. It does **not** assert on rendered pixels or exact `Transform` values beyond what a behavioral contract requires.
