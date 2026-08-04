---
name: pattern-dependency-cost-claim-vs-lockfile
description: A change that adds an optional feature to the definition of green writes "it adds no new dependency build beyond X" — check Cargo.lock, which usually adds six or seven crates.
metadata:
  type: feedback
---

When a change edits `.cargo/config.toml`, `.claude/rules/verification.md`, `CLAUDE.md` or
`docs/testing.md` to add a feature to the workspace gate, it almost always writes a
build-cost sentence to justify itself. Check that sentence against `Cargo.lock`, never
against the prose.

**Why:** the repo's whole reason for refusing `--all-features` is build cost —
`docs/testing.md:18` still argues the gate's feature list on exactly that ground ("it forces
a second full bevy build for no lint gain"). So a cost claim is load-bearing, and one
`schemars` derive is not one crate. `Cargo.lock` carries `schemars` (5080),
`schemars_derive` (5093), `serde_derive_internals` (5178), `dyn-clone` (2430), `ref-cast`
(4932), `ref-cast-impl` (4941), and a SECOND `syn` major — `syn 2.0.117` at 5404 alongside
`syn 3.0.3` at 5415. Seven packages behind a sentence that said one.

**How to apply:** one command settles it —

```bash
git diff develop -- Cargo.lock | grep -E '^[+-]name = ' | sort | uniq -c
```

A `+name` with no matching `-name` is a new crate; a `+name` for a name that already exists
is a second major version. Then grep the diff for `no new dependency`, `one new dependency`,
`no extra`, `adds no`. If the change cites an earlier decision as precedent, check whether
that decision MEASURED — a measured precedent does not license an unmeasured claim.

Second thing to check on the same diff: config, CI, rule and doc edits that add a feature to
the gate are usually outside what the ticket enumerated, which is a change to the repo's
definition of green made on implementer authority. See
[[pattern-deviation-cemented-by-rewriting-the-rule]] and
[[pattern-guard-suite-inventory-drift]].
