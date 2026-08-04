---
name: invented-names-are-jargon
description: Don't coin metaphor names for components or edits — name the real thing (the MCP server, `gdtf_qa_mcp`; the fixed sections 8-10), because a coined name forces every reader to hold a private glossary.
metadata:
  type: feedback
---

Do not invent a name for a component, a file, or an edit. Name what it actually is.

**Why:** the user flagged this directly — "courier" is jargon, and so is "corrected tail", adding
"Claude seems particularly bad for this kind of jargon use." A coined metaphor ("courier" for
`bins/gdtf_qa_mcp`, "the brain" for a state file, "corrected tail" for a fixed section) makes every
reader hold a glossary only you have. The Avoid list in `.claude/rules/plain-language.md:24-33` is
examples, not the boundary — the failure mode is inventing a name instead of naming the real thing.

**How to apply:**

- Say "the MCP server (`gdtf_qa_mcp`)", "the protocol crate (`gdtf_qa_protocol`)", "the fixed
  sections 8-10 of the design".
- Before writing a cute name, ask: would a teammate know what this is without having read my earlier
  messages?
- Landed code and docs that already carry a coined name are NOT retro-edited for it —
  `bins/gdtf_qa_mcp/src/mcp/courier/` and `docs/decisions/0008-qa-command-courier.md` keep theirs.
  New text does not spread it.

Related: [[dont-retune-what-the-user-tuned]].
