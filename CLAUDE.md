# gdtf

A turn-based tactics **situation generator** (Necromunda campaign × XCOM) — grimdark.
Core loop: `fight → consequences on survivors → carry the scarred roster forward → fight again, changed`.
This is the **Rust + Bevy 0.18** (0.18.1) rewrite of the Godot game *grimdark* — same design, new engine.
GDTF = **GrimDark TurF war**. Design canon lives in [`docs/`](docs/index.md).

The **work queue lives in Linear** (project **GDTF**; tickets prefixed **GTW-**). Discover the owning
team via the Linear MCP (the team that owns project GDTF) — don't hardcode a team name. Read / add /
update tasks and pick the next one through the Linear MCP; don't hand-edit task state elsewhere.

## Process

The dev loop is **`/next-task` → build → `/gate` → `/land`**: pick a ticket onto its own
`feature/gtw-N-slug` branch, build it, gate it (the green suite below + a 3-lens `design-gate`
fan-out — fidelity / tests / structure+Bevy, merged any-non-compliant-blocks — auditing the diff
against the ticket and `docs/`), then land it onto `develop` and close the ticket. Found a defect?
`/file-bug` **before** fixing. Docs drifted? `/docs-sync`. Kit sanity? `/health-check`.

Orchestration is **Claude Code Workflows(Ultracode) / on-demand sub-agents** — there is no persistent team. A
workflow step (the main session) spawns a sub-agent per need; the sub-agent does its slice and reports
back, and the orchestrating workflow relays to the user. Favor that fan-out over doing review work
inline — e.g. `/gate` and `/health-check` spawn THREE parallel read-only sub-agents in one message.
Sub-agents that review stay read-only and never spawn further sub-agents — the orchestrating step fans
out and relays. Commit subjects: `Area: summary (GTW-N)`.

Binding rules live in `.claude/rules/` — short, read them, follow them:

- [`design-fidelity.md`](.claude/rules/design-fidelity.md) — build exactly what's specified; `docs/` + the ticket are the contract; deviations approved **before** building; Linear "Done" is not evidence.
- [`verification.md`](.claude/rules/verification.md) — the one suite that defines green; tests exercise the real path; view/runtime work needs in-engine evidence.
- [`git-workflow.md`](.claude/rules/git-workflow.md) — branch per ticket, gate-pass commits, explicit staging, land = finish + push.
- [`linear-discipline.md`](.claude/rules/linear-discipline.md) — every change has a GTW-* ticket; statuses move with the work; bugs filed before fixing.
- [`bevy-traps.md`](.claude/rules/bevy-traps.md) — Rust/Bevy ECS gotchas (system ordering, change detection, schedules, state transitions, query conflicts).
- [`no-bare-types.md`](.claude/rules/no-bare-types.md) — no bare Rust/std type for a domain value; wrap each in a named newtype that `Deref`s to it.

## The one definition of green

Run from the repo root; green = **all** pass (these are the `.cargo/config.toml` aliases):

```bash
cargo fmt --check
cargo dclippy -- -D warnings
cargo dtest
cargo dbuild
```

`dclippy` / `dtest` / `dbuild` expand to the full `--workspace --all-targets --features
grimdark_turfwar/dynamic_linking` forms; `dbuild` (`build -p grimdark_turfwar --features
dynamic_linking`) builds+links the actual binary, which `clippy`/`test` never do — so it's the only
step that catches an `unreachable_pub`/link error in the `grimdark_turfwar` binary. Dynamic linking
keeps the dev/gate loop fast; `--all-features` is not used — it forces a second full bevy build for no
lint gain. CI green is **static** (no `dynamic_linking`): fmt/clippy/test only — the release-binary
build (`cargo build -p grimdark_turfwar --release`) is deferred to packaging, not a CI gate. See
[`verification.md`](.claude/rules/verification.md).

The workspace `Cargo.toml` denies clippy `all`/`pedantic`/`correctness`/`suspicious` plus
`unwrap`/`expect`/`panic`/`todo`/`unimplemented` and `missing_docs`, so **lint-clean and fmt-clean
are part of "done"**. (`cargo nextest run` is an optional faster swap for the test step.)

