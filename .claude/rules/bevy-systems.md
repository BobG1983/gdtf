---
paths: ["**/*.rs"]
---

# Bevy systems — SystemParam, queries, no arity expects

Why this rule exists: fat system signatures get silenced with
`#[expect(clippy::too_many_arguments)]` instead of being shaped. That hides
the real access set and trains agents to paper over structure debt. Bevy already
gives the tools — **bundle, filter, or split**.

## The rule

1. **Do not add** `#[expect(clippy::too_many_arguments)]` (or `type_complexity`
   for the same reason) on a **system function** or on a free function that is
   only a system body extracted 1:1. Fix the shape instead.
2. **Prefer, in order:**
   1. **`#[derive(SystemParam)]`** — group resources/queries/writers that always
      travel together into one param (name it for the job, not `Params`).
   2. **`QueryData` / `QueryFilter`** (or existing query type aliases) — collapse
      wide `Query<(…, …, …)>` tuples so arity and complexity both drop.
   3. **Split systems** — one clear job per system (per message family, read vs
      write phase, spawn vs tick). Register both; do not keep a god system.
3. **Mirror existing house bundles** before inventing new shapes:
   - `gdtf_battle_sim` act_log `record/sources` (`SystemParam` per act family)
   - presenter playback / `static_map` / FX probes
4. **`#[expect(clippy::too_many_arguments)]` is last resort only** when:
   - the parameter list *is* the documented public access contract, and
   - bundling would hide independent borrows a reader must see, and
   - a short `reason = "…"` names that contract (not “Bevy needs many args”).
   Prefer a human-approved exception in review over agent-added expects.
5. **Do not** workspace-`allow` `too_many_arguments` to “fix” noise. Keep the
   lint; remove expects by structuring systems.

## Not this rule

- Non-system helpers with many true domain args — still prefer newtypes and
  structs; this file is about **ECS systems and their param lists**.
- Other `#[expect]` uses (casts already workspace-allowed, rare restriction
  lints) — see lint policy in root `Cargo.toml`; still need `reason =`.
- `#[allow]` is forbidden project-wide (`clippy::allow_attributes = deny`);
  use `#[expect]` only when an expect is justified under (4).

## Agent checklist

When clippy reports `too_many_arguments` or `type_complexity` on a system:

1. List what the system borrows.
2. Bundle co-borrowed sets into `SystemParam` (or reuse an existing bundle).
3. If still fat, split by responsibility and schedule both.
4. Only if (4) applies, add `#[expect]` with a concrete reason — never as the
  first move.
