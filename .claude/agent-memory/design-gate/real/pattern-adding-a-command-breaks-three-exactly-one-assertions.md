---
name: pattern-adding-a-command-breaks-three-exactly-one-assertions
description: Adding one entry to GAME_COMMANDS turns three landed assertions red, and a test cannot even name the new command type without a line in the test_support export list.
metadata:
  type: feedback
---

Every ticket that adds a GAME command must name these as files it edits, or the suite goes
red:

- `crates/gdtf_app/tests/net_qa/commands.rs:25-29` — `catalogue.entries.len() == 1`.
- `crates/gdtf_app/tests/net_qa/commands.rs:124` — the unknown-name reply lists every known
  name, pinned as `vec![CommandName::from_static(APP_PHASE)]`.
- `crates/gdtf_app/tests/net_qa/command_set.rs:26-32` — `the_game_offers_exactly_app_phase`.

The list itself is one line: `GAME_COMMANDS` at
`crates/gdtf_app/src/dev/net_qa/commands/set.rs:7`.

Second half: a command's unit struct is `pub(crate)` — `AppPhase` at
`crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs:25`. A clause asserting
`DeferredReplies::<TheCommand>::len()` or `::budget()` from `crates/gdtf_app/tests/net_qa/`
cannot compile until that type, or a probe helper, joins the explicit `pub use` list at
`crates/gdtf_app/src/test_support.rs:36-40`. That edit sits outside `commands/`, so it also
breaks any "nothing outside the command directory changes" clause.

**Why:** `docs/tooling/qa-commands.md:9` says adding a command is "a file, one line in a
host's list, and a test" — it counts only the additive edits. Its "What you do NOT edit"
section (`:167-176`) names `command_set.rs` only as a thing PINNED, never as a thing edited,
so a ticket drafted from that doc misses all three assertions.

**How to apply:** on any ticket adding a command, check the three assertions and the export
line are in the edit list before passing the contract audit. Flag as self-contradicting any
ticket that both forbids edits outside the command directory and asks for `DeferredReplies`
assertions from an integration test — only one of those can hold.

Related: [[pattern-adding-a-command-reddens-three-more-assertions]].
