---
paths:
  - "**/*"
---

# Verification — the definition of done

Why this rule exists: completion claims with nothing actually run are this
kit's recurring lie. "It should work" is not evidence. Done = the suite was
observed GREEN in THIS session, after the final edit, and you saw it pass.

## The ONE definition of green (dev / gate — dynamic-linked, fast)

Run from the repo root; green = ALL FOUR pass:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings
cargo test --workspace --features grimdark_turfwar/dynamic_linking
cargo build -p grimdark_turfwar --features dynamic_linking
```

`cargo dclippy` / `cargo dtest` / `cargo dbuild` / `cargo drun` (aliases in
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

CI uses the STATIC suite (no `dynamic_linking` feature):

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`dynamic_linking` is a dev-iteration speedup ONLY — it is NEVER used for static
builds. The dev/gate green above is the fast, dynamic-linked loop; this static
suite is the CI gate.

The static **release-binary** build (`cargo build -p grimdark_turfwar --release`)
is NOT a CI gate. It is DEFERRED to the packaging process — a much-later,
packaging-time check — because building Bevy + wgpu in release on top of the
dev/test target exhausts the CI runner's disk ("No space left on device"). When
release artifacts are packaged, that build runs there (still static — release
NEVER uses `dynamic_linking`). See GTW-140.

NOTE: gdtf currently has ZERO tests, so `cargo test` passes trivially today.
That is NOT red — do not invent a "nothing ran = red" rule. But every
behavioral ticket MUST add tests on the real code path.

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
   logic. Reserve in-engine evidence — RUN the app
   (`cargo run -p grimdark_turfwar --features dynamic_linking`) and observe
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
