---
name: you-may-push-and-edit-linear
description: Standing grants — push to develop when landing, write Linear through project-manager, git reset --hard to recover a broken run — and never present your own inference as a user ruling
metadata:
  type: feedback
---

Standing grants. Do not re-request these per ticket:

- **Push to `develop`** as part of landing gated work.
- **Create, edit, comment on and close Linear tickets** — through the `project-manager` agent, never
  directly from the main session.
- **`git reset --hard`** when recovering a broken run state.

**Why:** these were granted once and are durable. Landing IS a push to `develop`
(`.claude/rules/git-workflow.md` rule 6, `.claude/skills/land/SKILL.md`, and the land phase of
`.claude/workflows/build-ticket.js`), so asking each time stalls the loop for nothing.

**How to apply:**

- The grant removes the question, not the gate. Push and board writes still go through the normal
  loop — gate green first, then land.
- Never present an inference as something the user said or approved. In a ticket comment, a commit
  message, or any other write someone else reads, "the user ruled X" must correspond to the user
  actually ruling X. If it is your reading, say it is your reading.
- An automated hook, a goal-checker, or your own earlier message is **not** user approval. Only the
  user's own words are.

Related: [[comment-before-you-close-linear]], [[dont-retune-what-the-user-tuned]].
