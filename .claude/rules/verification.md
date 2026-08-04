---
paths:
  - "**/*"
---

# Verification — the definition of done

Done means the suite was observed green in **this** session, after the final edit. "It should work" is not evidence.

## The one definition of green

**This file is the only authority.** Skills, agents, workflows, and `CLAUDE.md` point here. Do not invent a shorter or longer suite.

Run from the repo root. Green = all six exit 0. Use the `.cargo/config.toml` **aliases** — never hand-type the expanded feature lists.

```bash
cargo fmt --check
cargo dclippy -- -D warnings
cargo dtest
cargo dbuild
cargo doc --workspace --no-deps
cargo doc-full
```

| Alias | Purpose |
|-------|---------|
| `fmt --check` | Formatting is part of done. |
| `dclippy` | Workspace clippy + unwrap/expect/panic/todo + missing_docs. Features: dynamic_linking, dev_tools. |
| `dtest` | Same feature set. Zero tests in a target is still exit 0. |
| `dbuild` | Links the real `grimdark_turfwar` binary (check/clippy never link it). |
| `doc` | Default-feature rustdoc. Workspace rustdoc lints are deny. |
| `doc-full` | Same + dev_tools so feature-gated modules are checked. |

QA modules compile under `debug_assertions` (no `net_qa` feature). Protocol schema derives are always on (no `schema` feature / no separate schema suite steps).

`cargo nextest run` may replace the test step when available; default is `cargo dtest`.

### CI green (static)

CI does not use `dynamic_linking` or `dev_tools`. See `.github/workflows/`.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Release binary builds are packaging-time only (not a CI gate).

### Pre-commit subset

`.claude/hooks/pre-commit-gate.sh` runs a **fast subset** (`fmt`, `dclippy`, `dtest`, `dbuild`) as a deterministic backstop. Full green is still the six commands above via `/gate`.

### Suite scope (docs-only skip)

Scope is agent judgment in the `/gate` skill (and re-checked by `/land`), not a script. Default **FULL**. **DOCS** only when every changed path is allowlisted markdown under `docs/`, `.claude/`, or the repo root — except `docs/tooling/qa-commands.md` and this file, which always force FULL. Pre-commit always runs its cargo subset. This does not change the six-command list above.

## Rules

1. **Done = full green observed after the final edit.** Anything less is in progress.
2. **New behavior needs real-path tests.** Same systems/components the app runs; never a reimplementation inside the test. Stub only true externals. Sim logic in `gdtf_battle_sim` is unit-testable with injected seeded RNG.
3. **Scene / state / app behavior** needs a headless integration test (`gdtf_test_utils::GdtfTestAppBuilder`) asserting on `State` / `World`. Reserve live app runs (`cargo drun`) for rendering, real input, and layout.
4. **Report failures verbatim** — paste the assert, compiler, or clippy output.
5. **`/gate` is the gatekeeper** — this suite plus the design-gate audit. No gate-pass, no commit.

6. **Do not pin changeable literals in tests.** If an ordinary content or tuning edit (new weapon file, renamed stem, magnitude tweak) turns a test red, the test is pinning a changeable literal — assert the **property** instead (non-empty registry, deserializes a `Cone` somewhere, gate waits on the resource). Exact filenames, counts, and magnitudes belong in content data, not in `assert!`. Dedicated guard crates under `gdtf_test_utils/tests/` are the exception: they pin repo structure on purpose.
