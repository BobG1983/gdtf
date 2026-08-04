---
name: linear-labels-are-team-scoped
description: Always pass team GDTF to list_issue_labels — an unscoped call omits the team labels and still reports hasNextPage false, which reads as a complete list and is not
metadata:
  type: feedback
---

Always call `list_issue_labels` with `team: GDTF`. Without the team filter the GDTF team labels are
silently omitted, and the reply still says `hasNextPage: false`.

**Why:** `hasNextPage: false` on an unscoped call means "no more workspace labels", not "no more
labels". It reads as a complete list and is not. This already caused a real error: one agent filed a
docs ticket and reported "there is no Documentation label in this workspace", declining to apply one
— while another agent had applied that exact label minutes earlier. The second was right.

**How to apply:**

- Pass `team: GDTF` on every label call.
- Read `.claude/rules/linear-discipline.md` (Labels section) instead of trusting a query. It is the
  authority for the full label set and for each label's meaning, who applies it, and what removes it.
  `.claude/agents/project-manager.md` mirrors the same list under "Common Tags" and now states
  team-scoped labels only, with the `team: GDTF` requirement written in.
- Never invent a label. A real new team label means adding it in Linear and documenting it in
  `linear-discipline.md` in the same change.

Related: [[comment-before-you-close-linear]].
