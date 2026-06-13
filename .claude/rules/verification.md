---
paths:
  - "**/*"
---

# Verification — the definition of done

Why this rule exists: completion claims with nothing actually run are this
kit's recurring lie. "It should work" is not evidence. Done = the suite was
observed GREEN in THIS session, after the final edit, and you saw it pass.

## The ONE definition of green (dev / gate — dynamic-linked, fast)

Run from the repo root; green = ALL THREE pass:

```
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings
cargo test --workspace --features grimdark_turfwar/dynamic_linking
```

`cargo dclippy` / `cargo dtest` / `cargo drun` (aliases in `.cargo/config.toml`)
are the shorthand for these. Dynamic linking via
`grimdark_turfwar/dynamic_linking` keeps the dev/gate loop fast; `--all-features`
is NOT used — it forces a second full bevy build and adds no lint value.

The workspace denies clippy all/pedantic/correctness plus
unwrap/expect/panic/todo/unimplemented and missing_docs, so being fmt-clean and
lint-clean IS part of "done", not a separate nicety. (nextest is an optional
faster swap for the test step; default to `cargo test`.) Run it yourself —
never report a remembered or assumed result.

## The RELEASE / CI green (static — no dynamic_linking)

Release artifacts and CI use the STATIC suite (no `dynamic_linking` feature):

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build -p grimdark_turfwar --release
```

`dynamic_linking` is a dev-iteration speedup ONLY — it is NEVER used for release
artifacts. The dev/gate green above is the fast, dynamic-linked loop; this static
suite is what gates a release build.

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
3. View / scene / presenter work additionally requires in-engine evidence:
   RUN the app (`cargo run -p grimdark_turfwar --features dynamic_linking`) or a
   headless Bevy integration test, and observe the behavior (the app can capture its own
   screenshot). Reading the code is not verification. Richer input/scene
   automation is **TBD (Bevy harness)** until one exists.
4. Report failures VERBATIM — paste the failing assert / compiler / clippy
   output. Never summarize a failure away or present partial success as
   success.
5. `/gate` is the gatekeeper: it runs this suite plus the `design-gate` audit.
   No gate-pass, no commit.
