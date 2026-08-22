---
paths:
  - "**/*"
---

# Verification: the definition of done

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Done means the suite was observed green in **this** session, after the final edit. "It should
work" is not evidence.

## The one definition of green

This file is the only authority. Skills, agents, workflows, and `CLAUDE.md` point here. Do not
invent a shorter or longer suite.

Run from the repo root. Green means every command below exits 0. Use the `.cargo/config.toml`
**aliases**. Never hand-type the expanded feature lists.

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

Running one test or one suite uses the same aliases.
[cargo-commands.md](./cargo-commands.md) owns that, and a hook blocks a bare cargo command.

QA modules compile under `debug_assertions`, with no `net_qa` feature. Protocol schema derives
are always on, so there is no `schema` feature and no separate schema steps in the suite.

`cargo nextest run` may replace `cargo dtest` when available.

### Pre-commit subset

`.claude/hooks/pre-commit-gate.sh` runs a fast subset: `fmt`, `dclippy`, `dtest`, `dbuild`. Full
green is still every command above, run by `/gate`.

### Suite scope (docs-only skip)

The `/gate` skill decides the scope, and `/land` re-checks it. It is a judgment call, not a
script. The default is **FULL**. Use **DOCS** only when every changed path is allowlisted
markdown under `docs/`, `.claude/`, or the repo root. A change to this file always forces FULL.
Pre-commit always runs its cargo subset. None of this changes the command list above.

## Rules

1. Anything less than full green is in progress.

2. New behavior needs tests that run the app's own systems and components. Never reimplement the
   behavior inside the test. Stub only what lives outside the app. Sim logic in
   `gdtf_battle_sim` is unit-testable with a seeded RNG injected.

3. Scene, state and app behavior need a headless integration test
   (`gdtf_test_utils::GdtfTestAppBuilder`) that asserts on `State` and `World`. Keep live app
   runs (`cargo drun`) for rendering, real input, and layout.

4. Paste the assert, compiler, or clippy output verbatim.

5. `/gate` runs this suite plus the design-gate audit. No gate-pass, no commit.

6. A flaky test is a broken test. A passing re-run is not evidence. Fix the cause so it cannot
   race: remove the clock, the shared path, the ordering assumption. A wider timeout, a sleep, a
   retry, or a larger random range only makes a collision less likely, so every one of them is
   the wrong fix. If you cannot make it deterministic, you're wrong. Never re-run until it
   passes and then report green.

7. Prove it the cheap way first. An integration test is about as good as a unit test. Both beat
   reading the code. Every one of them beats building tooling to demonstrate behavior. Read the
   existing tests before writing anything. The invariant you are about to prove is often already
   asserted, and a numeric difference is not a defect until you have searched the tests for it.

8. Do not pin changeable literals in tests. If an ordinary content or tuning edit turns a test
   red (a new weapon file, a renamed stem, a magnitude tweak), the test is pinning a changeable
   literal. Assert the property instead: the registry is non-empty, a `Cone` deserializes
   somewhere, the gate waits on the resource. Exact filenames, counts, and magnitudes belong in
   content data, not in `assert!`. Dedicated guard crates under `gdtf_test_utils/tests/` are the
   exception: they pin repo structure on purpose.

### Gate-pass fingerprint

`.claude/.gate-pass` records the tree `/land` is committing. **`/land` writes it; `/gate` does
not.** Fields: `TICKET`, `BRANCH`, `HEAD`, `FINGERPRINT`, `SCOPE`.

`FINGERPRINT` is a content hash of the uncommitted tree (staged, unstaged, and untracked names)
relative to `HEAD`. It is **not** a hash of `HEAD` itself; `HEAD` is a separate field.
Pre-commit checks only `BRANCH` and `HEAD`, ancestor-or-equal, after land's per-concern commits.
`/land` runs the command below on the tree it is about to commit and writes the result, so the
file says what was committed. Nothing re-checks the fingerprint: `/docs-sync` runs between
`/gate` and `/land` and normally moves the tree. Instead, `/land` re-runs the full suite on the
tree it commits.

Run this from the repo root.

```bash
{ git diff HEAD; git ls-files -o --exclude-standard; } | shasum -a 256 | awk '{print $1}'
```

Write the 64-character hex only, e.g. `FINGERPRINT=a1b2c3…`. Do not invent a different hash, path
list, or tool.
