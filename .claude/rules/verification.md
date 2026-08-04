---
paths:
  - "**/*"
---

# Verification — the definition of done

Done means the suite was observed green in **this** session, after the final edit. "It should work" is not evidence.

## The one definition of green

**This file is the only authority.** Skills, agents, workflows, and `CLAUDE.md` point here. Do not invent a shorter or longer suite.

Run from the repo root. Green = all eight exit 0. Use the `.cargo/config.toml` **aliases** — never hand-type the expanded feature lists.

```bash
cargo fmt --check
cargo dclippy -- -D warnings
cargo dtest
cargo dbuild
cargo doc --workspace --no-deps
cargo doc-full
cargo clippy-schema -- -D warnings
cargo test-schema
```

| Alias | Purpose |
|-------|---------|
| `fmt --check` | Formatting is part of done. |
| `dclippy` | Workspace clippy + unwrap/expect/panic/todo + missing_docs. Features: dynamic_linking, dev_tools, both net_qa. |
| `dtest` | Same feature set. Zero tests in a target is still exit 0. |
| `dbuild` | Links the real `grimdark_turfwar` binary (check/clippy never link it). |
| `doc` | Default-feature rustdoc. Workspace rustdoc lints are deny. |
| `doc-full` | Same + dev_tools + both net_qa so feature-gated modules are checked. |
| `clippy-schema` | Package-scoped (`-p gdtf_qa_protocol --features schema`). |
| `test-schema` | Same package scope. |

`cargo nextest run` may replace the test step when available; default is `cargo dtest`.

### CI green (static)

CI does not use `dynamic_linking` or `dev_tools`. See `.github/workflows/`. Guards under `crates/gdtf_test_utils/tests/ci_workflow_features/` fail if workflow feature flags drift.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --features grimdark_turfwar/net_qa,gdtf_content_editor/net_qa -- -D warnings
cargo test --workspace --features grimdark_turfwar/net_qa,gdtf_content_editor/net_qa
cargo clippy -p gdtf_qa_protocol --all-targets --features schema -- -D warnings
cargo test -p gdtf_qa_protocol --features schema
```

Release binary builds are packaging-time only (not a CI gate).

### Pre-commit subset

`.claude/hooks/pre-commit-gate.sh` runs a **fast subset** (`fmt`, `dclippy`, `dtest`, `dbuild`) as a deterministic backstop. Full green is still the eight commands above via `/gate`.

## Rules

1. **Done = full green observed after the final edit.** Anything less is in progress.
2. **New behavior needs real-path tests.** Same systems/components the app runs; never a reimplementation inside the test. Stub only true externals. Sim logic in `gdtf_battle_sim` is unit-testable with injected seeded RNG.
3. **Scene / state / app behavior** needs a headless integration test (`gdtf_test_utils::GdtfTestAppBuilder`) asserting on `State` / `World`. Reserve live app runs (`cargo drun`) for rendering, real input, and layout.
4. **Report failures verbatim** — paste the assert, compiler, or clippy output.
5. **`/gate` is the gatekeeper** — this suite plus the design-gate audit. No gate-pass, no commit.
