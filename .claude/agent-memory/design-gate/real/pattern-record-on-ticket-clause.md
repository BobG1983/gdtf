---
name: pattern-record-on-ticket-clause
description: A clause with two halves — do X, and record the decision on the ticket — is only half-checkable, because the design-gate agent has no Linear tools.
metadata:
  type: feedback
---

A clause of the form "do X, and record the decision on this ticket" has a code half and a
board half. The code half is checkable first-hand. The board half is not: the design-gate
agent's tool list is `mcp__gdtf-qa__*, Read, Grep, Glob, Bash`
(`.claude/agents/design-gate.md:8`) — no Linear MCP. Everything the agent knows about the
comment thread comes from the contract header it was handed.

**Why the board half is real:** `.claude/rules/linear-discipline.md` rule 3 requires
decisions and evidence to live on the ticket, so a future reader can audit the claim from
the ticket alone. An implementer's report is not that record.

**Why you cannot just block on it:** the contract header is a SNAPSHOT taken when the
orchestrating step fetched the ticket. On a re-gate the comment can already exist while the
header still says the thread is empty. Blocking blind on a stale header is how a satisfied
clause gets ruled non-compliant twice.

**How to apply:**

- First pass, header says the thread is empty: VIOLATION, and say plainly the remedy is a
  Linear comment, not a code change.
- Re-gate: corroborate before re-blocking. The verify report should quote the comment's
  author, timestamp, and the guards it names. Check every named guard in code. If the code
  matches the quoted decision point for point, credit the clause and state that the board
  half was not verifiable with this agent's tools.

Related: [[pattern-unsatisfiable-acceptance-clause]].
