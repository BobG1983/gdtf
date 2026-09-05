# Testing — the cargo suite

The combat model is render-free by design (the model/view split is recorded in [architecture.md](architecture.md)), so its rules are pinned by a headless Rust unit suite — no Bevy `App`, no rendering, no scene. The suite is the regression net for the math: tuning is a data edit, but the *forms* (formulas, clearance rules, event contracts) must not drift silently.

## What the suite is made of

[verification.md](../.claude/rules/verification.md) lists the commands and is the only authority on which ones must pass. The pre-commit hook runs a named subset of them; it skips the `cargo doc` steps, which are a dev/gate check rather than a commit check. This section explains what those aliases do and why, and never restates the list.

`dclippy` / `dtest` / `dbuild` / `doc-full` are the `.cargo/config.toml` aliases for the dynamic-linked dev/gate forms. The workspace-wide ones share ONE feature list — `game/development,editor/development` — so `dclippy` = `clippy --workspace --all-targets --features <that list>`, `dtest` = `test --workspace --features <that list>`, `doc-full` = `doc --workspace --no-deps --features <that list>`; `dbuild` = `build -p game --features development`. `development` is the umbrella feature on each host binary package: dynamic linking, the dev tools, the MCP host behind the `mcp` feature and the asset file watcher. Both hosts are named because a cargo feature is per-package, so `game/development` alone would leave the editor's QA modules and their test suites out of the run. `dev_tools` is the procgen load-time stepper's `bevy_egui` dependency — folded into every dev alias so the dev/gate loop always compiles, lints, and tests the stepper's drive/gate/command/summary logic (the non-egui majority of it, including the pure stage-summary formatter, split into its own `summary` module specifically so it stays unit-tested rather than riding behind the panel's exclusion); only the egui draw closure itself (`ui.rs`'s `draw_stepper_panel`) needs a primary window, so it is compiled only by `dbuild` (the content editor's own egui closure, `editor_egui_ui`, needs no window: a test adds `EguiPlugin` and leaves the editor's own camera holding the primary Egui context, so the closure and the form syncs it calls run under `dtest`). The whole affordance stays disengaged at runtime until a developer turns on the dev-only procgen-stepper toggle on the Options screen, which defaults to OFF and is only present in a `dev_tools` build (there is no environment variable). `dbuild` is the only step that builds + links the actual game binary (`check`/`clippy` never link; `test` links only test binaries). **`--all-features` is deliberately NOT used** — it forces a second full bevy build for no lint gain. `cargo doc` is dev/gate-only: the workspace `broken_intra_doc_links = "deny"` lint surfaces only under `cargo doc`, and it runs twice — the default-feature `cargo doc --workspace --no-deps` plus `cargo doc-full`, which enables the `dev_tools`-gated modules so their doc comments are link-checked too.

The workspace `Cargo.toml` denies clippy `all` / `pedantic` / `correctness` / `suspicious` plus the restriction lints (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`) and `missing_docs`, so being **fmt-clean and lint-clean IS part of "done"**, not a separate nicety. Run it yourself — never report a remembered or assumed result.

(`cargo nextest run` is an optional faster swap for the test step; default to `cargo dtest`.)

> Every behavioral change MUST add tests that exercise the **real** code path — never a stub or a shadow copy of the unit under test — so the suite grows with the code.

## Suite layout

Standard Cargo, placed per the module-layout test convention ([module-layout.md](../.claude/rules/module-layout.md), rule 5):

- **In-crate unit tests** — a sibling `test/` directory (or `test.rs` leaf) next to the module under test (an inline `#[cfg(test)] mod test` only while tiny), for white-box coverage of private helpers and tight math. This is where most `gdtf_battle_sim` coverage lives.
- **Integration tests** — `crates/<crate>/tests/`, one file per system or focused concern (`tests/<suite>/main.rs` dir-form once a suite outgrows one file), exercising the crate's public surface as a downstream user would (e.g. constructing a battle and driving a volley through the public verbs).

These repo-wide **guard suites** live in `crates/gdtf_conformance/tests/` and ride every `cargo dtest` run:

- **`module_layout`** — the module-layout conformance guard: wiring-only `mod.rs`, the 400-line block, and the exemption registry.
- **`no_flat_integration_tests`** — every crate's, bin's and lib's `tests/` holds dir-form suites (`<suite>/main.rs`), never flat `tests/*.rs` binaries.
- **`rustdoc_lint_gate`** — the workspace denies the whole rustdoc lint group and every member opts into the workspace lints.
- **`libs_layer`** — every crate under `libs/` carries a `cobalt_` name, names the game in no tracked file, and depends on nothing whose path leaves `libs/`.
- **`mcp_feature_gate`** — every file declaring the MCP host gates it on that host package's `mcp` feature, never on the build profile.

A guard suite asserts something about the code. One that asserts the state of the checkout instead
does not belong here: it goes red on ordinary editing, and it cannot tell a real defect from a file
someone just moved or a word someone just wrote. `doc_path_citations` and `no_graphic_name` were
both deleted for this. A check that needs the tree in a particular state belongs in `/gate` or the
pre-commit hook.

## Conventions

- **Idiomatic, warning-free Rust.** Test code lives under the same deny-by-default workspace lints as production code — no `unwrap`/`expect`/`panic!` slipping in via tests either; prefer `assert!`/`assert_eq!` and `?` on `Result`-returning helpers. fmt and clippy gate the tests too (`--all-targets`).
- **Determinism via injected, seeded RNG.** Anything random takes an RNG by parameter (e.g. `&mut impl rand::Rng`); a sim test seeds it by constructing a production stream resource — `ShotRng::from_root(BattleSeed::new(seed))` (or `SeverityRng::from_root(…)`, etc.) — and drawing through its `.rng()` handle, then asserts exact values or distributions (e.g. the body-part roll's weighting; see `crates/gdtf_battle_sim/src/damage_resolution/hit_location/test.rs`). The per-stream `rand_chacha::ChaCha12Rng` streams replace `rand::rngs::StdRng` (documented non-portable). Never a global/thread RNG. Same-seed-same-stream is itself a pinned property.
- **The sim's math needs no `App`.** Its rules are plain functions on data — construct the battle value directly (with coded-default tuning and a seeded RNG) and assert, with **no Bevy `App`, no `World`, no plugins**. The sim's ECS side — the plugins, the message handlers, the setup and teardown systems — does need a `World`, and its harness is the crate's own `gdtf_battle_sim::test_support::SimAppBuilder`: it adds the sim's plugins and test resources a piece at a time onto a `MinimalPlugins` app (`with_battle`, `with_acts`, `with_registries`, `with_full_vision`), driven with `app.update()`. `crates/gdtf_battle_sim/src/lifecycle/battle/test/teardown.rs` is one — it runs two battles in one app. **Nothing renders either way**, which is the model/view split the suite is there to hold (see [architecture.md](architecture.md)).
- **Presenter / scene / state tests** need Bevy. The standard way to step a minimal `App` is the headless harness `cobalt_test_utils::MinimalTestAppBuilder`: it takes the state registration as an argument, so passing `gdtf_game::test_support::register_headless` wires the real GDTF state stack onto a deterministic `MinimalPlugins` app. Build one with `MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless).starting_in(AppState::…).build()`. Drive it with `app.update()` (or `cobalt_test_utils::advance_until`, which spins updates until its condition holds, with no frame cap), and assert on `State<…>` / the `World`. **A test never waits on a frame budget or a wallclock deadline** (owner ruling 2026-08-14: both are flaky by definition — a loaded machine may make a test slower, never red). Wait for the observable condition and go red only on a definite failure signal; where a transition needs `FixedUpdate` steps, pin `TimeUpdateStrategy::FixedTimesteps(1)` instead of waiting out the real clock. A fixed frame count is fine only where it is itself the deterministic input — a hold proving a gate does NOT advance, a settle over per-frame draw systems — written as an explicit `for` loop. Scene / state / app **behavioral** logic — transitions, `OnEnter`/`OnExit` wiring, a system's effect on the `World` — is now a *required* headless integration test, not an optional one. Stub only true externals; never shadow the unit under test. Running the app (`cargo run -p game`) and observing behavior (screenshots come from the MCP channel's `capture.screenshot` command — there is no in-app trigger) is reserved for the genuinely unautomatable: actual rendering / visual correctness, real input, font / layout — and richer automation of *those* is **TBD (Bevy harness)**.
- **Guard coverage is deliberate.** Null/empty tuning, malformed arrays, out-of-bounds cells, degenerate directions — each hardening rule carries an assert, so a refactor can't quietly drop a guard.

## The `GDTF_*` dev flags

Every dev / QA / test affordance in the workspace is opted into by a `GDTF_*` environment
variable. They are ALL non-shipping: each is additionally gated on a debug build and/or a
dev-only cargo feature at its wiring site, so a release binary ignores the lot.

The old env-var battle capture/drive rig (`GDTF_AUTOBATTLE`, capture paths, fire-at-frame flags)
was deleted, not deprecated. The one drive path now is the `mcp` loopback QA network control
channel, compiled in by the `mcp` feature: a coding-agent QA harness (`mcp`, or any client speaking
`cobalt_mcp_protocol`) launches the game, negotiates the protocol version, reads the command list
that build publishes, runs one by name, and stops it — no env-var-scripted battle drive left in
the workspace. The request vocabulary is `Hello`, `Catalogue` and `Run` — so WHAT
the game can be asked to do is its command list, read at run time. That list is being rebuilt
one command at a time and today spans the app phase, screenshots, the shell reads, the battle
reads, starting and fleeing a battle, the procgen step, `wait`, the acts, the raw input paths
and the view controls. Read the names off [tooling/qa-commands.md](tooling/qa-commands.md) — a
second copy here would only rot. That guide is also how a command is added, and driving it is
[tooling/agent-qa.md](tooling/agent-qa.md). Census command (run from the repo root; re-run it
when adding a flag and keep this table in step):

```bash
grep -rhoE 'GDTF_[A-Z_0-9]+' crates/ bins/ --include='*.rs' | sort -u
```

| Variable | Owner crate | Effect |
| --- | --- | --- |
| `COBALT_TEST_FORCE_NO_GPU` | `cobalt_test_utils` | Forces the GPU-adapter probe to report Absent, driving the exact no-GPU skip path on a GPU machine. |
| ~~`GDTF_ASSETS_CLEAN_ROOT`~~ | — | Read by nothing. The assets-tree-clean guard it hooked no longer exists. |
| `GDTF_BATTLE_SEED` | `gdtf_game` (consumed by `gdtf_battle_sim`; honored by the `gdtf_game::test_support` battle harness) | Pins the root battle RNG seed for a reproducible replay; unset = wall-clock entropy, logged at `info!`. |
| `GDTF_DEBUG_REACHABLE_OVERLAY` | `gdtf_battle_presenter` (mirrored by `gdtf_battle_input` docs) | Truthy renders the reachable-range debug overlay in a debug build (default off — visual noise). |
| ~~`GDTF_EDITOR_MCP`~~ | — | Read by nothing. The editor's channel is armed by `EDITOR_MCP_PORT` alone; there is no separate arming variable. Launch recipe is in [tooling/agent-qa.md](tooling/agent-qa.md). |
| `EDITOR_MCP_PORT` | `gdtf_editor`, `mcp` | Required. The editor binds this value when it trims and parses as a `u16`, always on `Ipv4Addr::LOCALHOST`, and opens no listener at all otherwise. There is no fallback port. The spawner sets the port name on every child it starts from the port the launch picked, so `launch(host="editor", port=…)` puts the editor child there unless a record of that host already holds the port, in which case it takes the first free port above. Read in the MCP host's own environment, it also moves the port the editor link opens on and the port a `launch` or `stop` that names none targets. |
| `GDTF_MODULE_LAYOUT_ROOT` | `gdtf_conformance` | Overrides the repo root the module-layout conformance guard test scans (guard-test hook). |
| `GDTF_RUSTDOC_GATE_ROOT` | `gdtf_conformance` | Overrides the repo root the rustdoc-lint guard test scans (guard-test hook). |
| `GDTF_SEED_LOGGING_CASE` | `gdtf_game` | Names which seed-logging case a re-invoked child test process runs — `env-pinned` or `wall-clock`. Set for the child by the parent test. |
| ~~`GDTF_MCP`~~ | — | Read by nothing. The game's channel is armed by `GDTF_MCP_PORT` alone; there is no separate arming variable. That channel is the ONE drive path (`launch` → `commands` → `run` → `stop`, driven via `mcp` or any `cobalt_mcp_protocol` client). |
| `GDTF_MCP_PORT` | `gdtf_game`, `mcp` | Required. The game binds this value when it trims and parses as a `u16`, always on `Ipv4Addr::LOCALHOST`, and opens no listener at all otherwise. There is no fallback port. The spawner sets it on every child it starts, so `launch(host="game", port=N)` puts the game child on `N`. Read in the MCP host's own environment, it moves the port the game link opens on and the port a `launch` or `stop` that names none targets. |

## What the suite pins (and what it doesn't)

It pins **behavioral contracts**: the march's exact cell walk, the strictly-higher-sails-over clearance rule, the per-hit damage formula, the severity bucket edges' *application* (not their magnitudes), event-emission discipline, the no-target-stop rule, dead-centre at cone 0, and the id-not-value boundary between sim and presenter. It does **not** pin tuning magnitudes — those live in tuning-config data and are expected to change. It does **not** assert on rendered pixels or exact `Transform` values beyond what a behavioral contract requires.

## Integration test packing

Flat `tests/*.rs` files each become a separate binary and re-link Bevy. Prefer **dir-form** suites (`tests/suite_name/main.rs` + modules), as [module-layout.md](../.claude/rules/module-layout.md) rule 5 requires. A guard under `crates/gdtf_conformance/tests/no_flat_integration_tests/` walks every `crates/*/tests/`, `bins/*/tests/` and `libs/*/tests/` and fails on a new flat file; the ones already there when the walk was added are listed in its `ALLOWED_FLATS`.
