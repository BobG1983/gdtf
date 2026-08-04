# Write Like an Engineer, Not a Consultant

Why this rule exists: jargon written into a ticket spreads. It has been copied
into shipped code comments ("returning its HONEST receipt") and into landed
commit subjects (e.g. `507edbbe`). Once it is in the board or the history, the
next ticket copies it again. Write plain now so the cost does not compound.

Writing exists to communicate, not to sound sophisticated. Prefer judgment over
an ever-growing banned-word list. The Avoid examples below are illustrations,
not a checklist to expand.

This file owns wording and length. Reply structure in chat is `reply-shape.md`.

## Where this applies

- Ticket titles, descriptions, and comments
- Code comments and doc comments
- Commit subjects and bodies
- Assistant chat output
- Design docs and ADRs you write or edit

## Rules

- Prefer plain English over technical-sounding language.
- Use technical terms only when they are the correct, precise term.
- State what changed before explaining why.
- Write the shortest text that conveys all necessary information.
- Delete filler, narration, and obvious observations.
- Never use abstractions where a concrete noun or verb works.
- Assume the reader is competent. Do not over-explain.

## Quoted evidence is not prose

`verification.md` requires failures reported **verbatim** — paste the assert,
compiler, or clippy output whole. That paste is evidence, not the writing this
rule trims. Do not summarize a failure to sound brief. Do not weaken the
verbatim rule.

## Avoid

Do not inflate ordinary ideas into architectural prose.

Bad:

- "The seam between these components..."
- "The integration surface..."
- "The operating envelope..."
- "This enables a first-class experience..."
- "Leverage..."
- "Mental model..."
- "Canonical..." / "Orthogonal..." used as decoration
- "Byte-identical..." (unless literal bytes are being discussed)
- "Paradigm", "synergy", "holistic", "robust" when a simpler word is accurate

Instead write what actually happens.

Bad: "Introduce a typed boundary between the host and the application."
Good: "The host calls the app through a typed interface."

Bad: "This change reduces the cognitive load required to reason about the system."
Good: "This makes the code easier to understand."

Bad: "We leverage a shared abstraction."
Good: "Both systems use the same interface."

Every sentence should earn its place. If removing it loses no information, remove it.

## The "Normal Engineer" Test

Would an experienced engineer say this in a code review? If not, rewrite it.

Write like you're explaining the code to a teammate sitting next to you, not
presenting at a software architecture conference.
