# Testing — the cargo suite

The combat model is render-free by design (the model/view split is recorded in [architecture.md](architecture.md)), so its rules are pinned by a headless Rust unit suite — no Bevy `App`, no rendering, no scene. The suite is the regression net for the math: tuning is a data edit, but the *forms* (formulas, clearance rules, event contracts) must not drift silently.

## What the suite is made of

[verification.md](../.claude/rules/verification.md) lists the commands and is the only authority on which ones must pass. The pre-commit hook runs a named subset of them; it skips the `cargo doc` steps, which are a dev/gate check rather than a commit check, and it skips the two release QA-host builds, which cost minutes each on the release profile. This section explains what those aliases do and why, and never restates the list.

`dclippy` / `dtest` / `dbuild` / `doc-full` are the `.cargo/config.toml` aliases for the dynamic-linked dev/gate forms. The workspace-wide ones share ONE feature list — `game/development,editor/development` — so `dclippy` = `clippy --workspace --all-targets --features <that list>`, `dtest` = `test --workspace --features <that list>`, `doc-full` = `doc --workspace --no-deps --features <that list>`; `dbuild` = `build -p game --features development`. `development` is the umbrella feature on each host binary package: dynamic linking, the dev tools, the MCP host behind the `mcp` feature and the asset file watcher. Both hosts are named because a cargo feature is per-package, so `game/development` alone would leave the editor's QA modules and their test suites out of the run. `dev_tools` is the procgen load-time stepper's `bevy_egui` dependency — folded into every dev alias so the dev/gate loop always compiles, lints, and tests the stepper's drive/gate/command/summary logic (the non-egui majority of it, including the pure stage-summary formatter, split into its own `summary` module specifically so it stays unit-tested rather than riding behind the panel's exclusion); only the egui draw closure itself (`ui.rs`'s `draw_stepper_panel`) needs a primary window, so it is compiled only by `dbuild` (the content editor's own egui closure, `editor_egui_ui`, needs no window: a test adds `EguiPlugin` and leaves the editor's own camera holding the primary Egui context, so the closure and the form syncs it calls run under `dtest`). The whole affordance stays disengaged at runtime until a developer turns on the dev-only procgen-stepper toggle on the Options screen, which defaults to OFF and is only present in a `dev_tools` build (there is no environment variable). `dbuild` and `mcpbuild` are the steps that build + link the actual game binary, `dbuild` on the dev profile and `mcpbuild` on release (`check`/`clippy` never link; `test` links only test binaries). **`--all-features` is deliberately NOT used** — it forces a second full bevy build for no lint gain. `cargo doc` is dev/gate-only: the workspace `broken_intra_doc_links = "deny"` lint surfaces only under `cargo doc`, and it runs twice — the default-feature `cargo doc --workspace --no-deps` plus `cargo doc-full`, which enables the `dev_tools`-gated modules so their doc comments are link-checked too.

`mcpbuild` and `edmcpbuild` are the release-profile aliases: `build --release -p game --features mcp` and `build --release -p editor --features mcp`. Each host package declares `default = []`, so those builds carry the QA host and nothing else: no dynamic linking, no dev tools, no file watcher. Nothing else in the suite runs on the release profile, so nothing else compiles that configuration, and a release-only break in the QA host shows nowhere else.

The workspace `Cargo.toml` denies the whole rustc `warnings` group, clippy `all` / `pedantic` / `correctness` / `suspicious` plus the restriction lints (`unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`) and `missing_docs`, so being **fmt-clean and lint-clean IS part of "done"**, not a separate nicety. The rustc group deny is written in the manifest because `cargo build` takes no trailing `-- -D warnings`, so it is the only thing that can fail a build step on a rustc warning. It reaches every cargo command and every crate in the workspace. Run it yourself. Never report a remembered or assumed result.

(`cargo nextest run` is an optional faster swap for the test step; default to `cargo dtest`.)

> Every behavioral change MUST add tests that exercise the **real** code path — never a stub or a shadow copy of the unit under test — so the suite grows with the code.

## Suite layout

Standard Cargo, placed per the module-layout test convention ([module-layout.md](../.claude/rules/module-layout.md), rule 5):

- **In-crate unit tests** — a sibling `test/` directory (or `test.rs` leaf) next to the module under test (an inline `#[cfg(test)] mod test` only while tiny), for white-box coverage of private helpers and tight math. This is where most `gdtf_battle_sim` coverage lives.
- **Integration tests** are one binary per crate, `crates/<crate>/tests/<name>_suite/main.rs` (`game_suite`, `editor_suite`, `battle_sim_suite`), declaring one `mod` per suite. Each suite is a directory beside it with a wiring-only `mod.rs` and one file per system or focused concern, exercising the crate's public surface as a downstream user would (e.g. constructing a battle and driving a volley through the public verbs). Run one suite with `cargo dtest --test <name>_suite -- <suite>::`.

These repo-wide **guard suites** live in `crates/gdtf_conformance/tests/` and ride every `cargo dtest` run:

- **`module_layout`** — the module-layout conformance guard: wiring-only `mod.rs`, the 400-line block, and the exemption registry.
- **`no_flat_integration_tests`** — no crate, bin or lib has a flat `tests/*.rs` binary, a second `tests/<dir>/main.rs` binary, or one not named `<name>_suite`.
- **`rustdoc_lint_gate`** — the workspace denies the whole rustdoc lint group and the whole rustc `warnings` group, and every member opts into the workspace lints.
- **`libs_layer`** — every crate under `libs/` carries a `cobalt_` name, names the game in no tracked file, and depends on nothing whose path leaves `libs/`.
- **`mcp_feature_gate`** — every file declaring the MCP host gates it on that host package's `mcp` feature, never on the build profile.
- **`asset_plugin_sites`** fails when an `AssetPlugin` outside the game and editor hosts leaves `watch_for_changes_override` unset or true.
- **`no_restored_automatic_time`** fails when a tracked Rust file under `crates/`, `bins/` or `libs/` names `TimeUpdateStrategy::Automatic`, so a test that pins a manual frame delta leaves it pinned.
- **`workflow_scripts`** — every tracked `*.js` file under `.claude/workflows/` compiles, and `WORK_RESULT` in `build-ticket.js` stays within the guard's `LIMIT`, 3500 characters, above which an agent's output schema is refused before the agent does any work. Those scripts use a top-level `return`, so the guard compiles each one wrapped in a function body, the way the harness that runs them does. It parses with `node` and fails when `node` is absent rather than skipping, because a script no run can load would otherwise reach develop.

A guard suite asserts something about the code. One that asserts the state of the checkout instead
does not belong here: it goes red on ordinary editing, and it cannot tell a real defect from a file
someone just moved or a word someone just wrote. `doc_path_citations` and `no_graphic_name` were
both deleted for this. A check that needs the tree in a particular state belongs in `/gate` or the
pre-commit hook.

## Conventions

- **Idiomatic, warning-free Rust.** Test code lives under the same deny-by-default workspace lints as production code — no `unwrap`/`expect`/`panic!` slipping in via tests either; prefer `assert!`/`assert_eq!` and `?` on `Result`-returning helpers. fmt and clippy gate the tests too (`--all-targets`).
- **Determinism via injected, seeded RNG.** Anything random takes an RNG by parameter (e.g. `&mut impl rand::Rng`); a sim test seeds it by constructing a production stream resource — `ShotRng::from_root(BattleSeed::new(seed))` (or `SeverityRng::from_root(…)`, etc.) — and drawing through its `.rng()` handle, then asserts exact values or distributions (e.g. the body-part roll's weighting; see `crates/gdtf_battle_sim/src/damage_resolution/hit_location/test.rs`). The per-stream `rand_chacha::ChaCha12Rng` streams replace `rand::rngs::StdRng` (documented non-portable). Never a global/thread RNG. Same-seed-same-stream is itself a pinned property.
- **The sim's math needs no `App`.** Its rules are plain functions on data — construct the battle value directly (with coded-default tuning and a seeded RNG) and assert, with **no Bevy `App`, no `World`, no plugins**. The sim's ECS side — the plugins, the message handlers, the setup and teardown systems — does need a `World`, and its harness is the crate's own `gdtf_battle_sim::test_support::SimAppBuilder`: it adds the sim's plugins and test resources a piece at a time onto a `MinimalPlugins` app (`with_battle`, `with_acts`, `with_registries`, `with_full_vision`), driven with `app.update()`. `crates/gdtf_battle_sim/src/lifecycle/battle/test/teardown.rs` is one — it runs two battles in one app. **Nothing renders either way**, which is the model/view split the suite is there to hold (see [architecture.md](architecture.md)).
- **Presenter / scene / state tests** need Bevy. The standard way to step a minimal `App` is the headless harness `cobalt_test_utils::MinimalTestAppBuilder`: it takes the state registration as an argument, so passing `gdtf_game::test_support::register_headless` wires the real GDTF state stack onto a deterministic `MinimalPlugins` app. Build one with `MinimalTestAppBuilder::new(gdtf_game::test_support::register_headless).starting_in(AppState::…).build()`. Drive it with `app.update()` (or `cobalt_test_utils::advance_until`, which spins updates until its condition holds, with no frame cap, and `advance_until_mut` for a condition that needs `&mut App` to read, such as one that runs a `World::query`), and assert on `State<…>` / the `World`. **A test never waits on a frame budget or a wallclock deadline** (owner ruling 2026-08-14: both are flaky by definition — a loaded machine may make a test slower, never red). Wait for the observable condition and go red only on a definite failure signal; where a transition needs `FixedUpdate` steps, pin `TimeUpdateStrategy::FixedTimesteps(1)` instead of waiting out the real clock. **A test app never restores automatic time.** A test that pins `TimeUpdateStrategy::ManualDuration` leaves it pinned and never writes `TimeUpdateStrategy::Automatic` back, because every frame after that write reads whatever the machine took. A helper that steps time by hand returns with a manual delta still pinned: either the step it used, or the pinned delta its harness starts from, which is what the `fx_draw` helpers in `crates/gdtf_battle_presenter/tests/battle_presenter_suite/fx_draw/harness.rs` restore. The `no_restored_automatic_time` guard fails any tracked Rust file that names `TimeUpdateStrategy::Automatic`. The frame-budget rule is about test code. `HostManager::await_readiness` in `libs/cobalt_mcp_server/src/lifecycle/manager/host.rs` sleeps its configured `poll_interval` between readiness probes, which is production polling: it waits on a real child process it cannot observe any other way. A fixed frame count is fine only where it is itself the deterministic input — a hold proving a gate does NOT advance, a settle over per-frame draw systems — written as an explicit `for` loop. Scene / state / app **behavioral** logic — transitions, `OnEnter`/`OnExit` wiring, a system's effect on the `World` — is now a *required* headless integration test, not an optional one. Stub only true externals; never shadow the unit under test. Running the app (`cargo run -p game`) and observing behavior (screenshots come from the MCP channel's `capture.screenshot` command — there is no in-app trigger) is reserved for the genuinely unautomatable: actual rendering / visual correctness, real input, font / layout — and richer automation of *those* is **TBD (Bevy harness)**.
- **A test app never watches its asset root.** The dev feature set turns Bevy's `file_watcher` on, and a watcher stats every file under its root each time an app is built, which is most of a `cargo dtest` run's system time. So every test asset plugin comes from `cobalt_test_utils::asset_plugin_at` for a chosen root, or `cobalt_test_utils::unwatched_asset_plugin` for the default one. A test that changes a file on disk asks for the reload itself with `AssetServer::reload`. The `asset_plugin_sites` guard checks the watcher setting rather than the route to it: outside the two hosts, every plugin literal must carry `watch_for_changes_override: Some(false)`, and a fully defaulted plugin fails. `gdtf_battle_sim::test_support` writes that literal inline, because it is a `pub mod` of the sim library and cannot reach a dev-dependency.
- **No doc examples run.** Every lib sets `doctest = false`. No doc comment holds a runnable example, and rustdoc's per-crate test harness cost 4.7s a run for zero tests. A code sample in a doc comment is a `text` fence or lives in a test.
- **Guard coverage is deliberate.** Null/empty tuning, malformed arrays, out-of-bounds cells, degenerate directions — each hardening rule carries an assert, so a refactor can't quietly drop a guard.

## The `GDTF_*`, `COBALT_*` and `EDITOR_*` census

The workspace configures nothing through an environment variable of its own. A host takes the
port its listener binds on the command line, as `-- --mcp-port <N>`, which the spawner appends
to the `cargo run` it starts the child with. A host started without that argument opens no
listener. A `launch` or `stop` that names no port targets the host's registered default, held
by `registry()` in `bins/mcp/src/hosts.rs`. Cargo's own variables, such as
`CARGO_TARGET_TMPDIR` in a test, are cargo's and are not on this list.

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
when adding a flag):

