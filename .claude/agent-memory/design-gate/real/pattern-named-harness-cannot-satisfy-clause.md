---
name: pattern-named-harness-cannot-satisfy-clause
description: A clause naming a specific existing test harness whose configuration makes the clause's own assertion impossible — judge by the properties the clause lists, not the harness name.
metadata:
  type: feedback
---

A clause that says "a test on <the existing> harness … asserts a PNG exists" is self-defeating
when that harness is the no-renderer app, which can never write a PNG.

**Why:** `GdtfUiTestAppBuilder` builds `DefaultPlugins` with
`WgpuSettings { backends: None }`
(`crates/gdtf_test_utils/src/default_plugins_harness/ui/app_builder.rs:43-46`), so no render
device exists and no screenshot can land. The implementer built a second real-editor app on
`DefaultPlugins` with a live wgpu device instead. That is not narrowing — the clause's
parenthetical spelled out the properties that mattered (real `MapEditorPlugin` lifecycle, live
`AssetServer`, no `insert_resource` of model state, no `MinimalPlugins`) and the substitute
satisfies all four. `MapEditorPlugin` is still the real lifecycle plugin these harnesses add
(`crates/gdtf_content_editor/tests/prefab_mode/harness.rs:9`).

**How to apply:** when a clause names a harness AND lists its properties, judge against the
PROPERTIES, and check whether the named harness could satisfy the assertion at all before calling
a substitution a deviation. Separately, note which production arm the substitute leaves untested —
that is a note for the follow-up ticket, not a violation, when the ticket defers the live proof.
Here the untested arm has since narrowed: `EditorShotSource::PrimaryWindow` still exists
(`crates/gdtf_content_editor/src/net_qa/screenshot/config.rs:46`, matched at `spawn.rs:18`), but
the shipped editor now defaults to `Offscreen` (`config.rs:53`) and a test pins that the real app
carries it (`crates/gdtf_content_editor/tests/net_qa_editor_screenshot/source.rs:12-14`).

Related: [[pattern-mcp-host-built-from-the-session-checkout]],
[[pattern-contract-mandated-coverage-loss]].
