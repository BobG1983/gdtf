# Plain language — no jargon in tickets, code, or comments

Why this rule exists: words like "seam" and "sanctioned" spread through Linear
tickets, then got copied verbatim into a shipped code comment
("returning its HONEST receipt"). Jargon written in a planning doc doesn't stay
there — an agent building from that ticket repeats it in the code. Plain
words describing the actual mechanism don't have this problem.

## Banned outright — never use these, anywhere (tickets, code, comments, chat)

- **"seam"** as a stand-in for a queue, API, or integration point. Name the
  real thing: "the input queue", "the drain", `PendingActIntent`, the actual
  type or function.
- **"sanctioned"**. Drop the adjective. Say what the code does — "the same
  push path local UI uses" — without an approval flourish.
- **"byte identical" / "byte-identical"**. Say what's actually true:
  "identical", "unchanged", "matches exactly", or name the specific
  comparison being made.

## The general principle behind the ban

- No metaphor standing in for a technical term (seam, spine, backbone, rail).
  Name the actual mechanism.
- No institutional or value-laden adjective standing in for a plain
  description (sanctioned, canonical-when-unnecessary, blessed).
- No capitalized word used as rhetorical emphasis instead of a real
  qualifier (HONEST, GENUINE, REAL used as tone, not content).
- If a sentence reads the same or clearer with the impressive word deleted,
  delete it.

## Where this applies

- Linear ticket titles, descriptions, and comments.
- Code comments and doc comments.
- Commit messages.
- Assistant chat output.

## Non-scope

This rule governs writing going forward. It does not, by itself, require
rewriting already-shipped code comments — that sweep is tracked separately
(GTW-751).
