---
name: "ADR 0007: net_qa command discoverability — per-family query enums over a runtime registry"
description: How agents discover and invoke state-specific net_qa commands as more state families (editor, future scenes) come online — a per-family query-enum pair, not a runtime command registry.
---

# 0007. net_qa command discoverability — per-family query enums over a runtime registry

## Status

`Accepted` — 2026-07-24 (user-ratified on GTW-786), driven by a user design question on GTW-786
(the editor net-QA/MCP extension epic). Proposed 2026-07-24. GTW-805/806 (GTW-786's protocol
children) lock their wire enums against it.

## Context

The `net_qa` protocol's `query_state` (`QaRequest::GetBattleState` → `BattleView`,
`crates/gdtf_qa_protocol/src/view/battle.rs:38`) returns one large payload — gangers, terrain,
buttons, fog, selection, turn — every time, regardless of what the calling agent actually wants.
Two related pains follow: an LLM driver has to parse a big blob for a small answer, and as more
state families appear (the content editor, future scenes) there is no established pattern for
exposing their reads/acts without either duplicating today's `query_state` sprawl per family or
bolting on a new discovery mechanism.

There are two distinct protocol layers in play, and the discoverability question lands entirely
on the inner one:

1. **Outer (MCP) layer**, between the LLM client and the `gdtf_qa_mcp` host binary. MCP already
   has native discovery: `tools/list` returns every tool's `{name, description, inputSchema}`.
   gdtf implements this today as a fixed enum walk (`bins/gdtf_qa_mcp/src/mcp/tools/name.rs`,
   `ALL`, 16 tools since GTW-808 added the four editor tools — `get_editor_query_options`,
   `query_editor`, `launch_editor`, `stop_editor` — to the 12 that GTW-802's `focus_control`
   completed). gdtf does not declare `listChanged`; the tool list is static per
   session.
2. **Inner (net_qa wire) layer**, between the MCP host and the running game process — and,
   since GTW-808, the running content-editor process too. This is `QaRequest`
   (`crates/gdtf_qa_protocol/src/envelope/request.rs`), a fixed serde enum of 15 typed variants
   (GTW-802 added `FocusControl`; GTW-805 added the editor pair; GTW-939 added `Catalogue` and
   `Run` beside them, the first two of the command layer). The editor is wired into
   `net_qa` as of GTW-804/805, and GTW-808 drives it as a second host on default port `7617`:
   `QaRequest` carries `GetEditorQueryOptions` and `QueryEditor(EditorQueryKind)`, and
   `crates/gdtf_content_editor/src/net_qa/router.rs` answers them. There is no flat single-view
   editor request variant and none is planned.

A partial capability-advertisement mechanism already exists at the inner layer:
`AppFlowView.available: Vec<RequestKindNet>` (`view/appflow.rs:171`) is a runtime, state-filtered
list of which request kinds the server will service right now — battle-only kinds vanish until a
battle runs, act-bearing kinds vanish while the presenter replays. It runs over a closed,
compile-time enum, not a self-registering open set.

A user design comment on GTW-786 (2026-07-24) proposed generalizing this into a self-registering
command registry: state-specific commands revealed via a `get_available_commands` call and
invoked via `submit_command`, with each state/scene registering its own commands to a shared
crate (`app.register_command(state, command)`), reusable across the game and the editor. The
user explicitly asked for this to be explored via an adversarial design workflow — their idea as
one input among several, checked against MCP/tool-discovery best practices and any prior art for
LLM agents driving games via a control protocol — rather than built as proposed.

Three candidates were generated and independently judged:

1. **Self-registering command registry** — most reuse-oriented (one registry serves any state,
   game or editor), but adds a second in-protocol discovery layer duplicating what MCP's own
   `tools/list` already gives an agent, introduces a real Bevy startup-ordering risk class
   (`world.register_system` at plugin `build()` time, keyed by state), and degrades typing
   (`SubmitCommand { id: CommandId, args: CommandArgs }` is stringly-typed dispatch wearing
   newtypes).
2. **Granular typed requests** — split `BattleView` into `GetTeam`/`GetCover`/`GetVisibleMap`/etc.,
   each a normal `QaRequest` enum variant with its own router arm and MCP tool entry. Safest and
   cheapest: compile-time exhaustive, rides MCP's existing discovery unchanged. Its weakness:
   every new command is a three-file edit (protocol enum, router, MCP tool table), and nothing
   shares a pattern across state families.
3. **Hybrid: per-family query enum + options menu** — two variants per state family
   (`Get<Family>QueryOptions`, `Query<Family>(kind)`), where `kind` is a small closed enum of
   that family's topics. Keeps everything compile-time exhaustive like (2), but collapses N flat
   MCP tools down to two per family, and gives agents both static discovery (MCP's `inputSchema`
   enumerates legal topics) and live discovery (`Get*QueryOptions` reuses the existing
   `AppFlowView.available` filter pattern to say which topics are actually available right now).

## Decision

We will adopt the per-family query-enum hybrid (candidate 3), built as a direct evolution of
candidate 2's payload split — not skip straight to the wrapper without first splitting the
payload.

