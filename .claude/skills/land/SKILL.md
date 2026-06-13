---
name: land
description: >-
  Commit gated work, finish the feature branch into develop, push, and close the
  Linear ticket(s) with evidence. Only after /gate passes this session — refuses to
  land otherwise.
argument-hint: "[GTW-N ...]"
---

# /land — finish the feature into develop

Landing is the only path to develop, and it refuses loudly rather than landing dirty.
Giant multi-ticket uncommitted trees and tickets marked Done without evidence are the
failure modes this skill blocks. Binding background: `.claude/rules/git-workflow.md`,
`.claude/rules/linear-discipline.md`. The pre-commit hook (`.claude/hooks/`) and
`.claude/.gitignore` are the companions of `/gate` whose fingerprint this skill
consumes.

Resolve `GTW-N` from the argument, or derive the single ticket from the branch name
(`feature/gtw-N-slug`) and confirm with the user. The argument may name a
space/comma-separated SET of tickets (runtime-coupled work that /gate'd together and
must land together) — it MUST match the `TICKET=` set recorded in `.claude/.gate-pass`.
Single-ticket is the common path.

## Preconditions — check ALL; on any failure, REFUSE, state which one, and stop

1. **Gate passed this session for THIS tree.** `/gate` — the green suite plus the
   design-gate sub-agent's clause-by-clause verdict — must have reported full PASS for
   this GTW-N (or, for a multi-ticket set, for ALL of them) in this conversation, and
   `.claude/.gate-pass` must exist, its `TICKET=` line must name exactly this ticket set
   (comma-separated for a set), and its `FINGERPRINT=` line must equal a fresh
   recomputation of /gate's exact command:

   ```
   { git rev-parse HEAD; git status --porcelain; git diff HEAD; git ls-files -o --exclude-standard -z | LC_ALL=C sort -z | xargs -0 -r shasum -a 256; } | shasum -a 256 | cut -d' ' -f1
   ```

   (The fingerprint hashes untracked file CONTENT too; `.claude/.gate-pass` itself is
   gitignored so its presence never perturbs the hash.) Any mismatch (edits since
   gating, different ticket, no file) → refuse; run /gate again first.
2. **Suite green NOW.** Run the one definition of green from the repo root
   (`$CLAUDE_PROJECT_DIR`), exact commands:

   ```
   cargo fmt --check
   cargo clippy --workspace --all-targets --features grimdark_turfwar/dynamic_linking -- -D warnings
   cargo test --workspace --features grimdark_turfwar/dynamic_linking
   ```

   (`cargo dclippy` / `cargo dtest` are the shorthand; aliases in `.cargo/config.toml`.)
   Any one not exiting 0 (fmt drift, a clippy warning, a failing test) → refuse.
3. **On a `feature/*` branch.** `git branch --show-current` must be the
   `feature/gtw-N-slug` branch for this ticket. On `develop`, `main`, or anything
   else → refuse.
4. **No files outside the ticket scope.** Walk `git status --porcelain` (tracked AND
   untracked): every modified or untracked file must belong to this ticket (the set
   /gate reviewed). If extras exist, LIST them and ASK the user what to do — never
   stage, stash, or delete them on your own initiative, and never auto-stage anything.
   `.claude/.gate-pass` is gate bookkeeping — never stage it.
5. **Every contract clause implemented.** Partial delivery is not delivery: if any
   clause of the /gate contract is unimplemented, stubbed (`todo!`/`unimplemented!`),
   or deferred "for now" → refuse. There is no "MVP of a ticket" unless the USER
   splits the ticket.
6. **Ticket scope not shrunk mid-session.** Re-pull EACH named ticket via the Linear MCP
   (project **GDTF**, team discovered via MCP); if any description was edited during
   this session in a way that shrinks scope relative to the contract restated at
   /gate time → surface the diff and ASK the user; do not land on the smaller wording.

## Steps

1. **Stage explicit files per concern** — `git add <file> <file> …` by name; NEVER
   `git add -A`, `-u`, or `.`. (Rust sources have no sidecar files to pair, so stage
   only the `.rs`, `Cargo.toml`, `docs/`, and asset files the ticket actually changed.)
2. **Commit in house style**: subject `Area: summary (GTW-N)` plus a wrapped body
   (what changed and why), matching the voice of `git log --oneline -15`; no co-author
   trailers. Separable concerns get separate commits. The PreToolUse pre-commit hook
   under `.claude/hooks/` guards commits — it re-reads `.claude/.gate-pass`; if it
   blocks, do NOT bypass or disable it (no `--no-verify`) — re-run /gate and come back.
3. **Record the target for the range report**: `OLD=$(git rev-parse origin/develop)`.
4. **Finish the feature**: `GIT_EDITOR=true git flow feature finish gtw-N-slug` (the
   name is the branch minus the `feature/` prefix). git-flow is initialized
   (feature/ off develop); this rebases onto develop, merges, and deletes the branch.
   If it halts on conflicts, stop and report — never resolve silently.
5. **Push**: `git push origin develop`.
6. **Close the ticket(s)**: move GTW-N to **Done** via the Linear MCP (project **GDTF**,
   team discovered via MCP) — for a multi-ticket set, move ALL named tickets to Done —
   with a short evidence note on each: landing/merge commit SHA, suite
   result (e.g. "cargo test --workspace: N passed, 0 failed; clippy + fmt clean"), and
   the pushed range. Done without evidence is forbidden.
7. **Report the pushed range** to the user — `git log --oneline OLD..develop` — then
   delete `.claude/.gate-pass`.
