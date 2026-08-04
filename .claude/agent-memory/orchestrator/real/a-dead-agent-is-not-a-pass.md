---
name: a-dead-agent-is-not-a-pass
description: Workflow control flow keys off agent prose — null-check before matching, take the first standalone verdict token, fail closed when none appears, and hand every reviewer the evidence it is forbidden to gather itself.
metadata:
  type: feedback
---

A workflow decides what to do next by reading agent prose. Parse it defensively and fail closed.
Every rule here comes from a finished, correct ticket being reported as failed, or a dead agent
being read as a pass.

**Why:** `agent()` returns `null` when a call dies, the prompt text an agent may echo contains the
same verdict words its report will use, and the harness can hand a workflow script its `args` as a
JSON string instead of an object. Each of those turns a plausible-looking check into a wrong
answer, and the wrong answer that costs most is the one that reads silence as success.

**How to apply:**

- **Guard `args` first.** `.claude/workflows/build-ticket.js:22` is the pattern:
  `if (typeof A === 'string') { A = JSON.parse(A) }`. Without it `A?.verify` is `undefined` and
  the word "undefined" gets interpolated into every downstream prompt, so lenses review with no
  evidence and say so. After interpolating, check the prompt does not contain the literal
  "undefined" where evidence belongs.
- **Null-check before you string-match.** `readVerdict` (`:226`) and `compliant` (`:266`) both
  open with `if (!out) return false`. A `!out.includes('RED')` check reads a dead agent as a pass.
- **Take the first standalone token.** `/\b(GREEN|RED)\b/i.exec(out)`, then compare. Never anchor
  with `^` — "start with GREEN on its own line" is a request, and agents prepend preamble. Never
  use a bare substring — `/RED/` matches "REQUIRED", "PREDICATE", "FILTERED". For the lens verdict
  put the negative first in the alternation, `/\b(NON-COMPLIANT|COMPLIANT)\b/i`, or every block
  reads as a pass. No verdict at all means the agent verified nothing: fail.
- **Bound the search to the report's opening.** The shipped matchers scan the whole output, and
  the prompt itself carries both tokens, so an agent that echoes its instructions can false-pass
  on the echo. Searching the first ~500 characters closes that: a real report leads with its
  verdict, an echo does not reach its own verdict sentence that early.
- **A dead reviewer blocks; only a silent round aborts.** `compliant(null)` is false, so a dead
  lens sends the run into a repair round rather than through the gate. The run throws only when
  *no* reviewer reported at all (`reviewersHeardFrom() === 0`, `:276`) — that is an infrastructure
  failure to resume, not a result. Keep both behaviours.
- **Hand a reviewer what it is forbidden to gather.** Gate lenses run zero cargo, so the verify
  report is interpolated into every lens prompt with an explicit null branch (`:249-251`,
  `'(verify died — treat cargo claims as UNPROVEN)'`). Audit every "take this as given" line in a
  prompt: each one is a promise something else supplies it. Fix the plumbing, never the rigour —
  a lens that refuses to certify unproven work is doing its job.
- **Lens agents sometimes die returning nothing, and the cause is unknown.** Several theories were
  stated as root cause and each was refuted. Say "unknown", record the score, and do not repeat a
  cause the evidence only correlates with. Keep `agentType: 'design-gate'` on the lenses (`:257`)
  — that is a user ruling; removing it loses the reviewer persona and its accumulated memory.
- **No schema on reasoning-heavy phases.** Prose-heavy design and plan steps break
  schema-validated output — the JSON truncates mid-string, and tightening `maxLength` does not
  fix it. Return plain text.

Related: [[build-it-in-a-workflow]], [[resume-dont-restart-a-dead-run]],
[[forward-evidence-never-assert-a-pass]].
