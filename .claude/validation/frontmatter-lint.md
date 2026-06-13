# gdtf kit — frontmatter & cross-reference lint

**Date:** 2026-06-12
**Scope:** `.claude/agents/*.md` (6), `.claude/skills/*/SKILL.md` (7), `.claude/rules/*.md` (5), `CLAUDE.md`
**Checks:** (1) YAML frontmatter vs allowed field sets, (2) line caps, (3) cross-reference resolution, (4) forbidden-token scan.

Forbidden tokens for gdtf are the OPPOSITE of grimdark: `spawn-team` / "teams" / TeamCreate / teammate / team-lead are banned (gdtf orchestrates via Workflows + on-demand sub-agents); the Workflow tool is REQUIRED, not banned.

---

## Result: ISSUES (no must-fix; all findings are line-cap warnings)

Frontmatter is clean on every file. All cross-references resolve. No forbidden orchestration tokens. The only findings are line-cap deviations (1 over, 4 under) — none are malformed-frontmatter, dangling-reference, or forbidden-token defects.

---

## 1. Frontmatter validation

### Agents (allowed: name / description / tools / model / memory / maxTurns)

| File | Keys present | Verdict |
| --- | --- | --- |
| `agents/bevy-expert.md` | name, description, tools, model | PASS |
| `agents/design-gate.md` | name, description, tools, model (`inherit`), memory (`project`), maxTurns (60) | PASS |
| `agents/engineer.md` | name, description, tools, model, memory | PASS |
| `agents/project-manager.md` | name, description, tools, model | PASS |
| `agents/qa.md` | name, description, tools, model | PASS |
| `agents/source-control.md` | name, description, tools, model | PASS |

No unknown keys. `#`-comment lines inside `design-gate.md` frontmatter are valid YAML and ignored.

### Skills (allowed: name / description / when_to_use / argument-hint / disable-model-invocation / user-invocable / allowed-tools / model / context / agent / hooks / paths)

| File | Keys present | Verdict |
| --- | --- | --- |
| `skills/design-notes/SKILL.md` | name, description, when_to_use, argument-hint, allowed-tools, disable-model-invocation, user-invocable | PASS |
| `skills/docs-sync/SKILL.md` | name, description, argument-hint | PASS |
| `skills/file-bug/SKILL.md` | name, description, argument-hint | PASS |
| `skills/gate/SKILL.md` | name, description, argument-hint | PASS |
| `skills/health-check/SKILL.md` | name, description, argument-hint | PASS |
| `skills/land/SKILL.md` | name, description, argument-hint | PASS |
| `skills/next-task/SKILL.md` | name, description, argument-hint | PASS |

No unknown keys.

### Rules (allowed: paths ONLY)

| File | Keys present | Verdict |
| --- | --- | --- |
| `rules/bevy-traps.md` | paths (`"**/*"`) | PASS |
| `rules/design-fidelity.md` | paths (`"**/*"`) | PASS |
| `rules/git-workflow.md` | paths (`"**/*"`) | PASS |
| `rules/linear-discipline.md` | paths (`"**/*"`) | PASS |
| `rules/verification.md` | paths (`"**/*"`) | PASS |

Every rule carries exactly one allowed key. No unknown keys.

---

## 2. Line caps

Caps: skills 120-150, rules 40-50, `CLAUDE.md` ≤ 200.

| File | Lines | Cap | Verdict |
| --- | --- | --- | --- |
| `skills/design-notes/SKILL.md` | 175 | 120-150 | **OVER (warn)** |
| `skills/docs-sync/SKILL.md` | 121 | 120-150 | OK |
| `skills/file-bug/SKILL.md` | 120 | 120-150 | OK (at floor) |
| `skills/gate/SKILL.md` | 95 | 120-150 | **UNDER (warn)** |
| `skills/health-check/SKILL.md` | 122 | 120-150 | OK |
| `skills/land/SKILL.md` | 81 | 120-150 | **UNDER (warn)** |
| `skills/next-task/SKILL.md` | 136 | 120-150 | OK |
| `rules/bevy-traps.md` | 42 | 40-50 | OK |
| `rules/design-fidelity.md` | 50 | 40-50 | OK (at ceiling) |
| `rules/git-workflow.md` | 35 | 40-50 | **UNDER (warn)** |
| `rules/linear-discipline.md` | 32 | 40-50 | **UNDER (warn)** |
| `rules/verification.md` | 49 | 40-50 | OK |
| `CLAUDE.md` | 113 | ≤ 200 | OK |

