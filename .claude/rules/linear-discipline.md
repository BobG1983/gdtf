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

## Labels

Labels state facts about a ticket. Inventing a label in chat is forbidden.
Creating a new team label requires writing its meaning here in the same change.

When listing labels via Linear MCP, pass the GDTF team (e.g. `team: GDTF`).
Without a team filter, team-scoped labels are silently omitted.

### Kind (what the work is)

| Label | Meaning | Who applies | What removes it |
| --- | --- | --- | --- |
| Bug | Broken, wrong, or regressed behaviour | Anyone filing a defect | Ticket Done, or cancel if not a bug |
| Feature | New product behaviour | Author of the ticket | Ticket Done / canceled |
| Improvement | Better existing behaviour, not a new feature | Author | Ticket Done / canceled |
| Hygiene | Internal quality only: tooling, docs, agent process, tests, build speed. No user-visible product change | Author | Ticket Done / canceled |
| Tech Debt | Known debt to pay down | Author | Ticket Done / canceled |
| Documentation | Docs-only (canon, ADRs, guides) | Author | Ticket Done / canceled |
| AI Workflow | Agent loop, skills, memory, process — not product MCP commands | Author | Ticket Done / canceled |
| MCP | QA MCP host, net_qa channel, protocol/transport, evidence over the wire | Author | Ticket Done / canceled |
| Editor | Content editor (bevy_egui) work | Author | Ticket Done / canceled |
| Art | Hand-authored art assets | Author | Ticket Done / canceled |
| MVP | On the critical path to the first playable cut | Author / prioritisation | Ticket Done / canceled, or scope leaves MVP |

### Size / hierarchy

| Label | Meaning | Who applies | What removes it |
| --- | --- | --- | --- |
| Mythos | Top thematic pillar. Never build directly — work child Epics | Author when filing the pillar | Superseded / canceled; children Done does not auto-remove if pillar remains |
| Epic | Multi-piece chunk. Never build directly — work children | Author | Superseded / canceled |

### Process flags (assert future work)

| Label | Meaning | Who applies | What removes it |
| --- | --- | --- | --- |
| Needs Splitting | Too big to build as-is; children not yet filed | Author when size is wrong | Children exist and parent is only a rollup — **or** ticket superseded / canceled. Not only when Done. |
| Needs User Input | Blocked on a decision only the user can make. Ticket stays Backlog; question is a comment on the ticket | Agent or author when stuck | User answers on the ticket **and** this label is removed (or ticket moves on). Do not leave it on after the answer. |

### User decisions

To get a decision: put the question on the ticket as a comment, apply
**Needs User Input**, leave status Backlog. The answer must be written on the
ticket before the label comes off. Do not keep the label after the work is
superseded or canceled.

### Informal / not team labels

Do not invent workspace labels (e.g. Enhancement, Chore, Easy, Refactor) for
GDTF. Use Improvement, Hygiene, or Feature instead. If a real new label is
needed, add it in Linear and document it in this section in the same change.
