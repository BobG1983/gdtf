---
name: pattern-hand-triggered-frame-fixture-is-ok-with-a-real-path-complement
description: A test that hand-inserts a framework's own state to PIN A FRAME (not to fake the fact under test) passes the tests lens only when the realness of that state is proven by a separate real-plugin test.
metadata:
  type: feedback
---

Judging hand-inserted state needs one question: is the inserted value the FACT UNDER TEST, or
the TRIGGER TIMING?

**Why:** the banned pattern is a `MinimalPlugins` app asserting a value the real app would
produce differently. Pinning a frame boundary is not that, provided the real path is covered
elsewhere — and demanding both in one test would make the ordering frame unobservable.

The worked case: `crates/gdtf_content_editor/src/net_qa/screenshot/test/ordering.rs:83-96`,
`record_egui_window_mapping`, hand-writes a `bevy_egui` `WindowToEguiContextMap` entry, and
`a_shot_settling_on_the_retarget_frame_is_captured_not_refused` calls it at `:134` purely so
the camera retarget fires on a chosen frame. The fact under test is
`EditorNetQaSystems::Present.before(EditorNetQaSystems::Gather)`
(`crates/gdtf_content_editor/src/net_qa/present/plugin.rs:23`), and the app carries the real
present plugin plus the real pump. That passes, because the realness of the map entry is
proven separately: `crates/gdtf_content_editor/tests/net_qa_editor_present/harness.rs:54`
builds a real editor app with a real `EguiPlugin` and inserts nothing, and
`crates/gdtf_content_editor/tests/net_qa_editor_screenshot/harness.rs:39`, `gpu_editor_app`,
does a GPU pixel readback with the real `EguiPlugin` at `:67`.

**How to apply:** for each hand-inserted resource ask "which assertion would change if the
real producer disagreed?" If none — trigger only — find the sibling real-plugin test and
cite it by path. If the assertion itself depends on the inserted value, it is a violation.

Related: [[pattern-canned-fixture-carries-the-unasserted-value]].
