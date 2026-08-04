---
name: pattern-absence-assertion-vacuous-on-minimal-app
description: An "inert build wires nothing" test on MinimalPlugins passes for the wrong reason — the missing window, not the plugin's inertness — so only the assertion on a resource inserted in Plugin::build discriminates.
metadata:
  type: feedback
---

A clause of the form "the feature-on, env-off build spawns no target, no camera and no
settings override" is usually answered with a `MinimalPlugins` app plus the real `from_env`
constructor, asserting three absences. Only the absence of a resource inserted directly in
`Plugin::build` discriminates. A window-sized render target and a camera that needs it would
be absent on a `MinimalPlugins` app even if the plugin were added unconditionally, because
there is no `PrimaryWindow` for the creating system to read.

Live example: `crates/gdtf_content_editor/src/net_qa/present/test/inert.rs`. Only the
`WinitSettings` check at `:22-25` is load-bearing; `EditorQaCaptureTarget` (`:26-31`) and
`EditorQaPresentCamera` (`:32-39`) would be absent regardless.

Worse in this case: the test returns early at `:12-14` when `editor_net_qa_enabled()` is
true, and that is `const fn ... { true }`
(`crates/gdtf_content_editor/src/net_qa/env.rs:7-9`) — so the body never runs at all.

**Why:** the `WinitSettings` assertion is the only one that would catch the mutation the
clause is about (moving the plugin out of the listener arm into `build`). Name which single
assertion carries the test rather than crediting three.

**How to apply:** for any absence-asserting test, ask what ELSE in the harness would make the
value absent. If the harness alone explains the absence, the assertion proves nothing. Read
the early-return guards in the same pass — a test that returns before its assertions proves
nothing at all.

Related: [[pattern-mirrored-unasserted-plumbing-consts]].
