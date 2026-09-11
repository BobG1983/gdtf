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
cargo mcpbuild
cargo edmcpbuild
```

| Alias | Purpose |
|-------|---------|
| `fmt` | Formatting is part of done. |
| `dclippy` | Workspace clippy + unwrap/expect/panic/todo + missing_docs. Features: `game/development,editor/development`. |
| `dtest` | Same feature set. Zero tests in a target is still exit 0. |
| `dbuild` | Links the real `game` binary (check/clippy never link it). |
| `doc` | Default-feature rustdoc. Workspace rustdoc lints are deny. |
| `doc-full` | Same + `development` on both hosts so feature-gated modules are checked. |
| `mcpbuild` | Release build of `game` carrying only `mcp`. No other listed command compiles that configuration. |
| `edmcpbuild` | Release build of `editor` carrying only `mcp`. Same configuration on the editor side. |

Running one test or one suite uses the same aliases.
[cargo-commands.md](./cargo-commands.md) owns that, and a hook blocks a bare cargo command.

The QA host is gated on each host package's `mcp` feature, which the `development` umbrella
feature turns on. The workspace aliases `dcheck`, `dclippy`, `dtest` and `doc-full` pass
`game/development,editor/development`, so the QA modules and their test suites compile in the
suite. A suite target that needs the feature says so with `compile_error!`, so dropping it
breaks the build instead of running zero tests. Protocol schema derives are always on, so
there is no `schema` feature and no separate schema steps in the suite.

`mcpbuild` and `edmcpbuild` build a host on the release profile with `mcp` and nothing else,
which no other command in the list compiles. A rustc warning fails those two because the root
`Cargo.toml` denies the whole `warnings` group in `[workspace.lints.rust]`. `cargo build` takes
no trailing `-- -D warnings`, so the manifest is the only place that deny can live. It applies
to every cargo command here and to every crate in the workspace, not only to the two release
builds. The first build after that lint level changes rebuilds everything, because an edit to
`[workspace.lints]` invalidates every crate's fingerprint.

`cargo nextest run` may replace `cargo dtest` when available.

### Pre-commit subset

`.claude/hooks/pre-commit-gate.sh` runs a fast subset: `fmt`, `dclippy`, `dtest`, `dbuild`. Full
green is still every command above, run by `/gate`.

`mcpbuild` and `edmcpbuild` are outside that subset and run in `/gate` and `/land`. Release
carries `lto = "thin"` and `codegen-units = 1`, so both cost minutes, and every commit would pay
it. `/land` re-runs the full list on the exact tree it commits, so nothing reaches develop
without both builds passing.

### Suite scope (docs-only skip)

The `/gate` skill decides the scope, and `/land` re-checks it. It is a judgment call, not a
script. The default is **FULL**. Use **DOCS** only when every changed path is allowlisted
markdown under `docs/`, `.claude/`, or the repo root. A change to this file always forces FULL.
Pre-commit always runs its cargo subset. None of this changes the command list above.

## The suite never tests the agent tooling

Nothing under `.claude/` is tested by the green suite. A workflow script that cannot parse refuses to launch and says which file and line, so it needs no guard. Owner ruling, given directly in conversation, 2026-09-10.

## Rules

1. Anything less than full green is in progress.

2. New behavior needs tests that run the app's own systems and components. Never reimplement the
   behavior inside the test. Stub only what lives outside the app. Sim logic in
   `gdtf_battle_sim` is unit-testable with a seeded RNG injected.

3. Scene, state and app behavior need a headless integration test
   (`cobalt_test_utils::MinimalTestAppBuilder`, passing
   `gdtf_game::test_support::register_headless`) that asserts on `State` and `World`. Keep live app
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
   red (a new weapon file, a renamed stem, a magnitude tweak, a reordered collection), the test
   is pinning a changeable literal. Assert the property instead: the registry is non-empty, a
   `Cone` deserializes somewhere, the gate waits on the resource. Exact filenames, counts, and
   magnitudes belong in content data, not in `assert!`. The order items come back in is not
   content data either — assert them as a set, not a sequence. Dedicated guard suites under
   `crates/gdtf_conformance/tests/` are the exception: they pin repo structure on purpose.

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
