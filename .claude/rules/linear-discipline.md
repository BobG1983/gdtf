# Linear discipline

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

The board is Linear, project **GDTF**, tickets prefixed **GTW-**. Find the owning team through
the Linear MCP. Do not hardcode a team name. Operate the board through those tools inside
the relevant workflow step: never a markdown kanban, never a hand-tracked TODO list, never a
persistent PM agent.

## Rules

1. Every change has a GTW-* ticket. No ticket → create one (via `/next-task` or the Linear
   MCP) BEFORE touching code.
2. Statuses move with the work: → In Progress when you branch, → In Review at `/gate`, → Done
   only after `/land` completes. Never pre-mark Done; never leave a landed ticket open.
3. Decisions and evidence live on the ticket: approved deviations, gate results, the
   green-suite result, QA screenshots and repro notes. A future reader must be able to audit
   the claim from the ticket alone.
4. Bugs are filed BEFORE the fix is committed (`/file-bug`), including bugs you found yourself
   and intend to fix immediately. The fix commit references the bug ticket. Normally that means
   filing before writing the fix. The one case where the code comes first is the carried fix in
   [design-fidelity.md](./design-fidelity.md): a build that hits a defect blocking one of its own
   clauses fixes it in place, and a project-manager step of the same run files the ticket before
   the land step commits it.
5. "Done" on the board is a claim. Audit the code before relying on it (see
   `design-fidelity.md`, rule 3).

## Every comment an agent writes says which agent wrote it

The MCP posts as the account owner, so the author field cannot tell an agent's note from the
user's ruling. **Open every comment with a source line and nothing above it:**

```
**[clause-audit]**
```

Use the agent's own name: `[project-manager]`, `[clause-audit]`, `[design-gate]`. A workflow
posting as itself names the step instead, for example `[build-ticket / land]`.

There is no `[user]`. Everything written through the MCP is an agent, so a comment with no
header is one the user typed into Linear themselves. That holds only from here on. Every
existing one has no header and is still ambiguous.

A ruling still needs its provenance in the body: "user ruling, given directly in conversation,
YYYY-MM-DD". The header says who typed it, not who decided it.

New ticket descriptions carry no header. Every description is agent-written.

## Point at a symbol or quote the text, never a bare line number

A ticket outlives the line numbers in it. Locate things the way a reader can still find them
after the file moves.

Name the symbol. `mode_segment_write` lives in the action bar's mode panel, and the `LSP` tool
finds it wherever it went. Or quote the line of code or prose you mean. A quote survives an
edit above it, a rename, and a file split.

A line number may follow the symbol as a hint, `fire_mode.rs:100`. It is never the only
locator, and never the thing a clause is written against.

A citation that has drifted looks exactly like one that has not, so re-resolve it every time.

The same holds for a `docs/` citation, and for any file:line an agent hands back in a report
that is about to become a clause.

## Linear queries omit data silently

**Never report absence from a default fetch.**

1. Labels need the team. Without `team: GDTF` you get only the workspace labels and
   `hasNextPage: false`, which reads as a complete list and is not.
2. Relations need `includeRelations: true`. Without it `get_issue` returns no blocks,
   blocked-by or related-to edges at all, so a ticket with relations looks exactly like a
   ticket without any.
3. Text search never reaches archived issues, and `includeArchived: true` does not fix it. The
   flag admits archived issues to enumeration, not to the query. Done tickets are archived
   routinely to stay under the workspace cap, so **most of the board cannot be found by
   searching**. To cover archived work, enumerate by state instead, and say in the report which
   half you swept.

## Never cite a Linear comment as a user decision

Cite one only when you can find that decision in the conversation. An agent's own note is not
evidence that anything was agreed.

## Ticket size: prefer the larger ticket

Do not split a ticket because it looks big. Gating costs about the same whatever the ticket
holds, so a small ticket pays a full gate bill for less delivered work.

The build workflow's fix loop absorbs size. It reads the ticket, builds, verifies, runs three
or four gate lenses, repairs what they find, and repeats. A long ticket goes round that loop
more times. It does not need a second ticket.

So `Needs Splitting` is for a ticket that cannot be built as one thing. A ticket that is merely
long is built as it is. Two signs it cannot be built as one thing:

- Its clauses cover work that cannot land together. A half-applied crate rename does not
  compile, so it is one ticket. Four unrelated editor families each with their own delete rule
  are four.
- A clause is blocked on something the others are not, so the whole ticket waits on it.

Owner ruling, given directly in conversation 2026-09-04: "our workflow is good enough to do
slightly larger tasks that you might think because of the fix loop. It's actually better in
terms of cost/ticket to have slightly larger tickets, because of the ratio of
tokens_spent_implementing:tokens_spent_checking."

This does not loosen [clause-writing.md](./clause-writing.md). One clause is still one
requirement, and a clause needing "and also" is still two clauses. Many clauses on one ticket
is the shape this rule asks for.

## Labels

