---
name: pattern-scene-plugin-means-presenter-resources-exist
description: "\"The headless test app has no presenter resources\" is false — register_headless adds ScenesPlugin, whose battlescape plugin adds BattlePresenterPlugin at build time."
metadata:
  type: feedback
---

"The headless suite's app has no presenter resources" is a false premise for any app built
through the `gdtf_app` test harnesses. Never accept a clause that rests on it.

**Why — the chain, all of it at app BUILD:** `gdtf_app::test_support::register_headless`
(`crates/gdtf_app/src/test_support.rs:98`) adds `ScenesPlugin` at `:102`; the battlescape
scene plugin's `build` calls `add_plugins`
(`crates/gdtf_app/src/states/running/game/battlescape/plugin.rs:23`), which adds
`gdtf_battle_presenter::BattlePresenterPlugin::default()` at `:52`; that builds
`TopDownRendererPlugin`, whose `build` inserts `TopDownRendererActive` and inits
`ActiveLevel`, `ViewMode`, `IsolateView` and more
(`crates/gdtf_battle_presenter/src/plugin/topdown/plugin.rs:29-32`). None of it waits for a
battle — the resources exist at the menu.
`crates/gdtf_app/tests/battle_shell/presenter_foundation.rs:97-103` pins it.

**How to apply:** a "does not depend on the presenter" clause must be phrased as a property
of the code — the command's availability predicate and its handler's system parameters name
nothing from `gdtf_battle_presenter` — not as an absence of presenter resources in the test
app. Asserting the absence would pass only on a bare `MinimalPlugins` app, where the
assertion is vacuous and proves nothing about the real one.