Concretely, for the battle family:

```rust
// crates/gdtf_qa_protocol/src/envelope/request.rs
GetBattleQueryOptions,              // "what can I ask about a battle right now?"
QueryBattle(BattleQueryKind),       // ask ONE topic

pub enum BattleQueryKind {
    Team, Enemies, Cover, VisibleMap, SeenMap, Turn, Selection,
}
```

- `GetBattleQueryOptions` replies with `Vec<BattleQueryTopic>` (topic + one-line description),
  filtered by current `AppState`/`caught_up` — the exact same filter `AppFlowView.available`
  already runs, applied to a new enum instead of `RequestKindNet::ALL`. No new mechanism, one
  more filter site.
- `QueryBattle(kind)` returns a `BattleQueryView` with one variant per topic (`Cover(CoverView)`,
  `Team(TeamView)`, …) — each a genuinely small, purpose-scoped typed view carved out of today's
  `BattleView`. `GetBattleState`/`BattleView` stays as a compatibility path for one release, then
  retires once drivers migrate.
- Router change is one arm in `router.rs`: match `QueryBattle(kind)` and dispatch to the same
  field-population code that already builds `BattleView`, returning only the relevant sub-struct.
- The MCP surface (`bins/gdtf_qa_mcp/src/mcp/tools/name.rs`) gets exactly two new fixed tool
  entries — `get_battle_query_options`, `query_battle` — not N. `query_battle`'s `inputSchema` is
  the `BattleQueryKind` enum, so an LLM sees the full legal topic list natively at the MCP layer.

When the editor family needs QA support, repeat the identical two-variant recipe:
`GetEditorQueryOptions` / `QueryEditor(EditorQueryKind)`, plus its own availability predicate. As
built (GTW-805) that predicate is `topic_available(kind, model)`, in
`crates/gdtf_content_editor/src/net_qa/snapshot/topics.rs` — a wildcard-free match on
`EditorQueryKind` that reads the query model (`EditorQaModel`, with `mode()`, `session()`,
`drafts()` and `report()`), not the editor's `EditorState`. `Readiness` is always answerable;
every other topic is offered exactly when the resource it reads is present, and that is not the
same window for all of them — `Validation` is offered from the editor's first frame, `Load`
included, because `ContentIntegrityReport` is `init_resource`'d while the app is still being
built (GTW-879).

The options list and the answer share that one predicate over a single model read:
`options_view` (same file) filters `EditorQueryKind::ALL` through it, and
`answer_editor_query` in `crates/gdtf_content_editor/src/net_qa/router.rs` calls it before
producing a topic's view, refusing with `BadRequest` when it says no. So "advertised" and
"answerable" cannot drift apart — a stronger guarantee than a separate router-side filter would
give, and the reason the editor family needs no filter clause of its own. This is still a
deliberate, small, per-family diff — an accepted cost, not a gap to engineer away preemptively.

We will **not** build the self-registering registry (candidate 1) now. Its central insight —
that state-scoped discovery needs to generalize across the game and the editor — is correct, and
should inform the next escalation if it's ever needed. Its mechanism trades away compile-time
safety and adds a Bevy startup-ordering risk class for a reuse benefit the codebase does not need
at roughly 10-15 commands across one family. Revisit it only if a third or fourth state family
arrives and the per-family two-variant pattern is visibly repeating itself with real
router/wiring duplication — observed, not projected.

## Consequences

- Two discovery signals must stay coherent: the MCP `inputSchema` enum (static, "what topics
  exist") and the runtime `Get*QueryOptions` reply (dynamic, "what's live now") must never drift
  — a topic added to `BattleQueryKind` without a corresponding `available`-filter entry silently
  disappears from live discovery. Guard this with a router-side exhaustive match (no wildcard
  arm) so the compiler enforces it.
- The per-family `Get*QueryOptions`/`Query*(kind)` pair is hand-duplicated for every new family —
  nothing mechanically shares it between battle and editor. Accepted, bounded cost.
- Existing drivers calling `GetBattleState` keep working during the compatibility window; cutover
  to per-topic `QueryBattle` calls is opt-in, not a breaking day-one change.
- GTW-805 built the editor's two-variant pair against this decision (`GetEditorQueryOptions` /
  `QueryEditor(EditorQueryKind)`), confirming the recipe transfers across families.
- A written trigger condition exists for reconsidering candidate 1: a second live state family in
  production QA use that visibly strains the per-family pattern.

## Alternatives considered

- **Candidate 1 (self-registering registry)** — rejected for now; see Decision above. Not wrong
  in kind, premature at current scale.
- **Candidate 2 alone (flat granular requests, no wrapper)** — under-serves discoverability at
  scale: fine at 10 variants, becomes a genuinely bloated MCP tool list once a second family's
  commands pile on flat. Candidate 3 fixes this by wrapping the same split in one enum layer.
- **Do nothing (keep `query_state`/`GetBattleState` as the only read)** — rejected; it was the
  original complaint (payload bloat) and blocks the editor extension from having any read
  surface at all.
