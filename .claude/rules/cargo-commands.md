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

The aliases carry `development`, the umbrella feature. A bare command has a
different feature set, so cargo rebuilds the whole graph including bevy. The next
aliased command rebuilds it back. The two feature sets never share artifacts.

### It hides tests

A bare command drops `development`, so the `dev_tools` and `mcp` modules do not compile in and
their tests do not run. A target with zero tests still exits 0, so the run looks
green while asserting nothing.

## Running a subset

All three forms take the alias:

```bash
cargo dtest -- selected_fire_mode               # name filter, matches nested modules
cargo dtest --test game_suite                   # one crate's integration binary
cargo dtest --test game_suite -- action_bar::   # one suite inside that binary
```

`--test` takes a target name. Every crate has one integration-test binary, `<name>_suite`
(`game_suite`, `editor_suite`, `battle_sim_suite`), and the old per-suite targets are modules
inside it, so `--test action_bar` fails. A suite is
selected by the name filter after `--`, with the trailing `::`. Run
`cargo dtest --test __x__` to list the real target names.

## Not aliased

`cargo fmt`, `cargo doc --workspace --no-deps` and `cargo nextest run` have no
`d`-prefixed alias and are correct as written. `doc-full` is an alias and is not.

`cargo mcpbuild` and `cargo edmcpbuild` are the release aliases: `--release` with
only the `mcp` feature, so the QA host can be driven against a release build.
They are not part of green. Do not hand-type a run without dynamic linking to get
past the gate. Add the alias instead.
