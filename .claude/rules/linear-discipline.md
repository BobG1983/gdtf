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

## Every comment an agent writes says which agent wrote it

The MCP posts as the account owner, so the author field cannot tell an agent's note from the
user's ruling. **Open every comment with a source line and nothing above it:**

```
**[clause-audit]**
```

Use the agent's own name — `[project-manager]`, `[clause-audit]`, `[design-gate]` — or the step
when a workflow posts as itself, e.g. `[build-ticket / land]`.

There is no `[user]`. Everything written through the MCP is an agent. A comment the user made is
one they typed into Linear themselves, and it never passes through this tool — so an unheadered
comment is theirs. That only holds for comments written from here on; every existing one is
unheadered and still ambiguous.

A ruling still needs its provenance in the body — "user ruling, given directly in conversation,
YYYY-MM-DD" — because the header says who typed it, not who decided it.

New ticket descriptions do not carry a header; a description is understood to be agent-written.

## Point at a symbol or quote the text — never a bare line number

A ticket outlives the line numbers in it. Locate things the way a reader can still
find them after the file moves:

- **The symbol.** `mode_segment_write` in the action bar's mode panel.
  The `LSP` tool finds it wherever it went.
- **The text.** Quote the line of code or prose you mean. A quote survives an edit
  above it, a rename, and a file split.

A line number may ride along as a hint — `fire_mode.rs:100` after the symbol — but it
is never the only locator, and never the thing a clause is written against.

This is measured, not cautious. GTW-1148's verify report quoted two failures at
`fire_one_spec.rs:208` and `:240` while the final file held them at `:212` and `:244` —
four lines had been inserted above them between the run and the report.

What makes it worse is that you cannot tell by looking. An edit that replaces text within
a line moves nothing; an edit that adds one line moves everything below it. So a citation
is neither trustworthy nor obviously broken — it has to be re-resolved, every time, which
is the cost a symbol or a quote does not have.

The same holds for a `docs/` citation, and for any file:line an agent hands back in a
report that is about to become a clause.

## Linear queries omit silently — an empty field may mean you did not ask

Three of them, all the same shape: the call succeeds, reports no error, and leaves data
out. **Never report absence from a default fetch.**

1. **Labels need the team.** Without `team: GDTF` you get only the workspace labels and
   `hasNextPage: false`, which reads as a complete list and is not. The team-scoped ones
   are missing. This has already produced two agents contradicting each other about
   whether a label exists — the one that scoped the query was right.
2. **Relations need `includeRelations: true`.** Without it `get_issue` returns no
   blocks / blocked-by / related-to edges at all, so a ticket with relations looks exactly
   like a ticket without any. One agent reported a blocked-by edge "lives only in a
   comment and does not exist"; it existed, reciprocal on both tickets.
3. **Text search never reaches archived issues, and `includeArchived: true` does not fix
   it.** The flag admits archived issues to enumeration, not to the query. Done tickets
   are archived routinely to stay under the workspace cap, so **most of the board cannot
   be found by searching**. To cover archived work, enumerate by state instead — and say
   in the report which half you actually swept.

## A comment written through the MCP wears the user's name

The Linear MCP posts as the account owner, so a comment an agent wrote and a ruling the
user made look identical on the board. **Never cite a Linear comment as a user decision**
unless you can find that decision in the conversation. An agent's own note is not evidence
that a dependency, a design principle, or a constraint was ever agreed.

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
| Documentation | Docs-only (canon, guides) | Author | Ticket Done / canceled |
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
