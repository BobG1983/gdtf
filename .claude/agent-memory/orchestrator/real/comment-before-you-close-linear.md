---
name: comment-before-you-close-linear
description: Never call Linear from the main session — dispatch project-manager — and post the reason on a ticket before changing its status, because archived issues reject comments
metadata:
  type: feedback
---

Never call `mcp__linear-server__*` from the main session. Dispatch the `project-manager` agent
(`.claude/agents/project-manager.md`) — not even for one small urgent fix. And write the reason on a
ticket **before** you change its status.

**Why:** closing or cancelling can archive an issue, and archived issues reject `save_comment` (they
still accept `save_issue`). There is no un-archive, so an explanation written after the state move is
simply lost. `.claude/workflows/build-ticket.js` routes every Linear read and write through
`agentType: 'project-manager'` for the same reason — one writer, and the main session never
hand-edits board state.

**How to apply:**

- Every ticket report must include the comment thread, not just the description and status. A user's
  answer posted as a comment is otherwise silently missed.
- Archiving silently drops dependency edges. Linear auto-archived three completed issues mid-session
  and one took its `blocks` relation with it — no notice, no record. Re-read the edges after any
  archive wave rather than trusting an earlier read, and never let a completed ticket's relation
  carry sequencing information; put it in the surviving ticket's text.
- Linear also flips status on its own. A ticket auto-moved to In Progress during an unrelated
  relation edit. Verify status after any bulk board change; do not assume your last write is what is
  there.
- Linear holds ONE relation per pair. Adding "related" over an existing blocking edge silently
  replaces the block. Check existing edges before adding any relation.
- Close cascades run both ways and are not deterministic. Marking a parent Done can auto-complete
  open children seconds later with no action taken; cancelling the last open child can auto-Cancel
  the parent, sometimes followed by a bulk archive. Check children before AND after closing a parent,
  and reparent survivors out first.
- A question asked only in chat is lost if the user is away. Put it on the ticket as a comment plus
  the `Needs User Input` label with status left in Backlog — the flow
  `.claude/rules/linear-discipline.md` specifies — and remove the label only once the answer is
  written on the ticket.
- Statuses move with the work: In Progress when the branch starts, In Review at the gate, Done only
  after landing (`.claude/rules/linear-discipline.md` rule 2). Linear cannot backdate a transition,
  so fixing this afterwards records when you corrected the board, not when the work happened — the
  status step has to run at the front of the build.

Related: [[you-may-push-and-edit-linear]], [[linear-labels-are-team-scoped]],
[[build-it-in-a-workflow]].
