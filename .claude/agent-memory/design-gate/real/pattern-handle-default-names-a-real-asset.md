---
name: pattern-handle-default-names-a-real-asset
description: A doc claiming a placeholder Handle::default() "names no image" is false — Bevy's ImagePlugin registers Image::default() at that handle.
metadata:
  type: feedback
---

`Handle::<Image>::default()` is not an unresolvable handle. `ImagePlugin::build` inserts
`Image::default()` (1x1 white, usage without `COPY_SRC`) at exactly that handle, and
`TRANSPARENT_IMAGE_HANDLE` beside it — `bevy_image-0.19.0/src/image.rs:222` in the cargo
registry checkout.

**Why:** the editor's capture source defaults to
`Offscreen(ImageRenderTarget::from(Handle::<Image>::default()))`
(`crates/gdtf_content_editor/src/net_qa/screenshot/config.rs:53`). A first round documented that
as "captures a handle naming no image and times out rather than silently reading the swapchain" —
false; it names a real 1x1 image. Two shapes are acceptable:

- Guard the placeholder where the capture is built, and test it. `spawn_capture` matches
  `Offscreen(target) if target.handle == Handle::<Image>::default()` and falls back to
  `Screenshot::primary_window()`
  (`crates/gdtf_content_editor/src/net_qa/screenshot/spawn.rs:19`), pinned by
  `the_default_placeholder_source_falls_back_to_the_primary_window`
  (`crates/gdtf_content_editor/src/net_qa/screenshot/test/source.rs:95`).
- Or take the fallback at the consumer, the way the game does — `Option<QaCaptureTarget>` with
  `None => Screenshot::primary_window()`
  (`crates/gdtf_app/src/dev/net_qa/screenshot/pump.rs:135-136`).

**How to apply:** when a diff adds a `Handle::default()` placeholder and documents what happens
when it is used, read `ImagePlugin::build` before believing the claim, and require a test on the
placeholder path.
