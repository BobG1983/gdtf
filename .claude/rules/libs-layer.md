# What may live in `libs/`

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

`crates/` and `bins/` hold gdtf. `libs/` holds the crates another game could take as-is. Every
crate there is named `cobalt_*`, and the folder and the prefix mean the same thing.

## The rule

A crate under `libs/` carries nothing game-specific.

1. It names no `gdtf_*` crate under `[dependencies]` or `[build-dependencies]`.
2. It defines no domain type. `Hp`, `Ganger`, `TerrainUuid` and everything else out of
   `docs/glossary.md` belong in `crates/`.
3. It makes no genre assumption in a signature. A function that takes a grid cell, a turn, or a
   line of fire is a gdtf function.
4. It uses no gdtf vocabulary in a doc comment, a test fixture, or an error message. A shared
   crate that says "ganger" in a panic message has leaked.
5. It carries no default that only makes sense for this game. A settle of 15 frames tuned
   against gdtf's battle scene is gdtf tuning; a caller passes its own number instead.

## The one exception

`[dev-dependencies]` may name `gdtf_test_utils`, and nothing else. The shared harness links only
when the crate's own tests build, so it never reaches a consumer of the published crate.

## Enforcement

`crates/gdtf_test_utils/tests/libs_layer/` backs the coarsest part: no `libs/*` manifest names a
`gdtf_*` crate a consumer would link, the `gdtf_test_utils` dev-dependency is allowed by name,
and the folder and the `cobalt_` prefix agree in both directions. Rules 2 to 5 are read by a
person, not a test. The rule carries the weight; the test is a backstop.

Nothing here says how a promotion is decided. That is a conversation.
