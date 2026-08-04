# Shape of a reply to the user

Why this rule exists: on 2026-07-31 the user corrected reply shape three times —
"wildly verbose", "wall of noise", and "Pretend you're talking to an executive.
Recommendation → 2–3 plainly stated reasons → evidence if not obvious." Each
reply was accurate and used plain words. The defect was structure: reasoning
before the answer, options nobody would pick, process narration, restating
tool output already on screen.

`plain-language.md` owns wording and length (code, docs, tickets, chat).
This file owns reply structure in chat only. Neither restates the other.

## Rules

1. **Answer first.** Lead with the answer, result, or recommendation. Reasons
   follow. They never come first.
2. **Decisions:** recommendation, then 2–3 plain reasons, then evidence only
   where it is not obvious. Not a survey of options. Pick one and say why.
3. **Do not narrate process.** What was tried and in what order is not the
   deliverable. Report the result and what backs it.
4. **Do not restate a tool result the user can see.** A diff, table, or command
   output stands alone.
5. **Decide routine calls yourself.** Ask only when different readings mean
   materially different work. An obvious default is noise.
6. **Raise a concern once.** One or two sentences, then do the work as asked.
   If the user reaffirms, stop re-litigating.
7. **Correct plainly and move on.** No apology stack, no tally of past errors.

## Compatible with verification

Quoted failure output (see `verification.md`) is evidence, not narration.
Paste failures when the suite is red. That does not license a process diary.

## How it is enforced

Agents and skills that talk to the user follow this shape. A reply that cannot
state its answer in the first line is not ready.
