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

### Suite scope (docs-only skip)

Scope is agent judgment in the `/gate` skill (and re-checked by `/land`), not a script. Default **FULL**. **DOCS** only when every changed path is allowlisted markdown under `docs/`, `.claude/`, or the repo root — except `docs/tooling/qa-commands.md` and this file, which always force FULL. Pre-commit always runs its cargo subset. This does not change the eight-command list above.

## Rules

1. **Done = full green observed after the final edit.** Anything less is in progress.

2. **New behavior needs real-path tests.** Same systems/components the app runs; never a reimplementation inside the test. Stub only true externals. Sim logic in `gdtf_battle_sim` is unit-testable with injected seeded RNG. New behaviour ships with its **own new tests** in the same change — that is required, not optional.

3. **Do not change production code and the existing tests that cover it in the same change.** A test is only evidence while it stays independent of the change it judges. Edit both together and a real regression gets absorbed by an "updated" expectation: suite green, behaviour broken.

   **Not permitted:** production code plus edits to existing covering tests' expectations — including "the test failed after my refactor so I adjusted what it expects." A gate repair-loop failure is surfaced, not absorbed by rewriting the test.

   **Permitted:** production-only changes (leave existing tests alone); test-only changes (add coverage, repack/move tests without changing what they assert); **new** tests for **new** behaviour in the same change as that behaviour (rule 2).

   **The line is whether the assertion changed**, not whether the test file was touched. Mechanical follow-ons are fine (path, type rename, import). Changing expected values, relaxing bounds, deleting a case, adding `#[ignore]`, or feature-gating a test out alongside the production change that made it fail is not.

4. **Scene / state / app behavior** needs a headless integration test (`gdtf_test_utils::GdtfTestAppBuilder`) asserting on `State` / `World`. Reserve live app runs (`cargo drun`) for rendering, real input, and layout.

5. **Report failures verbatim** — paste the assert, compiler, or clippy output. That is evidence, not prose (see `plain-language.md`).

6. **An implementer's prose report is not evidence. The diff and the working tree are.** Verifiers, gate lenses, and land steps read what actually changed (`git status`, `git diff`, `git diff --stat`, the files) and judge that. This is how land already worked on GTW-882, GTW-905, and GTW-880 (2026-07-29): the tree was right, the report was wrong (both understating and overstating, including a pasted grep that did not match what shipped). Codifying that practice so it is required, not luck.

   - Report vs tree disagreement is not accepted or "reconciled." The tree wins; the discrepancy is **reported** (what the report said, what the tree shows, which was used) — not silently fixed.
   - Quoted greps, line cites, and file excerpts inside a report are still report text; look at the tree.
   - File lists and line counts are re-derived from the tree, never taken from the report.

7. **`/gate` is the gatekeeper** — this suite plus the design-gate audit. No gate-pass, no commit.

8. **Do not pin changeable literals in tests.** If an ordinary content or tuning edit (new weapon file, renamed stem, magnitude tweak) turns a test red, the test is pinning a changeable literal — assert the **property** instead (non-empty registry, deserializes a `Cone` somewhere, gate waits on the resource). Exact filenames, counts, and magnitudes belong in content data, not in `assert!`. Dedicated guard crates under `gdtf_test_utils/tests/` are the exception: they pin repo structure on purpose.
