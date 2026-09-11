---
paths: [".claude/workflows/**", ".claude/agents/**"]
---

# Model tiering: the tier is set where the call is made

Every `agent()` call in a workflow script names its model. The agent definitions
in `.claude/agents/` all carry `model: opus` in frontmatter, and a call-site
`model:` overrides it.

## The three tiers

| Tier | Work | Examples |
|------|------|----------|
| `fable` | Very hard, or creative. Inventing from a blank page, or rulings where the answer cannot be derived | split proposals, proposal revision, requirement-vs-directive rulings |
| `opus` | Hard engineering. Building, verifying, auditing, judging against code | the engineer, clause-audit, gate lenses, vote lenses, land |
| `sonnet` | Mechanical. Compare X with Y, copy text from place to place, read a file into a schema | Linear fetches, status moves, filing finished text, recording votes |

## The tie-breaker

A call between two tiers takes the higher one.

Use `sonnet` only for steps where being wrong is cheap and visible.

## When adding an agent() call

Set `model:` explicitly, and add the call to the classification table at the top
of the workflow file. A call with no `model:` inherits the session model,
whatever the user happens to be running.