## Commit guard

Commits are guarded by `.claude/hooks/pre-commit-gate.sh` (PreToolUse hook on Bash, paths via
`$CLAUDE_PROJECT_DIR`): a `git commit` on `develop`/`main`, with a red suite, or without a fresh
gate-pass (`.claude/.gate-pass` written by `/gate`, matching the current branch + HEAD) is blocked,
loudly. Don't work around it — run `/gate`.

## Git workflow — git flow

This repo uses **git flow** (initialized: `feature/*` off `develop`, `develop` off `main`).
`main` = releases, `develop` = integration. Do **not** commit features straight to `main` or `develop`.

- New work:    `git flow feature start <gtw-N-slug>`  → branch `feature/<gtw-N-slug>` off `develop`
- Finish work: `git flow feature finish <gtw-N-slug>` → rebases onto & merges into `develop`, deletes the branch
- Fixes:       `git flow bugfix start/finish <name>` (off `develop`)
- Releases:    `git flow release start/finish <x.y.z>` (merges to `main` + `develop`, tags)
- Hotfixes:    `git flow hotfix start/finish <name>` (off `main`)

Commit only when asked. Features rebase onto `develop` on finish, so keep feature branches local until
intentionally shared.

## Run it

```bash
cargo dbuild                                               # debug build
cargo drun                                                 # fast dev iteration
cargo run -p grimdark_turfwar --release                    # release (no dynamic linking)
```

`grimdark_turfwar` is a thin wrapper that calls `GdtfApp::new().run()`. **Bevy dynamic linking** is an
opt-in `dynamic_linking` feature (binary → `gdtf_app` → `bevy/dynamic_linking`) for fast dev builds —
**never** enable it for release artifacts. There is no Godot, no `.tscn`, no MCP-driven editor.

## Project structure

A cargo workspace (resolver 3), `crates/*` + `bins/*`:

```text
crates/gdtf_app/             the Bevy App
  src/app/gdtf_app.rs        GdtfApp wrapper: init_state::<AppState>() + add ScenesPlugin
  src/states/app_state.rs    AppState enum: Init, Load, Intro, MainMenu, Playing, Teardown
  src/scenes/<scene>/        one module per scene; plugin.rs (OnEnter/OnExit) + systems/
  src/scenes/plugin.rs       ScenesPlugin registers every scene plugin
crates/gdtf_battle_sim/      the AUTHORITATIVE, render-free combat sim (the MODEL)
crates/gdtf_battle_presenter/  the VIEW/presenter that mirrors the sim
bins/grimdark_turfwar/       binary entry point (src/main.rs)
docs/                        design canon (pillars/, combat/, decisions/ ADRs, glossary, litmus-tests)
```

The **sim** (`gdtf_battle_sim`) is the source of combat truth — render-free, deterministic, unit-testable
with injected seeded RNG. The **presenter** (`gdtf_battle_presenter`) is a pure view that mirrors sim
state; it never owns combat rules. Keep that one-way dependency: presenter reads the sim, the sim never
reads the presenter. The `gdtf_app` scene plugins drive `AppState`; heavy scenes route through `Load`.

## Conventions (Rust + Bevy)

- **Clippy-clean** under the strict workspace lints (`-D warnings`); `cargo fmt` formatted.
- **No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`** — handle `Option`/`Result` explicitly (these are denied).
- **Doc every `pub` item** (`missing_docs = "deny"`); doc-comment the *why*, not the obvious.
- **Typed everything** — model domain state in real types; no bare rust types ever.
- **Bevy ECS idioms** — small focused systems; `Query`/`Commands`/`Res`/`ResMut`; state via `States` +
  `OnEnter`/`OnExit`; per-scene work behind its scene plugin. No global mutable singletons.

Engine/ECS gotchas (system ordering, change detection, schedule placement, state-transition timing,
query disjointness) are **binding rules**, not lore — they live in
[`bevy-traps.md`](.claude/rules/bevy-traps.md). Read it before adding systems or wiring schedules.
