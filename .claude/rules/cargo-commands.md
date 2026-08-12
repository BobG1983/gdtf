---
paths: ["**/*.rs", "**/Cargo.toml", ".cargo/config.toml"]
---

# Cargo commands — always the alias, never bare

`.cargo/config.toml` defines the aliases. Use them. A bare `cargo test`,
`cargo build`, `cargo clippy`, `cargo check` or `cargo run` is wrong here even
when it looks harmless, and `.claude/hooks/cargo-alias-gate.sh` blocks it.

[verification.md](./verification.md) owns which commands make up green. This file
owns how any cargo command is spelled, including a one-off during iteration.

## Two reasons

**It rebuilds everything, twice.** The aliases carry `dynamic_linking` and
`dev_tools`. A bare command has a different feature set, so cargo rebuilds the
whole graph including bevy, and the next aliased command rebuilds it back. The
two feature sets never share artifacts.

**It hides tests.** A bare command drops `dev_tools`, so feature-gated modules do
not compile in and their tests do not run. Zero tests in a target still exits 0,
so the run looks green while asserting nothing.

## Running a subset

Both forms take the alias:

```bash
cargo dtest -- selected_fire_mode    # name filter, matches nested modules
cargo dtest --test action_bar        # one integration suite
```

`--test` takes the **suite target name**, not a file. Integration tests here are
nested modules under a top-level suite, so `--test fire_one_spec` fails —
`cargo dtest --test __x__` lists the real target names. If two packages share a
target name, `--test` runs both.

## Not aliased

`cargo fmt`, `cargo doc --workspace --no-deps` and `cargo nextest run` have no
`d`-prefixed alias and are correct as written. `doc-full` is an alias and is not.

There is no release alias. Running the game without dynamic linking is not a
thing to hand-type around the gate — add the alias when a release run is needed.
