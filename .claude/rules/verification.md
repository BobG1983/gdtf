---
paths:
  - "**/*"
---

# Verification — the definition of done

Why this rule exists: completion claims with nothing actually run are this
kit's recurring lie. "It should work" is not evidence. Done = the suite was
observed GREEN in THIS session, after the final edit, and you saw it pass.

## The ONE definition of green (dev / gate — dynamic-linked, fast)

Run from the repo root; green = ALL EIGHT pass. **In practice, always invoke these via
their `.cargo/config.toml` ALIASES (`cargo dclippy`, `cargo dtest`, `cargo dbuild`,
`cargo doc-full`, `cargo clippy-schema`, `cargo test-schema`) — never hand-type the
equivalent long-form flags shown below.** The
block below documents exactly what each alias expands to; typing it out yourself risks
silently dropping the `dynamic_linking` feature (or a sibling feature) and falling back
to a slow, fully static rebuild — the one failure mode this whole fast dev/gate loop
exists to avoid. Every agent definition and skill in this repo (`engineer`,
`design-gate`, `qa`, `bevy-expert`, `/gate`, `/land`, `/next-task`, `/docs-sync`,
`/file-bug`, `/health-check`) is expected to use the alias form, not this expansion:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking,grimdark_turfwar/dev_tools,grimdark_turfwar/net_qa,gdtf_content_editor/net_qa -- -D warnings
cargo test --workspace --features grimdark_turfwar/dynamic_linking,grimdark_turfwar/dev_tools,grimdark_turfwar/net_qa,gdtf_content_editor/net_qa
cargo build -p grimdark_turfwar --features dynamic_linking,file_watcher,dev_tools,net_qa
cargo doc --workspace --no-deps
cargo doc-full
cargo clippy -p gdtf_qa_protocol --all-targets --features schema -- -D warnings
cargo test -p gdtf_qa_protocol --features schema
```

Note the TWO `net_qa` entries in that feature list. A package-qualified feature turns on only
THAT package's feature, and the game and the editor each own a separate `net_qa`:
`grimdark_turfwar/net_qa` reaches the game's `gdtf_app::dev::net_qa`, `gdtf_content_editor/net_qa`
reaches the editor's `gdtf_content_editor::net_qa`. Until GTW-877 only the game's was named, so
the editor's entire `src/net_qa/` module (GTW-804 / GTW-805) was compiled, linted, doc-checked and
tested by NOTHING in the gate, and its two `#![cfg(all(debug_assertions, feature = "net_qa"))]`
test binaries silently reported "0 tests". The fix names the editor's feature on the runs that
already existed: NO new suite step, no second bevy build (these runs already compile the editor
crate and both of the feature's optional deps). Adding a separate editor-only run instead would
have been redundant — the workspace runs already cover the crate.

`cargo doc` is a dev/gate-only step (not CI): the workspace rustdoc lints only surface under
`cargo doc`, not under `clippy` or `build`, so they require their own run. It runs TWICE, in two
feature configs — the same both-configs rule the `dclippy` dual gate follows (an optional-feature
module must be checked WITH its feature on, not just off):

- `cargo doc --workspace --no-deps` — the DEFAULT-feature run.
- `cargo doc-full` (alias for `cargo doc --workspace --no-deps --features
  grimdark_turfwar/dev_tools,grimdark_turfwar/net_qa,gdtf_content_editor/net_qa`) — enables
  `dev_tools` + BOTH `net_qa` features so the doc comments inside those feature-gated modules
  (`gdtf_app`'s `crate::dev::procgen_stepper` and `crate::dev::net_qa`, and the editor's
  `gdtf_content_editor::net_qa`) are link-checked too. Without it a broken intra-doc link inside
  one of those modules is never compiled by the default `cargo doc`, so it slips past the gate and
  CI (the gap GTW-790 fixed for the game, GTW-877 for the editor).

**What the doc steps gate: EVERY rustdoc lint, since GTW-929.** Before that they gated almost
nothing. `[workspace.lints.rustdoc]` named only `broken_intra_doc_links` and
`private_intra_doc_links` as `deny`; every other rustdoc lint kept its default `warn`, and no
`RUSTDOCFLAGS` promoted them, so both doc runs exited 0 no matter how many rustdoc warnings piled
up — and they did pile up, unobserved, because nothing could fail on them. Intra-doc link checking
itself always worked; what did not work was everything else. The workspace `Cargo.toml` now
declares `all = { level = "deny", priority = -1 }` in that table, so a rustdoc warning of ANY kind
fails the doc step, the same way `-D warnings` makes every clippy warning fatal. `priority = -1`
keeps an individual lint listed below the group able to override the group level.

Setting the level in `Cargo.toml` rather than on the aliases is deliberate: it covers both doc runs
by construction, so they cannot drift apart the way the alias feature flags did (the gap GTW-790
and GTW-877 each had to close separately). It reaches rustdoc LINTS only — cargo's own
"output filename collision" warning is not a lint, and no level touches it. That one was real: the
`gdtf_content_editor` bin target and the library crate of the same name both wrote
`target/doc/gdtf_content_editor/index.html`, so one replaced the other's rendered page and a reader
could not tell which crate's docs they had (rust-lang/cargo#6313). It is fixed at its source by
`doc = false` on the `[[bin]]` target in `bins/gdtf_content_editor/Cargo.toml`, which changes no
built path — `target/debug/gdtf_content_editor`, the path `docs/tooling/agent-qa.md` names for the
editor-QA process check, is untouched.

A `[workspace.lints]` table reaches only a crate that declares `[lints] workspace = true`, so the
group deny is worth exactly as much as that opt-in's coverage.
`crates/gdtf_test_utils/tests/rustdoc_lint_gate/` pins all three parts — the group deny with its
priority, the opt-in in every workspace member, and the bin's `doc = false`. Without it, deleting
any of them leaves the whole suite green while restoring the defect.

## The `schema` feature steps — PACKAGE-SCOPED on purpose (GTW-939)

The last two steps are `cargo clippy-schema` and `cargo test-schema`, aliases for
`clippy -p gdtf_qa_protocol --all-targets --features schema` and
`test -p gdtf_qa_protocol --features schema`. They exist because
`crates/gdtf_qa_protocol` declares `schemars` as an OPTIONAL dependency behind a `schema`
feature, and the 6 `#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]` lines across
`src/ids/{cell,shot}.rs` plus the whole `src/ids/test/schema.rs` module are reachable only with
that feature on. Nothing else in the gate or in CI turns it
on, so before these steps every one of those lines could be deleted — or replaced with a
compile error — and all six commands above, plus both CI workflows, stayed green. That is
the GTW-790 / GTW-877 defect class in a third crate: an optional-feature module must be
checked WITH its feature on, not just off.

**Why `-p gdtf_qa_protocol` rather than adding `gdtf_qa_protocol/schema` to the
`--workspace` feature lists:** cargo builds ONE `gdtf_qa_protocol` library unit per
invocation and every consumer in that invocation links it. Naming the feature on a
`--workspace` command therefore puts `schemars` inside `bins/gdtf_qa_mcp` — the MCP courier,
which carries schema documents as opaque text. `-p gdtf_qa_protocol` never builds the courier
at all. Two guards pin the command lines:
`crates/gdtf_qa_protocol/tests/engine_free.rs` fails if any `--workspace` line in
`.cargo/config.toml` or a workflow names `gdtf_qa_protocol/schema`, and
`crates/gdtf_test_utils/tests/ci_workflow_features/schema_step.rs` fails if either CI step
disappears or widens its scope.

**What that does NOT buy, since GTW-941.** A dependency edge enables a feature just as a
command line does, and neither guard reads a manifest. `crates/gdtf_qa_command` requires
`gdtf_qa_protocol/schema` unconditionally — the GTW-941 ticket mandates it, the crate derives
every command's argument and reply schema — and `crates/*` is a member glob, so every
`--workspace` run (`cargo dclippy`, `cargo dtest`) now unifies `schema` on and the courier's
unit DOES link schemars on those dev runs. The property that still holds is the shipped one:
`bins/gdtf_qa_mcp` enables no schema feature in its own manifest and depends on no crate that
does, so a courier-only build (`cargo build -p gdtf_qa_mcp`, and the packaging build) links no
schemars. Widening the guard to manifest edges is GTW-946; until it lands, do not read
`engine_free.rs` as proof that a workspace run keeps the courier clear.

`cargo dclippy` / `cargo dtest` / `cargo dbuild` / `cargo drun` / `cargo doc-full` /
`cargo clippy-schema` / `cargo test-schema` (aliases in
`.cargo/config.toml`) are the shorthand for these. Dynamic linking via
`grimdark_turfwar/dynamic_linking` keeps the dev/gate loop fast; `--all-features`
is NOT used — it forces a second full bevy build and adds no lint value.

The binary build (`cargo dbuild`) is NOT redundant with the others: `check` /
`clippy` never LINK and `test` only links TEST binaries, so neither builds the
actual `grimdark_turfwar` binary; and `--workspace` compiles `gdtf_app` WITH
`test-support` (feature unification via `gdtf_test_utils`), masking an
`unreachable_pub` that only fires when the binary is built WITHOUT it. Building
the binary is the only step that catches that class — and link errors. Do NOT
use `cargo drun` in the gate: it launches the GUI, which a gate cannot drive.
[added GTW-145]

The workspace denies clippy all/pedantic/correctness plus
unwrap/expect/panic/todo/unimplemented and missing_docs, so being fmt-clean and
lint-clean IS part of "done", not a separate nicety. (nextest is an optional
faster swap for the test step; default to `cargo test`.) Run it yourself —
never report a remembered or assumed result.

## The CI green (static — no dynamic_linking)

CI uses the STATIC suite (no `dynamic_linking` feature, no `dev_tools`):

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/net_qa,gdtf_content_editor/net_qa -- -D warnings
cargo test --workspace --features grimdark_turfwar/net_qa,gdtf_content_editor/net_qa
cargo clippy -p gdtf_qa_protocol --all-targets --features schema -- -D warnings
cargo test -p gdtf_qa_protocol --features schema
```

The last two are the GTW-939 `schema`-feature steps described above, mirrored into
`.github/workflows/clippy.yml` and `test.yml` so the feature-on configuration is not dark on
a PR either. They stay PACKAGE-SCOPED in CI for the same reason they are locally: a
`--workspace` run with `schema` on would link schemars into `bins/gdtf_qa_mcp`.

`dynamic_linking` is a dev-iteration speedup ONLY — it is NEVER used for static
builds. The dev/gate green above is the fast, dynamic-linked loop; this static
suite is the CI gate.

BOTH `net_qa` features are named on the clippy and test steps (GTW-883,
`.github/workflows/clippy.yml` and `test.yml`), for the same reason the dev
aliases name both: a package-qualified feature turns on only that package's
feature. Before GTW-883 the two CI steps named NO feature, so the game's
`gdtf_app::dev::net_qa` and the editor's whole `gdtf_content_editor::net_qa`
module were unlinted and untested on every PR, and the editor's two
`#![cfg(all(debug_assertions, feature = "net_qa"))]` test binaries reported
"0 tests". The build-time cost was measured before deciding, and it is small:
the workspace unit graph is the SAME size either way — 668 units with and
without both features (`cargo test --workspace --no-run -Z unstable-options
--unit-graph`), so no extra dependency crate is compiled; the features only
turn on code inside crates CI already builds. Measured marginal work on a warm
tree: `cargo check --workspace --all-targets` static took 51.83s, and re-running
it with `--features grimdark_turfwar/net_qa,gdtf_content_editor/net_qa` took
8.52s, rebuilding only the feature-affected workspace crates.
`dev_tools` stays OFF in CI: it is an egui dev overlay the dev/gate loop
covers, not a QA regression surface.

The two commands are pinned by a guard so the flag cannot be dropped again
unnoticed: `crates/gdtf_test_utils/tests/ci_workflow_features/` reads every
tracked `.github/workflows/*.yml`, and any `run:` step invoking a
`--workspace` cargo command must name both `net_qa` features and must not name
`dynamic_linking`. Without it, deleting `--features …` from a workflow leaves
the whole suite green — nothing else in the repo reads `.github/`. That suite's
`schema_step.rs` (GTW-939) guards the two package-scoped `schema` steps the same
way: it fails if either is deleted, and it fails if any workflow step enables
`--features schema` without `-p gdtf_qa_protocol`.

Do NOT add a `#![cfg(feature = "net_qa")]` gate to
`crates/gdtf_net_qa_transport/tests/transport/main.rs`. That crate declares no
`net_qa` feature (only `dynamic_linking`), so the gate would never be true and
would silently delete the transport tests.

The static **release-binary** build (`cargo build -p grimdark_turfwar --release`)
is NOT a CI gate. It is DEFERRED to the packaging process — a much-later,
packaging-time check — because building Bevy + wgpu in release on top of the
dev/test target exhausts the CI runner's disk ("No space left on device"). When
release artifacts are packaged, that build runs there (still static — release
NEVER uses `dynamic_linking`). See GTW-140.

NOTE: the suite is 2000+ tests across ~180 targets, but test COUNT is not the
gate: a target that collects zero tests still passes, and that is NOT red — do
not invent a "nothing ran = red" rule. Coverage is enforced the other way:
every behavioral ticket MUST add tests on the real code path.

## Rules

1. Done = the full suite above observed green in this session, AFTER your final
   edit. Anything less is "in progress", not done.
2. New behavior requires new tests that exercise the REAL code path. Never
   fake, stub, or shadow the unit under test to make a test pass; stub only
   true externals. Sim logic (`gdtf_battle_sim`) is render-free and
   unit-testable with injected seeded RNG. A test of a copy is not a test.
3. Scene / state / app BEHAVIORAL logic (transitions, `OnEnter`/`OnExit`
   wiring, system effects on the `World`) REQUIRES a headless integration test
   built with `gdtf_test_utils::GdtfTestAppBuilder`: construct the app, drive
   it with `app.update()` (or `gdtf_test_utils::advance_until`), and assert on
   `State<…>` / the `World`. The harness exists — reading the code, or "I ran
   it and it looked right", is not verification for state-machine or system
   logic. Reserve in-engine evidence — RUN the app (`cargo drun`) and observe
   (the app can capture its own screenshot) — for genuinely unautomatable
   checks: actual RENDERING / visual correctness, real input, font / layout.
   Those, and only those, are still **TBD (Bevy harness)** for richer
   automation.
4. Report failures VERBATIM — paste the failing assert / compiler / clippy
   output. Never summarize a failure away or present partial success as
   success.
5. `/gate` is the gatekeeper: it runs this suite plus the `design-gate` audit.
   No gate-pass, no commit. `/gate` step 4a is unchanged and unweakened — and
   note that, per rule 3, any scene / state / app clause now OWES a headless
   `GdtfTestAppBuilder` test as part of being covered, not just a screenshot.
