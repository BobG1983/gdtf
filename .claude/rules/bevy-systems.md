---
paths: ["**/*.rs"]
---

# Bevy systems: SystemParam, queries, and no `#[expect]`

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything here. It is
> binding on every word, and it is not optional.**

## The rule

1. `#[expect]` is not available for any lint, anywhere in the repo. Fix the shape instead.
2. Reuse the bundles this repo already has before writing a new shape:
   - `gdtf_battle_sim` act_log `record/sources` (one `SystemParam` per act family)
   - presenter playback / `static_map` / FX probes
3. If the rules above and the guide below do not solve a system, the set of borrows is wrong
   and needs redesigning. Say so and stop.
4. Do not workspace-`allow` or workspace-`expect` a lint.

## Choosing the tool

List every borrow first: queries, resources, messages, commands. Then walk top to bottom.
Stop at the first tool that fits. Combine tools when they solve different parts of the same
system. One `SystemParam` plus one `QueryData` is normal.

### 1. `QueryFilter`: which entities are in the query

Use it when the long part of the signature is a `With` / `Without` / `Or` tuple of marker
filters, or when the same filter set is copy-pasted across several queries.

Write `#[derive(QueryFilter)]`, or a named filter type alias, for that set of markers. The
system still takes `Query<Data, MyFilter>`.

Do not use it to pack `Res` / `Commands` / writers. Do not invent a filter for a two-marker
query used once.

### 2. `QueryData`: what you read and write on each entity

Use it when a single query's data tuple is long
(`Query<(&A, &B, &mut C, Option<&D>, …)>`), or when several systems or helpers share the
same data shape.

Write `#[derive(QueryData)]`, mutable or read-only as needed, so the system takes
`Query<MyRow, …>` or `Query<MyRowItem, …>`. Name the row for the job: `GangerPoseRow`, not
`Data`.

Do not use it to mix resources and entity data. That is `SystemParam`. Do not wrap a
two-field query for style.

It is a `QueryData` problem when the arity or `type_complexity` comes from the entity
columns, not from the system's resource list.

### 3. `SystemParam`: what the system needs as one unit of work

Use it when several system-level borrows always go together for one job:

- multiple `Res` / `ResMut` / `MessageReader` / `MessageWriter` / `Commands`
- two or more queries that only make sense as a set
- a probe or a set of writers shared by sibling systems
- two or more sibling systems take the same three to six borrows in the same order

Write `#[derive(SystemParam)] struct JobName<'w, 's> { … }`. Name it for the job:
`SuppressionSources`, `DrawnWriters`, `FxPipelineProbe`. Never `Params`, `SystemArgs` or
`Helpers`.

Do not bundle the borrows of a system that does several jobs. Split that system first. Do not
bundle a single `Res` or a single `Query`. Do not hide unrelated borrows to make the signature
look small.

It is a `SystemParam` problem when the system has many top-level params that are not one
query's columns.

### 4. Split systems: when one system does two jobs

Split when:

- one system handles several message types or phases with independent access
- a prepare step that only reads and an apply step that writes can run as separate systems
- spawn-once, per-tick and cleanup share a file but not a borrow set
- a `SystemParam` would only wrap two unrelated jobs under one name

Write two or more systems, each with a clear name and the smallest param list it needs.
Register all of them in the plugin, ordering with `.chain()` or explicit sets when needed.

Do not split a single update that needs one consistent set of borrows while it runs. Keep it
as one system and bundle instead. Do not split only to get the argument count down while the
halves still share a global.

It is a split when match arms each need different queries, or when the function is already
named `foo_and_bar`.

### 5. Other Bevy tools

| Tool | When |
| --- | --- |
| `ParamSet` | Two queries (or params) conflict if taken together, and you need them in one system and access them at different times (iterate A, then B). Not a general arity fix. |
| `Local<T>` | Per-system scratch that is not world state (frame counters, small caches). Not for shared game state. |
| `Option<Res<T>>` | The resource may be absent, so the system no-ops or branches. Do not `expect` the resource. |
| Extract helper free fn | Pure logic on data you already fetched, with no world access. Does not replace bundling. |

## What this rule does not cover

- Non-system helpers with many real domain arguments. Prefer newtypes and structs there too.
- `#[allow]` is forbidden project-wide (`clippy::allow_attributes = deny`).