```bash
grep -rhoE '"(GDTF|COBALT|EDITOR)_[A-Z_0-9]+"' crates/ bins/ libs/ --include='*.rs' \
  --exclude='poisoned_environment.rs' | tr -d '"' | sort -u
```

It prints nothing. The one excluded file is
`bins/mcp/tests/mcp_suite/launch/poisoned_environment.rs`, which writes `GDTF_MCP_PORT`,
`EDITOR_MCP_PORT`, `GDTF_MCP` and `GDTF_EDITOR_MCP` onto a child to prove the launcher sets
no variable and a poisoned one cannot move the port that child binds. A file that writes a
name so a run can forbid it is excluded for the same reason `ALLOWED` exists in
`crates/gdtf_conformance/tests/conformance_suite/libs_layer/scan.rs`. The battle seed is a
`battle.start` argument, the reachable-range overlay is `view.toggle_reachable_overlay`, and
the GPU probe takes its force-absent flag as a parameter.

## What the suite pins (and what it doesn't)

It pins **behavioral contracts**: the march's exact cell walk, the strictly-higher-sails-over clearance rule, the per-hit damage formula, the severity bucket edges' *application* (not their magnitudes), event-emission discipline, the no-target-stop rule, dead-centre at cone 0, and the id-not-value boundary between sim and presenter. It does **not** pin tuning magnitudes — those live in tuning-config data and are expected to change. It does **not** assert on rendered pixels or exact `Transform` values beyond what a behavioral contract requires.

## Integration test packing

Every crate has one integration-test binary. Each `tests/<dir>/main.rs` and each flat `tests/*.rs` is its own binary that compiles and links Bevy again, and cargo rebuilds every one of them that depends on an edited crate. Measured on 2026-09-08 with 96 such binaries: a one-line edit in `cobalt_test_utils` took `cargo dtest --no-run` 5:29 and a one-line edit in `gdtf_battle_sim` 1:02. A warm run pays little for the count (0.5s between targets), so the merge is for the edit loop, not the warm run. The guard under `crates/gdtf_conformance/tests/conformance_suite/no_flat_integration_tests/` fails on a flat file; [module-layout.md](../.claude/rules/module-layout.md) rule 5 names the shape.
