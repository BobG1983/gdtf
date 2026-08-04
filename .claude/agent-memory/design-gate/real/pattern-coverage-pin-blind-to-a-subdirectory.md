---
name: pattern-coverage-pin-blind-to-a-subdirectory
description: A "every type in this module has a test case" guard reads only *.rs files directly inside the module dir, so a directory submodule is invisible to it and lands with zero cases, green.
metadata:
  type: feedback
---

A source-scanning completeness guard walks `fs::read_dir(module_dir)` and skips anything whose
`path.extension()` is not `rs`. A SUBDIRECTORY has no extension, so it is skipped silently —
and `.claude/rules/module-layout.md` rule 1 says a concern that outgrows one file BECOMES a
directory module. The blind spot sits on the codebase's own growth path, not a hypothetical.

Live example: `crates/gdtf_app/src/dev/net_qa/wire/test/coverage.rs:25-27` is the skip. Both
corpora read one level only — `declared_wire_types` scans `wire/` (`:57-67`) and `test_corpora`
scans `wire/test/` (`:111-122`) — and the anti-vacuity test
`the_wire_scan_finds_the_vocabulary` (`:124-139`) iterates the same `rust_sources`. Adding
`wire/log/mod.rs` plus `wire/log/deed.rs` with zero test cases leaves every type in it
invisible to the round-trip pin, the schema pin AND the anti-vacuity test. The suite stays
green.

**Why:** the guard's whole job is "no type ships without a case". A rule the repo already
follows — split a growing module into a directory — silently switches it off.

**How to apply:** when a guard enumerates a module by reading its directory, ask what happens
to a subdirectory and name the mutation explicitly. Rule it a NOTE while no subdirectory
exists other than a deliberately-excluded `test/`; rule it a violation the moment the module
splits, or if the ticket text names a directory the guard cannot see.

Related: [[gate-manifest-guard-vacuity-check]].
