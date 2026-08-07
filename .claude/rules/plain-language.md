---
paths: ["**/*"]
---

# Write Like an Engineer, Not a Consultant

Writing exists to communicate, not to sound sophisticated.

This file owns wording and length in every artifact (code comments, docs,
tickets, commit messages, chat). Reply structure in chat is `reply-shape.md`.

## Rules

- Prefer plain English over technical-sounding language.
- Use technical terms only when they are the correct, precise term.
- State what changed before explaining why.
- Write the shortest text that conveys all necessary information.
- Delete filler, narration, and obvious observations.
- Never use abstractions where a concrete noun or verb works
- Never use metaphors when a literal description is possible.
- Prefer short sentences over long ones. Break up long sentences into multiple sentences.
- More sentences with simpler words is better than fewer sentences with complex words with dense meaning.
- Assume the reader is competent. Do not over-explain.

## Avoid

Do not inflate ordinary ideas into architectural prose.

Bad:

- "The seam between these components..."
- "The integration surface..."
- "The operating envelope..."
- "This enables a first-class experience..."
- "Leverage..."
- "Mental model..."
- "Canonical..."
- "Orthogonal..."
- "Byte-identical..." (unless literal bytes are being discussed)
- "Paradigm", "synergy", "holistic", "robust" when a simpler word is accurate.

Instead write what actually happens.

Bad:
> Introduce a typed boundary between the host and the application.

Good:
> The host calls the app through a typed interface.

Bad:
> This change reduces the cognitive load required to reason about the system.

Good:
> This makes the code easier to understand.

Bad:
> We leverage a shared abstraction.

Good:
> Both systems use the same interface.

Bad:
> Delete the legacy QA surface.

Good:
> Remove the old QA API.

Every sentence should earn its place. If removing it loses no information, remove it.

## The "Normal Engineer" Test

Before writing a sentence, ask:

"Would an experienced engineer naturally say this in a code review?"

If not, rewrite it.

Prefer:

- "use"
over
- "leverage"

Prefer:

- "remove"
over
- "deprecate" (unless you mean "keep but discourage")

Prefer:

- "works with"
over
- "integrates with"

Prefer:

- "limit"
over
- "operating envelope"

Prefer:

- "connection" or "interface"
over
- "seam"

Write like you're explaining the code to a teammate sitting next to you, not presenting at a software architecture conference.
