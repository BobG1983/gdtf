---
paths: ["**/*"]
---

# Plain language

Binding on every word written anywhere: chat, code comments, docs, tickets, commit
messages, rules files, agent prompts. [reply-shape.md](./reply-shape.md) owns the
structure of a chat reply. This file owns the words.

## The test

After writing a sentence, ask whether a competent engineer would say it out loud to a
colleague. If not, rewrite it.

Then delete the sentence and ask whether the reader lost anything. If not, leave it
deleted.

Before sending, ask what the reader now knows that they did not before. If the answer is
nothing, delete it.

## Banned

### Anecdotes

Never write the story behind a rule. No "Measured on GTW-1358...", no "an agent once...",
no "why this rule exists: ...", no "because X happened, do Y". State the rule and stop.
If a fact only lives inside the story, state the fact.

This holds everywhere: rules, docs, tickets, commit messages, code comments, chat
replies. A story about an incident pulls the reader onto the incident and teaches them
to write more stories.

Ticket ids do not appear in any file. The only exception is a format placeholder that
stands for any ticket: `GTW-N`, `GTW-*`, `feature/gtw-N-slug`, or a pattern that matches
ticket ids.

A dated owner ruling stays. That is provenance for a decision, not a story. Keep the
ruling and its date, and drop any narrative wrapped around it.

### Coined vocabulary

Do not invent a name for something. Use the name Rust, Bevy or this repo already uses.
A `SystemSet` is a system set. `mcp` is the MCP server. A file many modules read
is a file many modules read.

### Em dashes

Use a full stop. Two short sentences beat one long one.

### "Not X, it's Y"

It sounds like insight and carries none. Say what the thing is.

### Bold lead-in bullets

A list of `**Term:** explanation` lines. This is the most recognisable AI-writing
pattern there is. Write sentences, or use headings.

### Words that only add weight

genuinely, precisely, crucially, fundamentally, essentially, notably, importantly,
ultimately, effectively, arguably, meaningfully, inherently, clearly, obviously, simply.

Delete the word. If the meaning is unchanged, it was one of these. A synonym reached for
instead is the same violation.

### Consultant nouns

seam, surface, envelope, paradigm, synergy, mental model, canonical, orthogonal,
holistic, first-class, leverage, robust. Use the plain word.

### Editorialising

"The genuinely useful catch", "worth knowing", "rough edges worth knowing", "the
interesting part". State the finding. The reader decides whether it is interesting.

### Portentous closers

"That becomes the rule." "The gate held." A last line that sounds like a conclusion and
states nothing. Stop when the information stops.

### Compressed half-sentences

"The clause-audit correction premise." Nouns stacked as adjectives with the verb
removed. Write a sentence with a verb in it.

## Required

Say what changed before why it changed.

Use short sentences. More sentences with plain words beats fewer sentences with dense
ones.

Use the concrete noun and the concrete verb. Never a metaphor where a literal
description exists.

Assume the reader is competent. Do not explain what they already know.

Prefer use over leverage, remove over deprecate, works with over integrates with, limit
over envelope, interface over seam.

## Headings and titles

A heading names its subject. "Authentication", not "Auth, the deliberate gap". "Known
issues", not "Rough edges worth knowing".

A ticket title says what changes, using names from the tree. "Split
`a_held_entry_cell_leaves_the_occupant_stuck` into two tests", not "Split the trapped
test's control leg". A long real name beats a short invented one.
