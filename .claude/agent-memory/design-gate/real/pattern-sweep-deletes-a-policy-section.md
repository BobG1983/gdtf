---
name: pattern-sweep-deletes-a-policy-section
description: A deleted-name sweep through docs/ can remove a whole policy block because ONE sentence in it named a deleted thing — read removed blocks, not removed names.
metadata:
  type: feedback
---

When a deletion or rename ticket sweeps `docs/` for references to what it removed, check
whether the sweep DELETED prose rather than corrected it, and whether the deleted prose was
policy that later tickets depend on.

**Why:** design policy in `docs/` is almost entirely unguarded. The one guard over the QA
command guide, `crates/gdtf_test_utils/tests/qa_commands_doc/main.rs`, pins cited file paths
(`CITED_PATHS`, `:27-33`), code shapes that must appear in both the guide and the worked
example (`SHOWN_IN_THE_EXAMPLE`, `:11-25`), and three command names the guide must NOT claim
(`:100-110`). None of that reads the policy the same file states: `docs/tooling/qa-commands.md`
carries `## What the QA channel is for` (`:19`) — "It is not a test harness" (`:25`), "a
command helps an agent play or author, or it does not exist" (`:33-35`), and "Replies stay
small and scoped" (`:37`). Delete that whole section and every step of the green suite still
passes.

**How to apply:** on any deletion or rename ticket, run
`git diff develop -- docs/ | grep '^-' | grep -v '^---'` and read every removed BLOCK. For
each one ask two questions: does a ticket clause name this file, and does the removed text
survive anywhere (grep three distinctive phrases from it across `docs/`). A block that fails
both is uncontracted policy loss and a design-fidelity violation even with a green suite.

Related: [[pattern-test-doc-overclaims-its-assertion]], [[pattern-untested-mcp-description-prose]].
