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
- **Determinism via injected, seeded RNG.** Anything random takes an RNG by parameter (e.g. `&mut impl rand::Rng`, or `rand::rngs::StdRng::seed_from_u64(seed)`); tests seed it and assert exact values or distributions (e.g. the body-part roll's weighting). Never a global/thread RNG. Same-seed-same-stream is itself a pinned property.
- **Model tests need no Bevy.** `gdtf_battle_sim` is plain Rust on data — construct the battle value directly (with coded-default tuning and a seeded RNG) and assert. **No Bevy `App`, no `World`, no plugins, no rendering.** This is the whole point of keeping the sim Bevy-free (see the model/view split in [ADR 0001](decisions/0001-rust-bevy-rewrite.md)).
- **Presenter / scene / state tests** need Bevy. The standard way to step a minimal `App` is the headless harness `gdtf_test_utils::GdtfTestAppBuilder`: it wires the real GDTF state stack onto a deterministic `MinimalPlugins` app, so you `GdtfTestAppBuilder::new().starting_in(AppState::…).build()`, drive it with `app.update()` (or `gdtf_test_utils::advance_until`), and assert on `State<…>` / the `World`. Scene / state / app **behavioral** logic — transitions, `OnEnter`/`OnExit` wiring, a system's effect on the `World` — is now a *required* headless integration test, not an optional one. Stub only true externals; never shadow the unit under test. Running the app (`cargo run -p grimdark_turfwar`) and observing behavior (the app can capture its own screenshot) is reserved for the genuinely unautomatable: actual rendering / visual correctness, real input, font / layout — and richer automation of *those* is **TBD (Bevy harness)**.
- **Guard coverage is deliberate.** Null/empty tuning, malformed arrays, out-of-bounds cells, degenerate directions — each hardening rule carries an assert, so a refactor can't quietly drop a guard.

## What the suite pins (and what it doesn't)

It pins **behavioral contracts**: the march's exact cell walk, the strictly-higher-sails-over clearance rule, the per-hit damage formula, the severity bucket edges' *application* (not their magnitudes), event-emission discipline, the no-target-stop rule, dead-centre at cone 0, and the id-not-value boundary between sim and presenter. It does **not** pin tuning magnitudes — those live in tuning-config data and are expected to change. It does **not** assert on rendered pixels or exact `Transform` values beyond what a behavioral contract requires.
