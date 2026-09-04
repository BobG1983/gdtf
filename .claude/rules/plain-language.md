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

## Banned

### Coined vocabulary

Do not invent a name for something. Use the name Rust, Bevy or this repo already uses.
A `SystemSet` is a system set. `mcp` is the MCP server. A file many modules read
is a file many modules read.

Real failures: "the trapped test's control leg", "load-bearing", "the seam", "a pool
that cannot pay", "the junk gate". Each was invented mid-sentence and then used as if
the reader already knew it.

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

## Why this is strict

Measured across one session on 2026-08-21, the user corrected wording eleven times, and
every correction was one of the bans above. The pattern has a name outside this repo and
a browser extension exists to translate it back into English.

The failure underneath is writing to show the reasoning instead of writing to inform.
Before sending, ask what the reader now knows that they did not before. If the answer is
nothing, delete it.
