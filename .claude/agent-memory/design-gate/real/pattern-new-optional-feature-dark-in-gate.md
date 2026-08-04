---
name: pattern-new-optional-feature-dark-in-gate
description: A new cargo feature that no alias, no workflow, and no manifest names is dark — everything behind its cfg can be deleted and the suite still passes.
metadata:
  type: feedback
---

A diff that adds `[features] x = [...]` plus `#[cfg(feature = "x")]` code and tests ships
nothing the gate can see unless something turns `x` on.

**Why:** green is defined by the `.cargo/config.toml` aliases, and they name exactly two
features — `grimdark_turfwar/dynamic_linking` and `grimdark_turfwar/dev_tools` (`dcheck:17`,
`dclippy:18`, `dtest:19`; `doc-full:22` names `dev_tools` alone). CI names none at all:
`cargo test --workspace` (`.github/workflows/test.yml:50`), `cargo clippy --workspace
--all-targets -- -D warnings` (`clippy.yml:49`), `cargo fmt --all --check` (`fmt.yml:47`),
and `crates/gdtf_test_utils/tests/ci_workflow_features/check.rs:48-58` fails any CI
`--workspace` line that names `dynamic_linking`. The third way a feature gets turned on is a
manifest edge: `gdtf_app`'s `headless_test` is reached only because three crates ask for it
in their dev-dependencies (`crates/gdtf_test_utils/Cargo.toml:8`,
`crates/gdtf_battle_presenter/Cargo.toml:17`, `crates/gdtf_battle_input/Cargo.toml:18`).

**How to apply:** on any diff that adds a feature, grep `.cargo/config.toml`,
`.github/workflows/`, and every `Cargo.toml` for `<crate>/<feature>` and
`features = [`. Zero hits means the whole deliverable is revertible while green — say so and
name the mutation (delete the cfg-gated module and its tests). The fix that fits this repo is
a package-scoped step, `cargo test -p <crate> --features <f>`: putting the feature on a
`--workspace` alias turns it on for every crate in that build, which is usually the opposite
of what the ticket wanted. Related: [[pattern-manifest-dep-defeats-flag-guard]].
