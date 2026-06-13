# `.claude/validation/` — what each doc validates

These are validation / self-check notes for the gdtf Claude Code kit (the
skills, rules, agents, and hook under `.claude/`). They are NOT runtime code and
NOT transcripts of real ticket runs — they exist to prove the kit is internally
consistent and obeys the gdtf adaptation spec (Rust + Bevy 0.18, Linear project
GDTF with `GTW-` tickets, the Workflow orchestration model, and the one
definition of green: `cargo fmt --check` + `cargo clippy … -D warnings` +
`cargo test --workspace`, dynamic-linked via `grimdark_turfwar/dynamic_linking`).

## Docs

- **`frontmatter-lint.md`** — validates the YAML frontmatter of every skill,
  agent, and rule against the harness frontmatter field sets and line caps:
  skills (`name`, `description`, `when_to_use`, `argument-hint`,
  `allowed-tools`, `model`, …), agents (`name`, `description`, `tools`,
  `model`, `memory`, `maxTurns`), rules (`paths` only). Confirms required
  fields are present, no unknown fields leak in, and the terse line caps hold
  (skills 120–150, rules 40–50, CLAUDE.md ≤200).

- **`hook-dry-run.md`** — validates `.claude/hooks/pre-commit-gate.sh` by
  exercising its decision logic without running `cargo`: commit detection
  (including `git -C <dir> commit` and quoted/wrapped forms), the
  develop/main branch guard, the `.claude/.gate-pass` presence + branch +
  ancestor-HEAD guard, and the green-suite gate via the
  `PRE_COMMIT_GATE_SUITE_CMD` test seam (`=true` green, `=false` red). Confirms
  the hook blocks (exit 2) and allows (exit 0) in the right cases and fails
  OPEN only on an unreadable payload.

- **`loop-walkthrough.md`** — a design-trace of one hypothetical small `GTW-`
  ticket through `/next-task` → implement → `/gate` → `/land` under the
  Workflow model. Validates that the four skills, the rules, the `.gate-pass`
  fingerprint, and the pre-commit hook compose into a coherent loop, and that
  the defect lessons inherited from grimdark each stay owned by some step (the
  fingerprint not perturbing itself, covering untracked content, a single owner
  of the In-Review transition, and only `/land` committing).
