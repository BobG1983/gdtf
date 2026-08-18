---
paths: [".claude/workflows/**", ".claude/agents/**"]
---

# Model tiering — the tier is set where the call is made

Every `agent()` call in a workflow script names its model. The agent definitions 
in `.claude/agents/` all carry `model: opus`in frontmatter, and a call-site `model:` 
overrides it — so the call site is the only place a tier is actually decided.

## The three tiers

| Tier | Work | Examples |
|------|------|----------|
| `fable` | Very hard, or creative — inventing from a blank page, rulings where the answer is not derivable | split proposals, proposal revision, requirement-vs-directive rulings |
| `opus` | Hard and engineering-focused — building, verifying, auditing, judging against code | the engineer, clause-audit, gate lenses, vote lenses, land |
| `sonnet` | Mechanical — compare X with Y, copy text from place to place, read a file into a schema | Linear fetches, status moves, filing finished text, recording votes |

## The tie-breaker

A call between tiers takes the **higher** tier. Measured before this ruling and surviving it:
sonnet on real work costs more than opus, because turns-to-correct outweigh the cheaper run.
`sonnet` is only for steps where being wrong is cheap and visible.

## When adding an agent() call

Set `model:` explicitly and add the call to the classification table at the top of the
workflow file. A call with no `model:` inherits the session model, which is whatever the
user happens to be running — not a decision, an accident.
