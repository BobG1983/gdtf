---
name: gate-manifest-guard-vacuity-check
description: How to audit a tree-walking conformance guard for vacuity — it can pass while checking nothing — instead of trusting a green run.
metadata:
  type: feedback
---

A conformance guard that walks the tree can pass by checking *nothing*. Audit that
explicitly; a green run is not evidence the walk reached anything.

**Why:** every one of these suites derives what it checks by reading the tree —
`crates/gdtf_test_utils/tests/` holds `assets_tree_clean`, `ci_workflow_features`,
`module_layout`, `no_flat_integration_tests`, `qa_commands_doc` and `rustdoc_lint_gate`. When
the reader stops recognising the lines it looks for, the loop body never runs and the test
passes. Nothing about that failure shows up in the output.

**How to apply:** for any guard of this shape, check in order —

1. Does it assert the input list is non-empty? `ci_workflow_features/check.rs:38-42` fails
   loudly when no workflow file is found; `module_layout/conformance.rs:25-31` records a
   violation when no tracked `.rs` file is found. Without that, a broken enumeration reads as
   a pass.
2. Does it carry a list of inputs the walk MUST have reached?
   `ci_workflow_features/check.rs` builds a `reached` set (`:37`, `:59-63`), compares it
   against `REQUIRED_COMMANDS` (`:12-18`), and reports `UNREACHED` for anything missing
   (`:65-73`). `qa_commands_doc/main.rs` does the same job from the other side: for each entry
   in `CITED_PATHS` (`:27-33`) it asserts both that the guide names the path and that the path
   exists (`:59-68`), so either half going wrong fails. Without a list like that the walk is
   unfalsifiable.
3. Trace the walk by hand against the real tree: which inputs reach a check, which are
   skipped, and is at least one of them matched non-trivially?
4. Read the guard's own doc comment as a CLAIM and hold its code to it. Live example:
   `ci_workflow_features/check.rs:1` says workflows carry "no `dynamic_linking`, no
   `dev_tools`", but `STATIC_ONLY_BAN` (`:10`) is only `"dynamic_linking"` and `dev_tools`
   appears nowhere else in the suite — the doc bans two things, the code bans one.
5. Guard tests live in the test band, so `String` paths and bare `&str` consts are fine.
   `.claude/rules/no-bare-types.md:115` puts test bands out of scope.

Related: [[gate-untested-surfaces-inventory]].
