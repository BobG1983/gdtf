# What may live in `libs/`

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

`crates/` and `bins/` hold gdtf. `libs/` holds the crates another game could take as-is. Every
crate there is named `cobalt_*`, and the folder and the prefix mean the same thing.

## The rule

A crate under `libs/` carries nothing game-specific.

1. It names no `gdtf_*` crate under `[dependencies]`, `[build-dependencies]` or
   `[dev-dependencies]`. The shared harness a `libs/` crate's own tests need is
   `libs/cobalt_test_utils`.
2. It defines no domain type. `Hp`, `Ganger`, `TerrainUuid` and everything else out of
   `docs/glossary.md` belong in `crates/`.
3. It makes no genre assumption in a signature. A function that takes a grid cell, a turn, or a
   line of fire is a gdtf function.
4. It uses no gdtf vocabulary in a doc comment, a test fixture, or an error message. A shared
   crate that says "ganger" in a panic message has leaked.
5. It carries no default that only makes sense for this game. A settle of 15 frames tuned
   against gdtf's battle scene is gdtf tuning; a caller passes its own number instead.

## Enforcement

The `libs_layer` guard suite backs two parts.
`no_libs_crate_names_a_gdtf_crate_a_consumer_would_link` reads every `libs/*` manifest and fails
on a `gdtf_*` name in any of the three dependency tables, and
`the_libs_folder_and_the_cobalt_prefix_agree_in_both_directions` holds the folder and the
`cobalt_` prefix to each other. `no_file_under_libs_names_the_game` reads every tracked file
under `libs/`, tests included, and fails on the game's vocabulary: the game words plus the
command names a fixture might copy from the game's own wire. Two files hold those strings in
order to forbid them, and the scan names them in its `ALLOWED` list.

Rules 2, 3 and 5 are read by a person, not a test. The rule carries the weight; the test is a
backstop.

Nothing here says how a promotion is decided. That is a conversation.
