# gdtf

A turn-based tactics **situation generator** (Necromunda campaign × XCOM) — grimdark.
Core loop: `fight → consequences on survivors → carry the scarred roster forward → fight again, changed`.
Built in **Rust + Bevy 0.19**. Design canon lives in [`docs/`](docs/index.md).

Work queue is Linear (project **GDTF**, tickets **GTW-***). Use the Linear MCP; do not hardcode team names or hand-edit task state.

## Process

Dev loop: **`/next-task` → build → `/gate` → `/docs-sync` → `/land`**.

- Pick a ticket onto `feature/gtw-N-slug`.
- Build it.
- `/gate` (green suite + design-gate audit).
- `/docs-sync` if docs drifted.
- `/land` onto `develop` and close the ticket.

Found a defect? `/file-bug` before fixing. Kit sanity? `/health-check`.
Autonomous loop tick? `/heartbeat`.

Orchestration uses on-demand sub-agents. Favor fan-out (e.g. `/gate` and `/health-check` spawn parallel read-only design-gate lenses). Sub-agents that review stay read-only. Commit subjects: `Area: summary (GTW-N)`.

No agent has a memory store. Durable knowledge goes in `.claude/rules/`, the agent definition, or `docs/` — all of which are actually read.

## Binding rules

Short files under `.claude/rules/` — read and follow them:

- [`design-fidelity.md`](.claude/rules/design-fidelity.md) — build exactly the ticket + `docs/`; deviations approved before building.
- [`verification.md`](.claude/rules/verification.md) — **the one definition of green**.
- [`git-workflow.md`](.claude/rules/git-workflow.md) — branch per ticket, gate-pass commits, explicit staging, land = finish + push.
- [`linear-discipline.md`](.claude/rules/linear-discipline.md) — every change has a GTW-* ticket; statuses move with the work; labels defined there.
- [`no-bare-types.md`](.claude/rules/no-bare-types.md) — no bare Rust/std type for a domain value; named newtype that `Deref`s.
- [`module-layout.md`](.claude/rules/module-layout.md) — module is a directory; mod.rs is wiring-only; size limits.
- [`plain-language.md`](.claude/rules/plain-language.md) — plain wording and length; name the real mechanism.
- [`reply-shape.md`](.claude/rules/reply-shape.md) — chat reply structure: answer first, no process narration.
- [`comment-hygiene.md`](.claude/rules/comment-hygiene.md) — short docs; no ticket ids in comments.
- [`code-navigation.md`](.claude/rules/code-navigation.md) — symbol questions go to the LSP; a grep count is not a caller count.
- [`cargo-commands.md`](.claude/rules/cargo-commands.md) — every cargo command is an alias from `.cargo/config.toml`, including a single test.
- [`bevy-systems.md`](.claude/rules/bevy-systems.md) — SystemParam / QueryData / split; no too_many_arguments expects on systems.
- [`background-work.md`](.claude/rules/background-work.md) — never poll; sub-agents always run backgrounded; relay every result.
- [`qa-mcp-access.md`](.claude/rules/qa-mcp-access.md) — drive the running app only through the `mcp__gdtf-qa__*` tools; never a socket.

Bevy ECS gotchas (system ordering, change detection, schedules, state transitions, query conflicts) live in the `bevy-expert` agent and supporting notes — treat them as binding when writing systems.

## The one definition of green

**[`.claude/rules/verification.md`](.claude/rules/verification.md) — go read it.**

## Commit guard

`.claude/hooks/pre-commit-gate.sh` blocks commits on `develop`/`main`, red suite, or missing/stale gate-pass. Run `/gate`.

## Git workflow

- New work: `git checkout develop && git pull && git checkout -b feature/gtw-N-slug`
- Finish: rebase onto `develop`, `merge --ff-only`, push, delete feature branch (see `/land`)
- Commit only when asked. Keep feature branches local until shared.

## Run it

```bash
cargo dbuild
cargo drun
```

## Project structure

Cargo workspace (`crates/*` + `bins/*`). Bevy app is `crates/gdtf_app`. Sim (`gdtf_battle_sim`) is the source of combat truth — render-free, deterministic. Presenter reads the sim; the sim never reads the presenter.

## Conventions

- Clippy-clean under workspace lints (`-D warnings`); `cargo fmt` formatted.
- No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`.
- Doc every `pub` item.
- Typed everything — no bare domain values.
- Bevy ECS idioms: small focused systems, `Query`/`Commands`/`Res`, `States` + `OnEnter`/`OnExit`.
