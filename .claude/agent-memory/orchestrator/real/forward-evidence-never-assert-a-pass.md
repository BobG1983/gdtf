---
name: forward-evidence-never-assert-a-pass
description: A bare "it passed the gate" in a land prompt reads as a fabricated pass and gets blocked — interpolate the verify report and gate verdicts verbatim, disclose the failed rounds, and let the land step refuse.
metadata:
  type: feedback
---

Never open a land prompt with "it passed the full suite and all three gate lenses." That is an
assertion, not evidence, and a safety check reads an unsupported pass-claim as fabricated —
especially when an earlier run of the same work is on record as blocked.

**Why:** this was earned. A run hit its round cap with one lens blocking and returned
`landed:false`. The cap was raised, the run resumed, and the next round came back green with all
three lenses compliant — genuinely. But the land prompt only *asserted* the pass, so the check
compared that claim against the `landed:false` it could see and blocked, reasoning that a limit
had been bumped without anything being re-run. Its inference was wrong about the facts and
entirely reasonable about the evidence in front of it.

**How to apply:**

- **Fix it in the workflow, not in the wording.** `.claude/workflows/build-ticket.js` interpolates
  the real artifacts into the land prompt (`:334` onward): the verify report inside
  `<verify-report>`, then every lens verdict inside its own `<gate-lens name="…">` block, labelled
  with the final round number.
- **Disclose the failed rounds.** The round count is in the prompt for this reason. A report
  reading "gate passed" when the record shows it passed on the fourth attempt looks like
  concealment even when nothing was concealed. State the history, state what was re-run, and let
  the reviewer weigh it.
- **Give the land step the power to refuse.** It re-runs the full suite itself if develop moved
  and is told to stop if the evidence does not support landing. A land step that can independently
  refuse is not blocked by a check; it satisfies one.
- **Do not let the land agent certify its own landing.** It is told not to claim landing is
  proven; a separate confirm agent fetches `origin/develop`, finds the commit, checks it is an
  ancestor, and replies with one line — `LANDED_COMMIT=<sha>` or `LANDED_COMMIT=NO`. Only that
  reply closes the ticket.
- **When a safety check blocks you:** state the truth, dispatch once more with the full history
  disclosed and the evidence attached, and if it blocks again **stop and ask the user**. Never
  edit the classifier's inputs to slip past it, never hand-dispatch around a negative
  determination, never reach for `--no-verify`.

Same family as [[a-dead-agent-is-not-a-pass]] — in both, the reviewing machinery was sound and the
plumbing around it produced the wrong answer. Fix the plumbing, never the rigour.

Related: [[build-it-in-a-workflow]].
