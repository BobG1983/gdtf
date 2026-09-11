---
name: run-state
description: >-
  Read and rewrite .claude/run-state.md. One numbered step per section, each saying
  when to act, what to write and what to trim.
argument-hint: ""
---

# /run-state: read and rewrite the run-state file

> **You MUST read and follow [plain-language.md](../../rules/plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

The file is [`.claude/run-state.md`](../../run-state.md). It is gitignored and machine-local, so a
tick on another machine starts blind, and nothing here survives a wiped checkout.

If the file is missing or unreadable, say so and stop. Do not start a build, and do not recreate
the file from memory. A session that cannot see what is in flight starts a second build on top of a
live one.

Two things hold everywhere:

- Never add, remove, reorder or reword a `## Heading`. A new heading goes inside a section, as
  `###` beneath it.
- Leave a section you have nothing to add to exactly as it is. Rewriting it to say the same thing
  differently hides what changed.

## The steps

### Step 1. Read the whole file before writing anything

Several steps below count existing entries.

### Step 2. Cron State

If you created or recreated a cron this tick, update its row with the new id. The table holds one
row per live cron, so anything `CronList` does not show is stale and comes out.

### Step 3. In Flight

Replace this section, never append to it. Describe only what is live right now. If a build is
running, write the ticket, the branch, the workflow run id, the exact resume command, and anything
else the next tick would have to rediscover about the work in progress. Do not add notes you want
to keep. If nothing is running, cut the section to one line saying so. Delete everything about work
that has landed, notes included, because that belongs in the Delivery Log or on the ticket. Do not
store these details in memory. Memory goes stale and is not durable.

### Step 4. Delivery Log

If a ticket landed since the last write, add one line at the top:
`YYYY-MM-DD-HH:MM  GTW-n  one sentence saying what the ticket was.`
Then count the lines. If there are 11, delete the bottom one.

### Step 5. The Queue

The board's order changes when a ticket lands, is filed, is cancelled, gains a blocking edge, or
the user gives a new ordering. When it does, rewrite the affected lines in the form
`GTW-n one sentence of detail Blocked By:… Blocks:…`, ordered by dependency rather than by number.
Drop any ticket that has landed. Cap the list at 20. If it would run longer, keep the ones nearest
to being worked and say in a line beneath that the rest are on the board.

### Step 6. Tick Log

Write this section only in a heartbeat tick. It records ticks, not every edit. Add one line at the
top:
`YYYY-MM-DD-HH:MM one sentence describing the state you found and what you did about it.`
Name the cron id, the run id of anything in flight, where `develop` is, and whether you started a
build. Then count the lines. If there are 6, delete the bottom one.

### Step 7. User Directed Notes

Touch this section only if the user directed a note in this conversation, DO NOT ADD ANYTHING ELSE.

If the user directed a note, add it under a heading in the form
`### Short Description - Agreed YYYY-MM-DD - Delete after landing GTW-n`.
Write what was decided and why, in enough detail that someone who was not in the conversation could
act on it.

If a ticket has landed, look for a note saying `Delete after landing GTW-n` for that ticket. If
there is one, delete it.

## Do not

Do not restate a format rule from this file anywhere else. This skill owns them. A second copy
drifts, and the copy people find first is usually the wrong one.
