---
name: quote-the-ticket-spec-the-files
description: Build every ticket contract from the live ticket text — full description AND comment thread, quoted verbatim — because the gate lenses audit the contract you hand them, not the ticket
metadata:
  type: feedback
---

Write every build contract from the live ticket text: the full description **and** the whole comment
thread, quoted verbatim. Never from memory, never from a summary argument, never from your own
earlier note.

**Why:** the three design-gate lenses audit the contract you hand them, not the ticket
(`.claude/skills/gate/SKILL.md` step 6 — three parallel read-only lenses, merged
any-non-compliant-blocks). A contract written from memory can pass all three while violating the
ticket outright. It happened: a contract said to guard a plugin add with `is_plugin_added` when the
ticket's acceptance clause said no such guard may remain. All three lenses passed the work against
the wrong specification and it landed wrong.

Answers, corrections, and rulings live in comments and routinely contradict a stale description.
`.claude/workflows/build-ticket.js` already works this way — it fetches "FULL description AND
complete comment thread, verbatim" through the `project-manager` agent before marking the ticket In
Progress, re-fetches it before the verify phase, and states outright that it never audits a summary
argument.

**How to apply:**

- Quote acceptance clauses verbatim into the contract. Paraphrasing loses the clause.
- Treat your own notes as untrusted. A dependency, an ordering, a "this blocks that" — verify it
  against the code before obeying it. A status file once asserted a strict three-ticket blocking
  order; it was assumption, never checked, obeyed for days, and false. It cost an epic its payoff.
- Line-number citations in tickets and design docs rot within days. Re-verify before building from
  one — quoted text survives renames.
- An implementer's report can misdescribe what it shipped in both directions: once the tree was
  right and the report wrong, once a report claimed a version pin the code had already moved past.
  Read the tree.
- Check every acceptance clause is satisfiable **when the ticket will be built**. A clause demanding
  evidence that needs a capability a later ticket builds cannot be met, so either a rigorous lens
  blocks forever or the build agent improvises a banned workaround. One clause required a live reply
  from a running editor while the tool that could produce one was two tickets away; the only way to
  satisfy it as written was a hand-written socket client, which the user had banned outright.
  `build-ticket.js` runs this as check 1 of its clause audit: "Every clause names evidence this run
  can actually produce."
- On a structural refactor, write a **file-by-file target tree** plus a verify step that fails the
  lazy move. Agents take the shallow directory rename that keeps concern-lumped files, and a loose
  gate passes it. The fix is not a better model — write the target yourself.
- Never split out the thing that populates or triggers a feature. Ask what the user can DO after the
  ticket lands. "Nothing yet" means the split is wrong.
- Ask it of the whole epic too: when all of this lands, what can the user actually do? Then check
  that answer against the code. One epic sequenced three tickets of protocol groundwork ahead of the
  one that exposed any of it, so three closed tickets left the editor undrivable. A later audit found
  the planned set would deliver a readable but not authorable editor: nine form modes plus a spatial
  painter (`crates/gdtf_content_editor/src/placement/`, `canvas/edit_level.rs`,
  `connector_pairing/`) that "set a named field" cannot express at all. Every ticket was well-formed;
  the set of them was not sufficient. Close a capability epic with a proof-and-re-audit ticket that
  drives the real thing end to end and re-derives what the target can do from its code — it does not
  pass merely because its siblings closed.

Related: [[comment-before-you-close-linear]], [[dont-build-tooling-to-prove-the-obvious]],
[[build-it-in-a-workflow]].
