---
paths: ["**/*"]
---

# Shape of a reply to the user

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

Why this rule exists: on 2026-07-31 the user corrected reply shape three times: "wildly verbose",
"wall of noise", and "Pretend you're talking to an executive. Recommendation → 2–3 plainly stated
reasons → evidence if not obvious." Each reply was accurate and used plain words. The structure
was the defect.

This file owns the structure of a chat reply, and nothing else. Agents and skills that talk to
the user follow this shape too.

## Rules

1. Lead with the answer, the result, or the recommendation. Reasons follow.

2. For a decision, give two or three plain reasons, then evidence only where it is not
   obvious. Do not survey the options.

3. Do not narrate the process. What was tried, and in what order, is not the deliverable.
   Report the result and what backs it.

4. Do not restate a tool result the user can see: a diff, a table, or command output.

5. Decide routine calls yourself. Ask only when two readings of the request mean materially
   different work.

6. Raise a concern once, in one or two sentences, then do the work as asked. If the user
   reaffirms, stop arguing.

7. Correct an error plainly and move on. Do not stack apologies or list earlier mistakes.

8. Long paragraphs, long sentences and repeated words are noise. If you cannot say it in two
   or three sentences, you are not ready to answer.

9. Again, **BE CONCISE**. The user will not read paragraphs of prose, especially if the prose
   does not follow `plain-language.md` rules.

## Quoted failure output

Paste the failures when the suite is red; `verification.md` requires it. Rule 3 still holds for
the words around them.
