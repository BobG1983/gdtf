# Pin: no git-flow in process surface

Process files must not reintroduce `git flow` / `git-flow` / `gitflow` commands.

Scan (from repo root):

```bash
rg -n 'git flow|git-flow|gitflow' CLAUDE.md .claude/rules .claude/skills .claude/agents .claude/workflows .claude/hooks
```

Expect **zero** hits that instruct running `git flow`. Historical mentions in `validation/` docs may remain until rewritten.

Plain-git start/finish are defined in `.claude/rules/git-workflow.md`.
