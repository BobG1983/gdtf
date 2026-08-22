---
paths: ["**/*.rs"]
---

# Bevy systems — SystemParam, queries, no arity expects

> **You MUST read and follow [plain-language.md](./plain-language.md) before writing anything
> here. It is binding on every word, and it is not optional.**

Why this rule exists: fat system signatures get silenced with
`#[expect(clippy::too_many_arguments)]` instead of being shaped. That hides
the real access set and trains agents to paper over structure debt. Bevy already
gives the tools — **bundle, filter, or split**. Reach for the right one; do not
default to `#[expect]`.

## The rule

1. **Do not add** `#[expect(clippy::too_many_arguments)]` (or `type_complexity`
   for the same reason) on a **system function** or on a free function that is
   only a system body extracted 1:1. Fix the shape instead.
2. **Pick the tool with the decision guide below** — not “always SystemParam”
   and not “always split.”
3. **Mirror existing house bundles** before inventing new shapes:
   - `gdtf_battle_sim` act_log `record/sources` (`SystemParam` per act family)
   - presenter playback / `static_map` / FX probes
4. **There is no last resort.** `#[expect]` is not available — not for
   `too_many_arguments`, not for `type_complexity`, not for any lint. The target
   is zero `#[expect]` in the repo. If §1–3 and the guide below do not solve a
   system, the access set is wrong and needs redesigning; say so and stop rather
   than silencing it.
5. **Do not** workspace-`allow` or workspace-`expect` a lint to “fix” noise.
   Keep the lint; remove expects by structuring systems.

---

## When to reach for what

Walk top to bottom. Stop at the first tool that fits. Combine tools when they
solve different parts of the same system (e.g. one `SystemParam` + one
`QueryData` is normal).

### 1. `QueryFilter` — *who* is in the query

**Use when:** the fat part is a long `With` / `Without` / `Or` / tuple of
marker filters, or the same filter set is copy-pasted on several queries.

**Do:** `#[derive(QueryFilter)]` (or a named filter type alias) for that
membership set. The system still takes `Query<Data, MyFilter>`.

**Do not use for:** packing `Res` / `Commands` / writers — filters only apply
to queries. Do not invent a filter for a one-off two-marker query used once.

**Smell that says “filter”:** `Query<&A, (With<B>, With<C>, Without<D>, …)>`
appears more than once, or the filter tuple alone trips complexity.

### 2. `QueryData` — *what* you read/write on each entity

**Use when:** a single query’s data tuple is long
(`Query<(&A, &B, &mut C, Option<&D>, …)>`), or the same data shape is shared
by several systems / helpers.

**Do:** `#[derive(QueryData)]` (mutable or read-only as needed) so the system
takes `Query<MyRow, …>` or `Query<MyRowItem, …>`. Name the row for the job
(`GangerPoseRow`, not `Data`).

**Do not use for:** mixing resources and entity data — that is `SystemParam`.
Do not wrap a 2-field query “for style.”

**Smell that says “QueryData”:** arity/`type_complexity` is on the **entity
columns**, not on the system’s resource list.

### 3. `SystemParam` — *what the system needs as a unit of work*

**Use when:** several **system-level** borrows always go together for one job:

- multiple `Res` / `ResMut` / `MessageReader` / `MessageWriter` / `Commands`
- two or more queries that are only meaningful as a set
- a reusable “probe” or “writers” bag shared by sibling systems

**Do:** `#[derive(SystemParam)] struct JobName<'w, 's> { … }`. Name it for the
**job** (`SuppressionSources`, `DrawnWriters`, `FxPipelineProbe`), never
`Params` / `SystemArgs` / `Helpers`.

**Do not use for:**

- stuffing an entire god-system’s access set into one mega-param (split first)
- a single `Res` or single `Query` (no bundle of one)
- hiding unrelated borrows so the signature looks small but the struct is a junk drawer

**Smell that says “SystemParam”:** the system has many **top-level** params that
are not one query’s columns; two systems take the same 3–6 params in the same
order; clippy complains about argument count, not query tuple length alone.

### 4. Split systems — *when one schedule unit is doing two jobs*

**Use when:**

- one system handles multiple message types / phases with independent access
- read-only prepare and write apply can run as separate systems
- spawn-once vs per-tick vs cleanup share a file but not a borrow set
- a `SystemParam` would only become a façade over two unrelated jobs

**Do:** two (or more) systems, each with a clear name and the minimal params;
register both in the plugin (order with `.chain()` / explicit sets when
needed).

**Do not use for:** splitting a single atomic update that must see one
consistent borrow set mid-logic (keep one system, bundle instead). Do not
split only to game the arity count while sharing a secret global.

**Smell that says “split”:** `if reader A …; if reader B …` with little shared
state; match arms that each need different queries; function already named
`foo_and_bar`.

### 5. Other Bevy tools (use when the problem matches)

| Tool | When |
| --- | --- |
| **`ParamSet`** | Two queries (or params) **conflict** if taken together; you need them in one system and will access them **non-overlapping** in time (e.g. iterate A then B). Not a general arity fix. |
| **`Local<T>`** | Per-system scratch that is not world state (frame counters, small caches). Not for shared game state. |
| **`Option<Res<T>>`** | Resource may be absent; system no-ops or branches. Do not `expect` the resource. |
| **Extract helper free fn** | Pure logic on already-fetched data (no world access). Keeps systems thin; does not replace bundling. |

### 6. Nothing — there is no sixth option

`#[expect]` is not a tool. If §1–5 do not solve the system, the access set
itself is wrong. Report that and stop; do not silence the lint.

---

## Decision cheat sheet

```text
Fat Query<(…many components…)>     → QueryData
Fat With/Without/Or filter tuple   → QueryFilter
Many Res/Query/Writer top-level    → SystemParam (job-named)
Same param bag on sibling systems  → shared SystemParam
Two jobs / two message families    → split systems
Conflicting queries, one system    → ParamSet (only if must be one system)
Still over the limit                → the access set is wrong; redesign
```

**Order of attack when clippy fires on a system:**

1. Is it query **data** width? → `QueryData`
2. Is it query **filter** width? → `QueryFilter`
3. Is it **system param** count? → `SystemParam` (cohesive groups only)
4. Is it **multiple jobs**? → split
5. Still stuck? → the access set is wrong. Redesign it, or report and stop.

---

## Not this rule

- Non-system helpers with many true domain args — still prefer newtypes and
  structs; this file is about **ECS systems and their param lists**.
- `#[allow]` is forbidden project-wide (`clippy::allow_attributes = deny`), and
  `#[expect]` is not the way around that.

## Agent checklist

When clippy reports `too_many_arguments` or `type_complexity` on a system:

1. List every borrow (queries vs resources vs messages vs commands).
2. Apply the cheat sheet — `QueryData` / `QueryFilter` / `SystemParam` / split.
3. Reuse a house bundle if one already matches the job.
4. Never add `#[expect]`. If nothing above fits, the access set is wrong —
   report it rather than silencing the lint.
