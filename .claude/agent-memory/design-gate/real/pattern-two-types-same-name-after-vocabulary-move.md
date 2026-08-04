---
name: pattern-two-types-same-name-after-vocabulary-move
description: The cell types exist in both gdtf_qa_protocol and gdtf_app's wire module, so a ticket naming CellLevelNet without a path is ambiguous.
metadata:
  type: feedback
---

`CellXNet`, `CellYNet`, `LevelNet`, `CellNet` and `CellLevelNet` exist TWICE:
`crates/gdtf_qa_protocol/src/ids/cell.rs:11-70` and
`crates/gdtf_app/src/dev/net_qa/wire/cell.rs:10-67`. The `wire/` copy landed; the protocol one
was never removed.

**Why:** the QA rewrite moves the game's wire vocabulary out of the shared protocol crate and
into the host that owns it. Only the copy half has landed. `cell.rs` and `shot.rs` are all that
`crates/gdtf_qa_protocol/src/ids/mod.rs` still declares.

**How to apply:** a ticket naming a cell type must give the FULL path. The two are distinct
types, and importing the protocol one puts the game's arguments back on the crate the rewrite is
emptying. No other wire name is duplicated — `GangerToken` (`wire/token.rs:10`), `ActSeqNet`
(`wire/act.rs:32`) and the pointer types (`wire/pointer.rs:10-53`) exist only under `wire/`.

Related: [[pattern-unsatisfiable-half-built-and-disclosed-late]].
