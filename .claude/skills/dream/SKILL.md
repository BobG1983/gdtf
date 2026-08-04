---
name: dream
description: >-
  Distil agent memory: promote verified facts from ephemeral/ into real/, then combine,
  trim and delete until each store is small enough to read before working. Re-check every
  claim against the tree. Runs on a dream/ branch in the main repo; /heartbeat owns its cron.
argument-hint: ""
---

# /dream — distil agent memory

Layout: [`.claude/agent-memory/`](../../agent-memory/).

**The job is distillation, not deduplication.** A store nobody reads before working has
cost with no benefit. The last run promoted 81 files into one agent's store — every one
verified, and the result still unreadable. Ending with fewer, sharper memories than you
started with is a good outcome.

## The bar

For every memory, kept or written, ask:

> Would this change what someone does on a problem they have never seen?

If not, it is a story, not a memory. Delete it. Specific cases that always fail the bar:

- A log of what happened on one ticket, or which findings were closed on it.
- A redirect stub pointing at another file.
- A fact recoverable in seconds from the code, git history, or the board.
- A lesson whose mechanism no longer exists.

## Where this runs

- On a branch in the main repo, named `dream/` or `feature/dream-` so `/heartbeat` does
  not read it as a ticket build. No worktrees — see [`git-workflow.md`](../../rules/git-workflow.md).
- Never write real/ from a dirty main tree mid-ticket.
- **The cron is not optional and this skill does not own it.** `/heartbeat` recreates it —
  see its cron table for the schedule. `/dream` cannot restore its own: it only runs when
  something invokes it, so once the cron is gone nothing brings it back.

## Pass 1 — Verify and promote

1. List every immediate child of `.claude/agent-memory/` that is a directory with
   `ephemeral/` and `real/`.
2. For each agent, read every file under `ephemeral/` (skip empty / `.gitkeep`).
3. Re-check every claim against the tree, `docs/`, or the board. **Open the cited
   file:line.** A memory is a claim about a tree that has moved since it was written, and
   line numbers rot faster than anything else in it.
4. Judge by mechanism, not by prose quality:
   - Mechanism gone → **delete**, whatever the writing is like.
   - Mechanism alive, citations rotted → rewrite against current code, then promote.
   - Claim unverifiable read-only → leave in ephemeral. Never promote on faith.
5. Promote into `real/` in the sibling format.
6. Never delete without reporting what went and why.

## Pass 2 — Combine and cut

This is where the store gets smaller. Be aggressive.

1. **Combine on shared mechanism, not shared wording.** Two memories describing one
   mechanism from different angles are one memory. They will not look like duplicates —
   different evidence, different symptoms, different names.
2. **Extract the lesson from the incident.** Most files are one war story plus one general
   rule. The rule is what lasts. Keep the rule, cut the story to the minimum that makes it
   checkable, and drop the rest.
3. **Trim hard.** A lesson that takes forty lines will not be read at the moment it is
   needed. If it cannot be stated in a short paragraph plus a how-to-apply line, it is
   probably two lessons — split it, then cut both.
4. **Merged files get a new descriptive name** naming the mechanism, not either original.
5. Re-check any merged claim before writing it. Drop false ones with a one-line reason.
6. Delete redundant ephemeral files and report them.

Two memories that give **contradictory verdicts on the same shape** are the highest
priority merge — a reviewer who reads one and not the other gets the wrong answer, and
which one they hit is luck.

## Pass 3 — Index

Each agent's `real/` carries **its own index**, listing only that agent's memories. There
is no shared root index — an agent should not read past three other agents' entries to
find its own.

Rewrite each index from the files' own frontmatter rather than by hand, so it cannot drift
from what is actually there.

## Report (required)

Counts: **promoted**, **merged**, **deleted**, **left alone**, and the store size before
and after. If a store did not shrink, say so and say why — that is the number this skill
exists to move. If ephemeral is empty, say so; do not invent promotions.

## Out of scope

Game code changes. Ticket builds. Editing `qa-protocol/` design records.
