---
name: run-state
description: >-
  Read and rewrite .claude/run-state.md. One numbered step per section, each saying
  when to act, what to write and what to trim.
argument-hint: ""
---

# /run-state — read and rewrite the run-state file

The file is [`.claude/run-state.md`](../../run-state.md). It is **gitignored and machine-local**,
so a tick on another machine starts blind and nothing here survives a wiped checkout.

**If the file is missing or unreadable, say so and stop.** Do not start a build, and do not
recreate the file from memory — a session that cannot see what is in flight is the one that starts
a second build on top of a live one.

Before the steps, three things that hold everywhere:

- The `## Headings` are fixed. Never add, remove, reorder or reword one. New headings go **inside**
  a section as `###` beneath it.
- A section you have nothing to add to is left exactly as it is. Rewriting a section to say the
  same thing differently is churn, and it hides what actually changed.
- [`plain-language.md`](../../rules/plain-language.md) applies to every line you write here.

## Rewrite the run-state file by doing the following

**Step 1 — read it first.** Read the whole file before writing anything. Several steps below need
to count existing entries, and a blind append is how a capped list grows to sixteen.

**Step 2 — Cron State.** Did you create or recreate a cron this tick? If not, leave the section
alone. If you did, update its row with the new id. The table is one row per live cron; anything
`CronList` does not show is stale and comes out.

**Step 3 — In Flight.** This section is a **replace, not an append.** Rewrite it to describe only
what is live right now. If a build is running, that means the ticket, the branch, the workflow run
id, the exact resume command, and anything the next tick would otherwise have to rediscover —
including any user ruling attached to the ticket that a builder could undo by accident. If nothing
is running, cut it to one line saying so and naming what comes next. Delete anything (including notes) about work that has landed; that belongs in the Delivery Log or on the ticket. Do note store the details in memory, memory goes stale and is not durable.

**Step 4 — Delivery Log.** Did a ticket land since the last write? If not, leave the section alone.
If it did, add one line at the top:
`YYYY-MM-DD-HH:MM  GTW-n  one sentence saying what the ticket was.`
Then count the lines — if there are 11, delete the bottom one.

**Step 5 — The Queue.** Did the board's order change — a ticket landed, was filed, was cancelled,
or gained a blocking edge? If not, leave the section alone. If it did, rewrite the affected lines in
the form `GTW-n one sentence of detail Blocked By:… Blocks:…`, ordered by dependency rather than by
number. Drop any ticket that has landed. Cap the list at 20; if it would be longer, keep the ones
nearest to being worked and say in a line beneath that the rest are on the board.

**Step 6 — Gotchas This Run.** Did anything this run behave in a way that cost time and would cost
it again? If not, leave the section alone. If it did, check whether the list already holds that
gotcha: if so, increment its count and move it to the top; if not, add it at the top as
`While working on GTW-n, <what happened>. The solution was: <what to do>. This gotcha has been seen 1 time.` one single, short, concise, sentence following the plain language rule.
Then two trims, in this order. If any entry now reads more than 5 times, take it out of the list and
either file it as a bug, if it is a defect in the code, the rules or the workflow, or write it to
memory if it is not. If the list is longer than 10, delete the oldest entry showing 1 time.

**Step 7 — Tick Log.** Are you writing this as part of a heartbeat tick? If not, leave the section
alone — it is a record of ticks, not of every edit. If you are, add one line at the top:
`YYYY-MM-DD-HH:MM one sentence describing the state you found and what you did about it.`
Name the cron id, the run id of anything in flight, where `develop` is, and whether you started a
build. Then count the lines — if there are 6, delete the bottom one.

**Step 8 — User Directed Notes.** **Do not touch this section** unless the user directed a note in
this conversation, or a ticket has landed.

If the user did direct a note, add it under a heading in the form
`### Short Description - Agreed YYYY-MM-DD - Delete after landing GTW-n`.
Write what was decided and why, and the ticket the note should be deleted after (GTW-n), in enough detail that someone who was not in the conversation could act on it.

If a ticket has landed, check if any note says `Delete after landing GTW-n` for that ticket. If so, delete the note.

## Do not

- Write a User Directed Note the user did not direct.
- Append to a capped list without counting it afterwards.
- Leave the In Flight section describing work that has finished.
- Restate a format rule from this file anywhere else. This skill owns them; a second copy drifts,
  and the copy people find first is usually the wrong one.