5 line-cap deviations, all warnings (content is high-signal in every case; these are length-budget nits, not defects).

---

## 3. Cross-reference resolution

Every `.claude/`, `docs/`, skill (`/name`), and agent reference inside the kit files was resolved against disk. All resolve.

**`.claude/` refs — all exist:** `.gitignore`, `agents/design-gate.md`, `hooks/pre-commit-gate.sh`, `rules/{bevy-traps,design-fidelity,git-workflow,linear-discipline,verification}.md`. (`.gate-pass`, `agent-memory/` are runtime/gitignored bookkeeping, correctly not present.)

**`docs/` refs — all exist:** `architecture.md`, `combat/` (`battle-space.md`, `resolution.md`, `stats.md`), `decisions/` + `decisions/index.md`, `glossary.md`, `index.md`, `litmus-tests.md`, `pillars/`. (Placeholders `docs/combat/<topic>.md`, `docs/decisions/NNNN-slug.md` are template tokens, not references.)

**Skills referenced (`/gate`, `/land`, `/next-task`, `/file-bug`, `/docs-sync`, `/design-notes`, `/health-check`)** — all 7 SKILL.md files exist.

**Agents referenced (`design-gate`, `engineer`, `qa`, `project-manager`, `bevy-expert`, `source-control`)** — all 6 agent files exist.

**Source-path refs verified on disk:** `crates/gdtf_app`, `crates/gdtf_battle_sim`, `crates/gdtf_battle_presenter`, `bins/grimdark_turfwar`, `crates/gdtf_app/src/states/app_state.rs` (AppState enum matches: Init/Load/Intro/MainMenu/Playing/Teardown), `crates/gdtf_app/src/app/gdtf_app.rs`, `crates/gdtf_app/src/scenes/plugin.rs`, `bins/grimdark_turfwar/src/main.rs`, `.claude/hooks/pre-commit-gate.sh`.

No dangling references found.

---

## 4. Forbidden-token scan

- **Banned orchestration tokens** (TeamCreate / spawn-team / teammate / team-lead / CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS / SendMessage / `~/.claude-work/teams`): **NONE found** across agents/skills/rules.
- **"the team" / "teammates"**: 3 hits (`project-manager.md`, `next-task/SKILL.md`, `linear-discipline.md`) — all refer to the **Linear team that owns project GDTF**, discovered via the Linear MCP, exactly as the spec requires. Not orchestration teams. Acceptable.
- **Godot tokens** (godot / gdscript / .tscn / res:// / GUT / @tool / ui_accept / TileMap): hits are all legitimate — describing the rewrite origin ("rewrite of a Godot game"), explicitly negating Godot ("there are no `.tscn` files", "no Godot MCP", "NOT `res://test`, NOT GUT"), or the documented re-grounding cheat-sheet table in `docs-sync` (spec-sanctioned for translating ported docs). No fabricated Godot specifics asserted as gdtf reality.

---

## 5. Notes (non-blocking)

- `agents/engineer.md` sets `memory:` to an **absolute machine path** (`/Users/bgardner/.claude-work/projects/-Users-bgardner-dev-gdtf/memory/engineer/`). The key is valid; the value is non-portable (hardcodes a user home, against the `$CLAUDE_PROJECT_DIR` spirit of cross-cutting rule 7) and the directory does not exist yet (the agent body says to create it). Flagged as a warning, not a frontmatter-shape error. `agents/design-gate.md` by contrast uses the portable `memory: project`.