Labels state facts about a ticket. Inventing a label in chat is forbidden. Creating a new team
label requires writing its meaning here in the same change.

If the user has created a label the tables below do not list, update them before continuing and
tell the orchestrator.

### Kind (what the work is)

| Label | Meaning | Who applies | What removes it |
| --- | --- | --- | --- |
| Bug | Broken, wrong, or regressed behaviour | Anyone filing a defect | Ticket Done, or cancel if not a bug |
| Feature | New product behaviour | Author of the ticket | Ticket Done / canceled |
| Improvement | Better existing behaviour, not a new feature | Author | Ticket Done / canceled |
| Hygiene | Internal quality only: tooling, docs, agent process, tests, build speed. No user-visible product change | Author | Ticket Done / canceled |
| Tech Debt | Known debt to pay down | Author | Ticket Done / canceled |
| Documentation | Docs-only (canon, guides) | Author | Ticket Done / canceled |
| AI Workflow | Agent loop, skills, memory, process. Not product MCP commands | Author | Ticket Done / canceled |
| MCP | QA MCP host, mcp channel, protocol/transport, evidence over the wire | Author | Ticket Done / canceled |
| Editor | Content editor (bevy_egui) work | Author | Ticket Done / canceled |
| Art | Hand-authored art assets | Author | Ticket Done / canceled |
| Content | Game content: authored data files, plus the sim or editor change a content batch directly needs. It may be applied alongside `Feature` or `Improvement` when the content needs code. Lives in the Content Authoring tree | Author | Ticket Done / canceled |
| MVP | On the critical path to the first playable cut | Author / prioritisation | Ticket Done / canceled, or scope leaves MVP |

### Size / hierarchy

| Label | Meaning | Who applies | What removes it |
| --- | --- | --- | --- |
| Mythos | Top thematic pillar. Never build directly; work child Epics | Author when filing the pillar | Superseded / canceled. Children being Done does not remove it while the pillar remains |
| Epic | Multi-piece chunk. Never build directly; work children | Author | Superseded / canceled |

### Process flags (assert future work)

| Label | Meaning | Who applies | What removes it |
| --- | --- | --- | --- |
| Needs Splitting | Cannot be built as one thing, because its clauses cannot land together or one of them is separately blocked. Not for a ticket that is merely long: see [Ticket size](#ticket-size-prefer-the-larger-ticket) | Author when the ticket cannot be built whole | Children exist and parent is only a rollup, **or** ticket superseded / canceled. Not only when Done. |
| Needs User Input | Blocked on a decision only the user can make. Ticket stays Backlog; question is a comment on the ticket | Agent or author when stuck | User answers on the ticket, or the ticket moves on. Do not leave it on after the answer, or after the work is superseded or canceled. |
| DO NOT CLOSE | A standing bucket Epic that collects children and never finishes. Never move it to Done, Canceled or Duplicate: closing a parent auto-completes and archives its open children, and every child being closed is not a reason. Not a Mythos marker; it sits on the bucket Epics, not on their parent | Author when creating a standing bucket | Only the user, retiring the bucket by hand. Never "Ticket Done / canceled" |
| Needs Design | The design is not settled enough to split or build. The Epic stays Backlog until the design is written | Author when filing | The design recorded in docs/ or on the ticket; usually hands over to `Needs Splitting` |
| Needs Research | A question must be answered before design can start. Today every ticket with this label also carries `Needs Design` and `Needs Splitting` | Author when filing | The answer recorded on the ticket |

**A ticket carrying `Needs Splitting` is not built.** Split it first with
[`../workflows/split-ticket.js`](../workflows/split-ticket.js), which proposes splits, argues
them to consensus, maps every parent clause to a child that owns it, and files the children
with their edges. The children are what get built. The parent stays as the rollup.

### Informal / not team labels

Do not invent workspace labels (e.g. Enhancement, Chore, Easy, Refactor) for GDTF. Use
Improvement, Hygiene, or Feature instead.

## The MCP block

### Format

```
## MCP surface
Host: game | editor | both | none
What the player/author can newly do: …
MCP: grow <existing command> | add <new command> | none because <one line>
Drive: the mcp__gdtf-mcp path that proves it
```

### Rules

0. Anything a player or an author can do, the matching host MCP **MUST** also be able to do
   (game or editor). A ticket that creates the capability adds the command with it.
1. Favor new commands over growing existing commands.
2. Scope a command to a screen, ganger or state, so it is only available where it is valid.
3. Return the minimum required. A `list inventory` command takes a ganger and returns that
   ganger's inventory, not every ganger's.
4. Add the `MCP` label only to tickets that target ONLY MCP functionality.
5. `none` is for hygiene, sim-only, docs, or a bug that adds no new verb, and must say why.
   Blank is not `none`.
6. Filing a `Feature`, `Editor` or `Improvement` ticket without an MCP block is a
   **VIOLATION** and must be refused.

ALL OF THIS FILE MUST BE FOLLOWED WITHOUT DEVIATION.
