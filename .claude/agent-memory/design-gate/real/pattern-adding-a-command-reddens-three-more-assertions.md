---
name: pattern-adding-a-command-reddens-three-more-assertions
description: Adding a QA command turns three assertions red and two counted sentences in the qa-commands guide silently false — no guard reads prose.
metadata:
  type: feedback
---

Adding a command to a QA host turns assertions red, and those get found the moment the suite
runs. It also turns counted sentences in PROSE silently false, and nothing reads those.

The prose that goes false:

- `docs/tooling/qa-commands.md:3` (frontmatter `description:`) — "written from the one command
  that exists, app.phase".
- `docs/tooling/qa-commands.md:14` — "Everything below is written from the ONE command that
  exists today".

The assertions that go red, for the edit list:

- `crates/gdtf_app/tests/net_qa/commands.rs:25-29` — `catalogue.entries.len() == 1`.
- `crates/gdtf_app/tests/net_qa/commands.rs:124` — the unknown-name reply lists every known
  name, pinned as the single `app.phase`.
- `crates/gdtf_app/tests/net_qa/command_set.rs:26-32` — `the_game_offers_exactly_app_phase`.

**Why:** `crates/gdtf_test_utils/tests/qa_commands_doc/main.rs` guards the guide's cited paths
(`CITED_PATHS`, `:27-33`), the identifiers it shows (`SHOWN_IN_THE_EXAMPLE`, `:11-25`), and a
three-name list of commands nobody built (`:103`). It reads no count sentence. Nothing reads a
test suite's `//!` doc at all — `crates/gdtf_app/tests/net_qa/main.rs:1` still calls the suite
"debug + `net_qa` feature only" long after `net_qa` stopped being a cargo feature. A sentence
that wrong survives the whole green suite.

The same trap runs the other way on a change that lands new shapes beside the old ones: a
counted claim copied from a design draft describes the END state, not what landed, and reads
plausibly because the paragraph around it is true.

**How to apply:** on any ticket that adds a command, require both prose sites in the edit list
alongside the three assertions, plus `crates/gdtf_app/src/dev/net_qa/commands/mod.rs` — the new
family's `mod` line, where `pub(crate) mod read;` at `:4` is the template. Grep any new doc
prose for counted claims ("the one command", "exactly N", "three variants") and check each
against the tree rather than against the draft it was copied from.

Related: [[pattern-additive-field-skips-version-bump]].
