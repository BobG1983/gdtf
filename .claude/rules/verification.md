---
paths:
  - "**/*"
---

# Verification — the definition of done

Done means the suite was observed green in **this** session, after the final edit. "It should work" is not evidence.

## The one definition of green

**This file is the only authority.** Skills, agents, workflows, and `CLAUDE.md` point here. Do not invent a shorter or longer suite.

Run from the repo root. Green means every command below exits 0. Use the `.cargo/config.toml` **aliases** — never hand-type the expanded feature lists.

```bash
cargo fmt
cargo dclippy -- -D warnings
cargo dtest
cargo dbuild
cargo doc --workspace --no-deps
cargo doc-full
```

| Alias | Purpose |
|-------|---------|
| `fmt` | Formatting is part of done. |
| `dclippy` | Workspace clippy + unwrap/expect/panic/todo + missing_docs. Features: dynamic_linking, dev_tools. |
| `dtest` | Same feature set. Zero tests in a target is still exit 0. |
| `dbuild` | Links the real `grimdark_turfwar` binary (check/clippy never link it). |
| `doc` | Default-feature rustdoc. Workspace rustdoc lints are deny. |
| `doc-full` | Same + dev_tools so feature-gated modules are checked. |

QA modules compile under `debug_assertions` (no `net_qa` feature). Protocol schema derives are always on (no `schema` feature / no separate schema suite steps).

`cargo nextest run` may replace the test step when available; default is `cargo dtest`.

### CI green (static)

CI does not use `dynamic_linking` or `dev_tools`. See `.github/workflows/`.

Release binary builds are packaging-time only (not a CI gate).

### Pre-commit subset

`.claude/hooks/pre-commit-gate.sh` runs a **fast subset** (`fmt`, `dclippy`, `dtest`, `dbuild`) as a deterministic backstop. Full green is still every command above, via `/gate`.

### Suite scope (docs-only skip)

Scope is agent judgment in the `/gate` skill (and re-checked by `/land`), not a script. Default **FULL**. **DOCS** only when every changed path is allowlisted markdown under `docs/`, `.claude/`, or the repo root — except this file, which always forces FULL. Pre-commit always runs its cargo subset. This does not change the list above.

## Rules

1. **Done = full green observed after the final edit.** Anything less is in progress.
2. **New behavior needs real-path tests.** Same systems/components the app runs; never a reimplementation inside the test. Stub only true externals. Sim logic in `gdtf_battle_sim` is unit-testable with injected seeded RNG.
3. **Scene / state / app behavior** needs a headless integration test (`gdtf_test_utils::GdtfTestAppBuilder`) asserting on `State` / `World`. Reserve live app runs (`cargo drun`) for rendering, real input, and layout.
4. **Report failures verbatim** — paste the assert, compiler, or clippy output.
5. **`/gate` is the gatekeeper** — this suite plus the design-gate audit. No gate-pass, no commit.

6. **A flaky test is a broken test.** A test that passes only sometimes was never green, and a passing re-run is not evidence — it is the same test disagreeing with itself. Fix the cause so it cannot race: remove the clock, the shared path, the ordering assumption. A wider timeout, a sleep, a retry, or a larger random range makes a collision less likely and is therefore the wrong fix. If you cannot make it deterministic, say so and file it; never re-run until it passes and report green.

7. **Prove it the cheap way first.** An integration test is about as good as a unit test; both beat reading the code; every one of them beats building tooling to demonstrate behaviour. **Read the existing tests before writing anything** — the invariant you are about to prove is often already asserted, and a numeric difference is not a defect until you have grepped the tests for it. Building a harness to show what a test already covers is wasted work, and a clause whose evidence depends on winning a race is a wrong clause.

8. **Do not pin changeable literals in tests.** If an ordinary content or tuning edit (new weapon file, renamed stem, magnitude tweak) turns a test red, the test is pinning a changeable literal — assert the **property** instead (non-empty registry, deserializes a `Cone` somewhere, gate waits on the resource). Exact filenames, counts, and magnitudes belong in content data, not in `assert!`. Dedicated guard crates under `gdtf_test_utils/tests/` are the exception: they pin repo structure on purpose.

### Gate-pass fingerprint

`.claude/.gate-pass` records the tree `/land` is committing. **`/land` writes it; `/gate` does not.** Fields: `TICKET`, `BRANCH`, `HEAD`, `FINGERPRINT`, `SCOPE`.

`FINGERPRINT` is a content hash of the **uncommitted tree** (staged + unstaged + untracked names) relative to `HEAD`. It is **not** a hash of `HEAD` itself — `HEAD` is a separate field. Pre-commit only checks `BRANCH` / `HEAD` (ancestor-or-equal after land's per-concern commits). `/land` runs this command on the tree it is about to commit and writes the result, so the file says what was committed. Nothing re-checks it: `/docs-sync` runs between `/gate` and `/land` and normally moves the tree. The check that the committed tree is good is `/land` re-running the full suite on it.

**The one command** (run from the repo root — nowhere else invents a recipe):

```bash
{ git diff HEAD; git ls-files -o --exclude-standard; } | shasum -a 256 | awk '{print $1}'
```

Write the 64-character hex only, e.g. `FINGERPRINT=a1b2c3…`. `/land` cites this section; do not invent a different hash, path list, or tool.
