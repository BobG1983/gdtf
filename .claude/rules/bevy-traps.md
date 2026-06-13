---
paths:
  - "**/*"
---

# Bevy traps — engine/Rust/wgpu/cargo gotchas that have burned real sessions

Why this rule exists: every item below cost a real debugging session. Check
this list BEFORE chasing "impossible" behavior or filing a bug. This list
starts nearly empty by design — ADD an entry each time a real trap costs a
session, so the next session pays the toll once.

## Bevy 0.18

1. There is NO built-in state-scoped RESOURCE. Entity state-scoping is
   `DespawnOnExit<S>` / `DespawnOnEnter<S>` (renamed in 0.18 — no "State"
   suffix). RESOURCES must be inserted in an `OnEnter(state)` system and
   removed in `OnExit(state)`; any system that touches such a resource must
   guard with `.run_if(resource_exists::<R>)` or take `Option<Res<R>>`, or it
   panics when the resource is absent.
2. Bevy renames/moves APIs across 0.x releases. Pin behavior to the
   `Cargo.lock` Bevy version (currently 0.18.1) and check the migration guide
   for the exact range — do not trust pre-0.18 snippets from memory. When in
   doubt, ask the `bevy-expert` agent.
3. System ordering is AMBIGUOUS without explicit `.before()` / `.after()` /
   `.chain()` or named system sets. Two systems that read+write the same data
   with no ordering will run in a nondeterministic order and produce flaky,
   one-frame-off bugs — order them explicitly.
4. Buffered events are now **messages** in 0.18: the buffered `EventWriter` /
   `EventReader` / `Event` became `MessageWriter` / `MessageReader` / `Message`.
   `AppExit` is a `Message` — to quit, take `MessageWriter<AppExit>` and
   `.write(AppExit::Success)`; `EventWriter<AppExit>` does NOT resolve (E0425).
   The targeted/observer `Event` API is a separate concept now — confirm the
   split via the 0.18 migration guide / the `bevy-expert` agent. [burned GTW-2]

## wgpu / cargo

(none yet — add the first real one here)

## Rust / Bevy style

- Stay clippy-clean under the deny-by-default workspace lints
  (all/pedantic/correctness). No `unwrap`/`expect`/`panic!`/`todo!`/
  `unimplemented!` in shipped code — propagate `Result`/`Option` or handle
  it. Document every `pub` item (`missing_docs` is denied).

Invite growth: when a Bevy/Rust/wgpu/cargo trap burns a session, append it
here with a one-line "what it looked like vs. what was actually wrong".
