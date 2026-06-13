---
paths:
  - "**/*"
---

# Linear discipline — the board is the truth of the work

Why this rule exists: tickets marked Done while un-done, and work done with no
ticket at all, make the board useless as evidence. A board only works if it
moves WITH the work, not after it.

The board: Linear, project **GDTF**, tickets prefixed **GTW-** (e.g.
GTW-123). Discover the owning team via the Linear MCP (the team that owns
project GDTF) — do not hardcode a team name. Operate the board through the
Linear MCP tools inside the relevant workflow step — never a markdown kanban,
never a hand-tracked TODO list, never a persistent PM agent.

## Rules

1. Every change has a GTW-* ticket. No ticket → create one (via `/next-task`
   or the Linear MCP) BEFORE touching code.
2. Statuses move with the work: → In Progress when you branch, → In Review at
   `/gate`, → Done only after `/land` completes. Never pre-mark Done; never
   leave a landed ticket open.
3. Decisions and evidence live on the ticket: approved deviations, gate
   results, the green-suite result, QA screenshots / repro notes. A future
   reader must be able to audit the claim from the ticket alone.
4. Bugs are filed BEFORE fixing (`/file-bug`) — including bugs you found
   yourself and intend to fix immediately. The fix commit references the bug
   ticket.
5. "Done" on the board is a claim, not proof — audit the code before relying
   on it (see `design-fidelity.md`, rule 3).
