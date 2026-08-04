# Testing — the cargo suite

The combat model is render-free by design (the model/view split is recorded in [ADR 0001](decisions/0001-rust-bevy-rewrite.md)), so its rules are pinned by a headless Rust unit suite — no Bevy `App`, no rendering, no scene. The suite is the regression net for the math: tuning is a data edit, but the *forms* (formulas, clearance rules, event contracts) must not drift silently.

## The ONE definition of green

This is the suite the gate, `/land`, and [verification.md](../.claude/rules/verification.md) all run. (The pre-commit hook runs the first four — it skips the two `cargo doc` steps, which are a dev/gate check rather than a commit check.) Run from the repo root; **green = ALL SIX pass**:

```bash
cargo fmt --check
cargo dclippy -- -D warnings
cargo dtest
cargo dbuild
cargo doc --workspace --no-deps
cargo doc-full
```

`dclippy` / `dtest` / `dbuild` / `doc-full` are the `.cargo/config.toml` aliases for the dynamic-linked dev/gate forms. The three workspace-wide ones share ONE feature list — `grimdark_turfwar/dynamic_linking,grimdark_turfwar/dev_tools,grimdark_turfwar/dev_tools` — so `dclippy` = `clippy --workspace --all-targets --features <that list>`, `dtest` = `test --workspace --features <that list>`; `dbuild` = `build -p grimdark_turfwar --features dynamic_linking,file_watcher,dev_tools`; `doc-full` = `doc --workspace --no-deps --features grimdark_turfwar/dev_tools,grimdark_turfwar/dev_tools` (no `dynamic_linking` — rustdoc never links the binary). The list names `net_qa` TWICE on purpose: a package-qualified feature turns on only that package's, and the game and the editor own separate `net_qa` features (`gdtf_app::dev::net_qa` vs `gdtf_content_editor::net_qa`). Both must be named — if only the game's is listed, the editor's whole `src/net_qa/` module goes uncompiled, unlinted, undoc-checked and untested. `dev_tools` is the procgen load-time stepper's `bevy_egui` dependency — folded into every dev alias so the dev/gate loop always compiles, lints, and tests the stepper's drive/gate/command/summary logic (the non-egui majority of it, including the pure stage-summary formatter, split into its own `summary` module specifically so it stays unit-tested rather than riding behind the panel's exclusion); only the egui draw closure itself (`ui.rs`'s `draw_stepper_panel`) needs a primary window, so it is compiled only by `dbuild` (the same screenshot-QA-territory carve-out as the content editor's own egui closure — see `.claude/rules/bevy-traps.md` #8). The whole affordance stays disengaged at runtime until a developer turns on the dev-only procgen-stepper toggle on the Options screen, which defaults to OFF and is only present in a `dev_tools` build (there is no environment variable). `dbuild` is the only step that builds + links the actual game binary (`check`/`clippy` never link; `test` links only test binaries). **`--all-features` is deliberately NOT used** — it forces a second full bevy build for no lint gain. `cargo doc` is dev/gate-only: the workspace `broken_intra_doc_links = "deny"` lint surfaces only under `cargo doc`, and it runs twice — the default-feature `cargo doc --workspace --no-deps` plus `cargo doc-full`, which enables the `dev_tools` / `net_qa`-gated modules so their doc comments are link-checked too. CI green is the static subset (no `dynamic_linking`, no `dev_tools`) but it DOES name both `net_qa` features: `cargo fmt --check`, `cargo clippy --workspace --all-targets --features grimdark_turfwar/dev_tools -- -D warnings`, `cargo test --workspace --features grimdark_turfwar/dev_tools` (`.github/workflows/clippy.yml` and `test.yml`), so an editor-QA regression fails a PR instead of going unlinted and untested.

The workspace `Cargo.toml` denies clippy `all` / `pedantic` / `correctness` / `suspicious` plus the restriction lints (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`) and `missing_docs`, so being **fmt-clean and lint-clean IS part of "done"**, not a separate nicety. Run it yourself — never report a remembered or assumed result.

(`cargo nextest run` is an optional faster swap for the test step; default to `cargo dtest`.)

> Every behavioral change MUST add tests that exercise the **real** code path — never a stub or a shadow copy of the unit under test — so the suite grows with the code.

## Suite layout

Two homes, standard Cargo, placed per the module-layout test convention ([module-layout.md](../.claude/rules/module-layout.md), rule 5):

- **In-crate unit tests** — a sibling `test/` directory (or `test.rs` leaf) next to the module under test (an inline `#[cfg(test)] mod test` only while tiny), for white-box coverage of private helpers and tight math. This is where most `gdtf_battle_sim` coverage lives.
- **Integration tests** — `crates/<crate>/tests/`, one file per system or focused concern (`tests/<suite>/main.rs` dir-form once a suite outgrows one file), exercising the crate's public surface as a downstream user would (e.g. constructing a battle and driving a volley through the public verbs).

Three repo-wide **guard suites** live in `crates/gdtf_test_utils/tests/` and ride every `cargo dtest` run:

- **`module_layout`** — the clause-7 module-layout conformance guard: wiring-only `mod.rs`, the warn>300 / block>400 line bands, and the exemption registry.
- **`assets_tree_clean`** — the tracked `assets/` tree must be git-clean when the suite runs, so a test that mutates shipped authored content is caught loudly instead of silently corrupting authored work.
- **`binary_feature_passthrough`** — a binary package under `bins/` must declare `net_qa = ["<lib>/net_qa"]` for every path-dependency library that declares a `net_qa` feature, so a QA control channel can never again be reachable from a `--features <lib>/net_qa` workspace check but not from a launchable binary.

## Conventions

- **Idiomatic, warning-free Rust.** Test code lives under the same deny-by-default workspace lints as production code — no `unwrap`/`expect`/`panic!` slipping in via tests either; prefer `assert!`/`assert_eq!` and `?` on `Result`-returning helpers. fmt and clippy gate the tests too (`--all-targets`).
- **Determinism via injected, seeded RNG.** Anything random takes an RNG by parameter (e.g. `&mut impl rand::Rng`); a sim test seeds it by constructing a production stream resource — `ShotRng::from_root(BattleSeed::new(seed))` (or `SeverityRng::from_root(…)`, etc.) — and drawing through its `.rng()` handle, then asserts exact values or distributions (e.g. the body-part roll's weighting; see `crates/gdtf_battle_sim/src/damage_resolution/hit_location/test.rs`). The per-stream `rand_chacha::ChaCha12Rng` streams replace `rand::rngs::StdRng` (documented non-portable). Never a global/thread RNG. Same-seed-same-stream is itself a pinned property.
- **Model tests need no Bevy.** `gdtf_battle_sim` is plain Rust on data — construct the battle value directly (with coded-default tuning and a seeded RNG) and assert. **No Bevy `App`, no `World`, no plugins, no rendering.** This is the whole point of keeping the sim Bevy-free (see the model/view split in [ADR 0001](decisions/0001-rust-bevy-rewrite.md)).
- **Presenter / scene / state tests** need Bevy. The standard way to step a minimal `App` is the headless harness `gdtf_test_utils::GdtfTestAppBuilder`: it wires the real GDTF state stack onto a deterministic `MinimalPlugins` app, so you `GdtfTestAppBuilder::new().starting_in(AppState::…).build()`, drive it with `app.update()` (or `gdtf_test_utils::advance_until`), and assert on `State<…>` / the `World`. Scene / state / app **behavioral** logic — transitions, `OnEnter`/`OnExit` wiring, a system's effect on the `World` — is now a *required* headless integration test, not an optional one. Stub only true externals; never shadow the unit under test. Running the app (`cargo run -p grimdark_turfwar`) and observing behavior (the app can capture its own screenshot) is reserved for the genuinely unautomatable: actual rendering / visual correctness, real input, font / layout — and richer automation of *those* is **TBD (Bevy harness)**.
- **Guard coverage is deliberate.** Null/empty tuning, malformed arrays, out-of-bounds cells, degenerate directions — each hardening rule carries an assert, so a refactor can't quietly drop a guard.

## The `GDTF_*` dev flags

Every dev / QA / test affordance in the workspace is opted into by a `GDTF_*` environment
variable. They are ALL non-shipping: each is additionally gated on a debug build and/or a
dev-only cargo feature at its wiring site, so a release binary ignores the lot.

The old env-var battle capture/drive rig (`GDTF_AUTOBATTLE`, capture paths, fire-at-frame flags)
was deleted, not deprecated. The one drive path now is the `net_qa` loopback QA network control
channel (`GDTF_NET_QA`): a coding-agent QA harness (`gdtf_qa_mcp`, or any client speaking
`gdtf_qa_protocol`) launches the game, negotiates the protocol version, reads the command list
that build publishes, runs one by name, and stops it — no env-var-scripted battle drive left in
the workspace. The request vocabulary is three commands — `Hello`, `Catalogue`, `Run` — so WHAT
the game can be asked to do is its command list, read at run time. That list is being rebuilt
one command at a time and today holds `app.phase` alone; adding to it is
[tooling/qa-commands.md](tooling/qa-commands.md), and driving it is
[tooling/agent-qa.md](tooling/agent-qa.md). Census command (run from the repo root; re-run it
when adding a flag and keep this table in step):

```bash
grep -rhoE 'GDTF_[A-Z_0-9]+' crates/ bins/ --include='*.rs' | sort -u
```

| Variable | Owner crate | Effect |
| --- | --- | --- |
| `GDTF_ASSETS_CLEAN_ROOT` | `gdtf_test_utils` | Overrides the repo root the assets-tree-clean guard test scans (guard-test hook). |
| `GDTF_BATTLE_SEED` | `gdtf_app` (consumed by `gdtf_battle_sim`; honored by the `gdtf_test_utils` battle harness) | Pins the root battle RNG seed for a reproducible replay; unset = wall-clock entropy, logged at `info!`. |
| `GDTF_BIN_PASSTHROUGH_ROOT` | `gdtf_test_utils` | Overrides the repo root the binary-feature-passthrough guard test scans (guard-test hook). |
| `GDTF_DEBUG_REACHABLE_OVERLAY` | `gdtf_battle_presenter` (mirrored by `gdtf_battle_input` docs) | Truthy renders the reachable-range debug overlay in a debug build (default off — visual noise). |
| `GDTF_EDITOR_ATTACHMENT` | `gdtf_content_editor` | Content-editor capture: pre-load a NAMED attachment item (a registry key / file stem, e.g. `scoped_sight`) into the ATTACHMENT form before the shot; unset keeps the sorted-first autoload. |
| `GDTF_EDITOR_MELEE_WEAPON` | `gdtf_content_editor` | Content-editor capture: pre-load a NAMED melee weapon (a registry key / file stem, e.g. `chainsword`) into the MELEE form before the shot; unset keeps the sorted-first autoload. |
| `GDTF_EDITOR_MODE` | `gdtf_content_editor` | Content-editor capture: force the Workbench mode (`terrain`/`theme`/`prefab`/`gang`/`armor`/`injury`/`sprite`/`attachment`/`weapon`/`melee_weapon`) before the shot. |
| `GDTF_EDITOR_NET_QA` | `gdtf_content_editor` | Truthy (`1`/`true`/`yes`/`on`) opts a `net_qa` debug EDITOR build into the editor's own loopback QA control channel. Distinct from the game's `GDTF_NET_QA` on purpose — one variable per host, so both can run at once. Launch recipe is in [tooling/agent-qa.md](tooling/agent-qa.md). |
| `GDTF_EDITOR_NET_QA_PORT` | `gdtf_content_editor` | Picks the EDITOR's `net_qa` loopback listen port; unset or unparseable = `7617` (one above the game's `7616`, so the two hosts never contend for a socket). Port only — the interface is always `Ipv4Addr::LOCALHOST`. |
| `GDTF_EDITOR_SHOT` | `gdtf_content_editor` | Content-editor capture: output PNG path; setting it opts the standalone editor into screenshot-then-exit. Not part of the `net_qa` migration. |
| `GDTF_EDITOR_TERRAIN_KIND` | `gdtf_content_editor` | Content-editor capture: pre-select the TERRAIN form's kind segment (`wall`/`cover`/`slab`/`emplacement`). |
| `GDTF_EDITOR_VIEW` | `gdtf_content_editor` | Content-editor capture: `full` forces the prefab-viewport full view (lifting the Isolate default); `isolate` stages the three-class Isolate shot. Unset keeps the editor defaults (Isolate on, one onion below). |
| `GDTF_EDITOR_WEAPON` | `gdtf_content_editor` | Content-editor capture: pre-load a NAMED weapon (a registry key / file stem, e.g. `heavy_bolter`) into the WEAPON form before the shot; unset keeps the sorted-first autoload. |
| `GDTF_EDITOR_ZOOM` | `gdtf_content_editor` | Content-editor capture: force a non-`1.0` prefab canvas zoom (`0.25`–`4.0`, clamped). |
| `GDTF_LOADING_SHOT` | `gdtf_app` | Loading-screen capture: output PNG path (rides the `net_qa` debug feature gate). |
| `GDTF_MODULE_LAYOUT_ROOT` | `gdtf_test_utils` | Overrides the repo root the module-layout conformance guard test scans (guard-test hook). |
| `GDTF_NET_QA` | `gdtf_app` | Truthy (`1`/`true`/`yes`/`on`) opts a `net_qa` debug build into the loopback QA network control channel — the ONE drive path (`launch` → `commands` → `run` → `stop`, driven via `gdtf_qa_mcp` or any `gdtf_qa_protocol` client). |
| `GDTF_NET_QA_PORT` | `gdtf_app` | Picks the `net_qa` loopback listen port; unset = the default. Port only — the interface is always `Ipv4Addr::LOCALHOST`. |
| `GDTF_SCREENSHOT_TEST_DEFINITELY_UNSET_VAR` | `gdtf_screenshot` | Test-only sentinel: a deliberately-never-set name proving `from_env` stays inert when its var is unset. Never set it. |
| `GDTF_TEST_FORCE_NO_GPU` | `gdtf_test_utils` | Forces the GPU-adapter probe to report Absent, driving the exact no-GPU skip path on a GPU machine. |

## What the suite pins (and what it doesn't)

It pins **behavioral contracts**: the march's exact cell walk, the strictly-higher-sails-over clearance rule, the per-hit damage formula, the severity bucket edges' *application* (not their magnitudes), event-emission discipline, the no-target-stop rule, dead-centre at cone 0, and the id-not-value boundary between sim and presenter. It does **not** pin tuning magnitudes — those live in tuning-config data and are expected to change. It does **not** assert on rendered pixels or exact `Transform` values beyond what a behavioral contract requires.
