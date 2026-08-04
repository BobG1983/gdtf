---
name: pattern-partial-falsehood-sweep
description: When a clause names one false doc line to fix, grep the CLAIM across crates, bins and docs — and grep the words the correction deleted, not the words the false sentence used.
metadata:
  type: feedback
---

A doc-correction clause cites one `file:line`. That line is an example of the claim, not its
extent. Grep the claim, and grep it with the vocabulary the CORRECTION removed — the
survivors rarely use the phrasing of the sentence that was cited.

**Why:** the sweep stops at whatever boundary the implementer was already inside — the file,
then the sibling module, then the crate — and the last survivor is usually a doc that states
the same claim as the reason for something else. Docs under `docs/` are the most common
resting place, because nothing compiles them. A live one:
`docs/tooling/qa-commands.md:116-121` still describes the editor's
QA query pair as built — `GetEditorQueryOptions` / `QueryEditor(EditorQueryKind)` with a
`topic_available(kind, model)` predicate in
`crates/gdtf_content_editor/src/net_qa/snapshot/topics.rs`. None of that exists: there is no
`snapshot/` under `crates/gdtf_content_editor/src/net_qa/` (it holds `config.rs`, `env.rs`,
`mod.rs`, `plugin.rs`, `present`, `router`, `schedule.rs`, `screenshot`), and neither symbol
appears anywhere in `crates/` or `bins/`.

**How to apply:**

1. Recover the old wording with `git show develop:<file>` and grep every distinctive phrase
   the correction DELETED across `crates bins docs` — not the phrase the cited line used.
2. Read every hit rather than counting them; a surviving sentence is often true in its own
   narrower scope.
3. Report all survivors in one pass. Handing them over one boundary at a time is what turns
   a two-sentence fix into five review rounds.
4. Grep `docs/` for the short half of a phrase too — markdown wraps, so a phrase that spans
   a line break is invisible to the obvious grep.
5. If the clause forbids the sweep ("no other wording changes"), a survivor is a note plus a
   follow-up ticket, not a block. Prove the edit stayed narrow with a word-stream diff
   (`git show HEAD:<f> | tr -s ' \n' '\n\n'` against the working copy), which reduces
   rewrapped markdown to just the words added and removed.

Related: [[pattern-docs-mirror-router-and-manifest]].
