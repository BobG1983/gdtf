---
name: pattern-mirrored-unasserted-plumbing-consts
description: Copying a render path does not copy its tests — audit both copies, because the ORIGINAL can end up the unpinned one (game present/blit vs the editor's).
metadata:
  type: feedback
---

When one render path is mirrored from another, coverage does not travel with the code.
Audit both copies against the same list of mutations, not just the new one.

**Why:** the editor's copy is fully pinned. `crates/gdtf_content_editor/src/net_qa/present/
test/blit.rs` asserts the present camera's order (`:52-56`), that its layer is disjoint from
the editor and preview cameras (`:57-61`), that exactly one blit sprite exists (`:76-77`) on
that layer (`:86-90`), and that the sprite is sized from the live window rather than a
fixture constant (`:91-99`).

The game's original pins none of it. `PRESENT_LAYER` and `PRESENT_ORDER`
(`crates/gdtf_app/src/dev/net_qa/present/blit.rs:9,11`) are private constants no test names,
and `QaPresentSprite` (`:17`) appears nowhere else in the repo. The game's present suite
(`crates/gdtf_app/src/dev/net_qa/present/test/present.rs`) has five tests — winit settings,
capture target format, camera retarget, "a present camera targets the window" (`:79-93`),
and default UI camera — so deleting the sprite spawn (`blit.rs:45-53`), flipping
`PRESENT_ORDER` negative, or setting `PRESENT_LAYER` to `0` each leave the suite green while
the window shows nothing or shows the blit underneath the game.

**How to apply:** name the three mutations explicitly for each copy — sprite deleted, order
flipped, layer collided — and check whether any assertion closes over them. If the acceptance
clause lists a closed set that omits them, it is a coverage gap to report, not a clause
violation. And check the size/scale pin: two test bands sharing one fixture constant let a
hardcoded value pass; only reading the window live proves "derived from the window".
Related: [[pattern-tautological-assertion-replacing-a-pin]].
