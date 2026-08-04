---
name: pattern-doc-heading-rename-anchors
description: Nothing in the suite reads a markdown link's #fragment, so renaming a docs/ heading silently breaks every inbound anchor link — grep the old slug yourself.
metadata:
  type: feedback
---

When a diff renames a heading in `docs/`, grep the repo for the OLD slug before passing the clause:
`grep -rn "the-old-heading-slug" docs/ .claude/ crates/ bins/ CLAUDE.md`. Do the same for every NEW
`#anchor` the diff adds — open the target file and find the heading it points at.

**Why:** anchor links are in use and no test looks at them. `docs/tooling/agent-qa.md:202` links
`[the protocol sketch](#the-protocol-sketch)`, which lands on `## The protocol sketch` at
`docs/tooling/agent-qa.md:321`. `docs/mvp/mvp.md:13` links `../combat/combat.md#arena-size`, which
lands on `## Arena size` at `docs/combat/combat.md:15`. Rename either heading and the link dies
with every check green. The only doc guard left is
`crates/gdtf_test_utils/tests/qa_commands_doc/main.rs`, and it is not a link scanner: it holds a
hand-written `CITED_PATHS` array (:27) for one guide (:7) and asks only
`repo_root().join(cited).exists()` (:60) — no fragment, no other file.

`docs/tooling/agent-qa.md` also carries the rename that makes this concrete:
`## The launch recipe` became `## The launch recipe — the game` (:30) so
`## The launch recipe — the content editor` (:96) could sit beside it. The old slug had no inbound
links, so nothing broke — the check is what matters, not that outcome.

**How to apply:** treat a heading rename like a public rename. Grep the old slug, confirm each new
anchor against a real heading in the file it names, and put the result in the finding — "no inbound
links, checked" is an answer, silence is not.

Related: [[pattern-docs-mirror-router-and-manifest]].
