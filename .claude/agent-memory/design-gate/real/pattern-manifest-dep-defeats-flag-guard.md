---
name: pattern-manifest-dep-defeats-flag-guard
description: A guard that greps command text for a cargo feature flag cannot see the same feature turned on by a Cargo.toml — one library unit per invocation means a feature enabled anywhere in a --workspace build is on for every crate in it.
metadata:
  type: feedback
---

A guard that pins "no `--workspace` command names feature F" reads command TEXT. A
workspace member whose manifest enables F unconditionally reaches the same end state and
the guard stays green. Cargo builds one library unit per invocation, so F enabled anywhere
in a `--workspace` run is on for every consumer in that run.

**Why:** `crates/gdtf_test_utils/tests/ci_workflow_features/check.rs:10` bans
`dynamic_linking` on any CI `--workspace` command (`:48-52`), and it only ever reads the
workflow YAML (`:20-30`). Today the text and the link graph agree, because every crate
keeps the feature opt-in — `dynamic_linking = ["bevy/dynamic_linking"]` in
`crates/gdtf_app/Cargo.toml:31`, `crates/gdtf_content_editor/Cargo.toml:25`,
`crates/gdtf_net_qa_transport/Cargo.toml:11`, `crates/gdtf_qa_command/Cargo.toml:15`,
`crates/gdtf_screenshot/Cargo.toml:13`. One crate writing
`bevy = { features = ["dynamic_linking"] }` outright would flip every CI build while the
guard passed. The local aliases show the flag route the guard does watch:
`.cargo/config.toml:17-19` name `grimdark_turfwar/dynamic_linking` on `--workspace`
commands on purpose, for the gate only.

**How to apply:** when a diff adds a workspace member, or a `features = [...]` entry on an
intra-workspace dependency, find the guard or rule that names that feature and read WHAT it
inspects. If it inspects only command text or manifest text, say so and name the route it
cannot see — a text guard is never evidence for a property of the built graph. `cargo tree
-p <bin>` is the wrong scope here: `-p` never builds the other members, so it reports clean
while the `--workspace` build links it. Related:
[[pattern-guard-placed-in-sibling-not-named-file]].
