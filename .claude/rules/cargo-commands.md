---
paths: ["**/*.rs", "**/Cargo.toml", ".cargo/config.toml"]
---

# Cargo commands: always the alias, never bare

`.cargo/config.toml` defines the aliases. Use them. A bare `cargo test`,
`cargo build`, `cargo clippy`, `cargo check` or `cargo run` is wrong here, and
`.claude/hooks/cargo-alias-gate.sh` blocks it.

[verification.md](./verification.md) owns which commands make up green. This file
owns how any cargo command is spelled, including a one-off during iteration.

## Why a bare command is wrong

### It rebuilds everything, twice

The aliases carry `dynamic_linking` and `dev_tools`. A bare command has a
different feature set, so cargo rebuilds the whole graph including bevy. The next
aliased command rebuilds it back. The two feature sets never share artifacts.

### It hides tests

A bare command drops `dev_tools`, so feature-gated modules do not compile in and
their tests do not run. A target with zero tests still exits 0, so the run looks
green while asserting nothing.

## Running a subset

Both forms take the alias:

```bash
cargo dtest -- selected_fire_mode    # name filter, matches nested modules
cargo dtest --test action_bar        # one integration suite
```

`--test` takes the suite target name, not a file. Integration tests here are
nested modules under a top-level suite, so `--test fire_one_spec` fails. Run
`cargo dtest --test __x__` to list the real target names. If two packages share a
target name, `--test` runs both.

## Not aliased

`cargo fmt`, `cargo doc --workspace --no-deps` and `cargo nextest run` have no
`d`-prefixed alias and are correct as written. `doc-full` is an alias and is not.

There is no release alias. Do not hand-type a run without dynamic linking to get
past the gate. Add the alias instead.
