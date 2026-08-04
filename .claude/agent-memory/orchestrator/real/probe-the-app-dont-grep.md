---
name: probe-the-app-dont-grep
description: A missing grep hit is not proof a plugin or capability is absent — assert it in a running App; pin the exact symptom before root-causing; let the app settle before reading state; gate lenses run zero cargo.
metadata:
  type: feedback
---

Never conclude "X is not wired" from a grep miss. Assert it against a real `App`:
`app.is_plugin_added::<T>()`.

**Why:** Bevy's `ui` feature pulls in picking, so `UiPickingPlugin` is live in every build of this
repo without appearing anywhere in a manifest. `crates/gdtf_battle_input/tests/picking/ui_picking_coexistence.rs:146`
asserts exactly that, and `docs/ui-picking-arbitration.md:20,180` records why adding it yourself
panics on the double add.

The same trap runs the other way with cargo features: they are package-qualified. The `dcheck` /
`dclippy` / `dtest` aliases in `.cargo/config.toml` name `grimdark_turfwar/dynamic_linking` and
`grimdark_turfwar/dev_tools` — a same-named feature on another package stays off, and its modules
never compile, so its tests report "0 tests" instead of failing. Read the alias, not the feature
name. (QA modules no longer sit behind a feature at all — they compile under `debug_assertions`,
see `crates/gdtf_app/src/dev/plugin.rs:25-28`.)

**How to apply:**

1. **Pin the exact observed behaviour before root-causing.** Never hand a sub-agent an assumed
   premise; a bug filed on a stale doc quote burns the whole investigation. When an agent measures
   and contradicts you, check the measurement instead of defending the premise.
2. **Probe the running app, don't reason about it.** A UI node that is visible and correctly laid
   out can still draw nothing. Read the real components.
3. **Settle before reading.** Drive several frames before asserting state in a headless test —
   `gdtf_test_utils::advance_until` (`crates/gdtf_test_utils/src/advance.rs:11`) exists for this.
4. **Never assert exact equality on an interpolated value.** `Vec3::lerp` does not land on its
   endpoint (`crates/gdtf_battle_presenter/src/actors/fx/projectile/travel.rs:95`).
5. **A `MinimalPlugins` stand-in proves nothing about the real app.** It is fine for render-free sim
   tests (`crates/gdtf_battle_sim/tests/*/harness.rs`); it is not evidence for a claim about the
   game or the editor. The gate's tests lens rejects it outright
   (`.claude/workflows/build-ticket.js:234`).
6. **Distrust stale signals.** rust-analyzer diagnostics during a build loop can be phantoms from a
   mid-edit file — verify with cargo. `[unstable] checksum-freshness = true` in `.cargo/config.toml`
   means `touch` does NOT invalidate a cached cargo result; to prove a lint really covers code,
   inject a real violation, watch it fail, then revert.

**One build at a time.** Reliability beats wall-clock: `/heartbeat` starts exactly one build per
tick and never a second while one is alive (`.claude/skills/heartbeat/SKILL.md:28,37`). Concurrent
cargo runs sharing one `target/` can leave the bevy dylib stale against the rlibs, which shows up as
a link failure at land time after a green verify. Read-only reviewers are safe to fan out because
the gate lenses run ZERO cargo (`.claude/workflows/build-ticket.js:232-236`).

Related: [[a-flaky-test-is-a-broken-test]], [[egui-editor-bevyui-game-never-cross]].
