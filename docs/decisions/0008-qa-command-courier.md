---
name: "ADR 0008: the QA command layer — one typed command list per host, carried by a command-agnostic courier"
description: A host publishes its own list of typed QA commands; two frozen envelope variants and two MCP tools carry any of them, so adding a command touches one file and one line.
---

# 0008. The QA command layer — one typed command list per host, carried by a command-agnostic courier

## Status

`Proposed` — 2026-07-31, driven by the user's design ruling on GTW-934 and the GTW-938 epic
that carries it. Built in phase A by GTW-939 (the wire vocabulary), GTW-940 (the handshake
gate), GTW-941 (`crates/gdtf_qa_command`) and GTW-942 (the game host scaffold, `app.phase`,
and the courier's two tools).

It moves to `Accepted` in GTW-943, which also deletes the surface
[ADR 0007](0007-net-qa-command-discoverability.md) describes and flips that ADR to
`Superseded by 0008`. Until then 0007's per-family query pair is still shipped code and both
surfaces exist side by side.

## Context

Every QA affordance the game and the content editor have gained since GTW-736 cost the same
four edits: a `QaRequest` variant, a `QaResponse` variant, a router arm, and an MCP tool with
a hand-written JSON Schema. The protocol version moved for most of them — it reached 13 —
and each addition had to be made twice if both hosts wanted it.

Two consequences drove the reversal recorded on ADR 0007. First, the cost of an affordance
was high enough that affordances were not added: the game serviced `StartBattle`,
`StepperControl`, `FocusControl` and the editor's whole query pair for weeks before any
client tool sent them, because the client half was a separate edit that kept being deferred
(GTW-760, GTW-766, GTW-802, GTW-808 each closed one of those gaps after the fact). Second,
the hand-written tool schema and the host's actual accepted shape are two accounts of one
thing, and nothing kept them in step.

ADR 0007 chose per-family query ENUMS over a runtime command registry, on the reasoning that
one live state family did not justify the registry's machinery. The editor family arrived,
and the user's GTW-934 ruling reversed that choice.

## Decision

**A host publishes ONE list of typed commands, and the wire carries any command in two
variants that never change.**

- A command is a unit struct implementing `QaCommand` (`crates/gdtf_qa_command`): two
  associated types whose JSON Schemas are DERIVED, a name, a summary, a declared timing, a
  pure availability predicate over that host's facts type, and the registration of an
  ordinary Bevy system.
- A host owns one `&[&dyn ErasedCommand<F>]` — the game's is `GAME_COMMANDS` in
  `crates/gdtf_app/src/dev/net_qa/commands/set.rs`. The catalogue walk, the admission scan
  and the registration walk all read that same slice, so a command cannot be advertised
  without being admissible or wired.
- The wire surface is exactly two request variants (`Catalogue`, `Run`) and two response
  variants (`Catalogue`, `Outcome`). A command is DATA inside them, so adding one moves no
  protocol version and adds no variant.
- The MCP courier exposes exactly two tools, `commands` and `run`
  (`bins/gdtf_qa_mcp/src/mcp/courier/`). **Neither names a command** — not in its schema, not
  in its description. A client discovers what it can call by calling `commands` against the
  running host, and the schemas it reads there are the ones the host derived from its own
  Rust types.
- **Exactly one system per host drains the request inbox.** `NetInbox::drain()` takes
  everything in the channel, so a second router would swallow the first one's requests
  nondeterministically. The command layer therefore WIDENS the existing router with
  `Catalogue` and `Run` arms rather than registering one of its own.
- A command's own preconditions are answered by its availability predicate, as
  `Unavailable { code, note }`. They are not route-time state gates: the router can only ever
  answer a blunter error, and a per-command precondition is per-command knowledge.

Adding a command is a file under a host's `commands/` directory, one line in that host's
list, and a test.

## Consequences

- The cost of a QA affordance drops from four coupled edits plus a version bump to one file
  and one line. That is the change the whole epic exists to make.
- A published schema cannot disagree with an accepted shape: both are derived from the same
  Rust type. `deny_unknown_fields` on an argument type reaches the wire as
  `"additionalProperties": false`, and a body that violates it comes back as `BadArguments`
  carrying that same document.
- A client must call `commands` before it can call anything: the vocabulary is read from the
  running host, not from the tool list. That is deliberate — availability is read from live
  state, and a static list could only ever describe a host that is not running.
- The courier cannot validate a command's arguments, because it does not know them. A wrong
  body costs one extra round trip, answered with the schema needed to fix it.
- Two things must be checked per host, by that host's own suite, because no test in the
  shared crate can see a host's slice: that no two commands claim one name, and that every
  published schema document is parseable JSON. `gdtf_qa_command::test_support` publishes both
  assertions for exactly that reason.
- The riders on a call (`await_ready`, `capture`) are declared but not built. A call carrying
  one is answered `Unavailable { code: NotBuilt }` rather than run with the rider dropped —
  an honest refusal, not a silent narrowing of what was asked.
- GTW-942 added two `#[serde(default)]` fields to already-shipped payloads — `RunCommand.options`
  and `CommandEntry.timing` — and moved `ProtocolVersion::CURRENT` from 13 to 14 for them. The
  defaults only make a NEW decoder read an OLD frame correctly (pinned by
  `a_run_encoded_without_options_decodes_as_a_plain_call` and
  `a_row_encoded_without_a_timing_decodes_as_immediate`); the other direction loses data, because
  `RunCommand` has no `deny_unknown_fields` — a version-13 host handed a `Run` carrying `options`
  would drop the riders and run the command anyway. That is the silent narrowing the `NotBuilt`
  refusal exists to prevent, and the courier and the game are separately built binaries that can
  easily be a version apart. So the rule stated on `ProtocolVersion::CURRENT` holds unchanged: the
  number covers the shapes of the `command` vocabulary, and a field added to one of them moves it.
  What still never moves it is a COMMAND — a name, its arguments, its reply.
- ADR 0007's per-family query pair becomes redundant: a topic query is a command with a
  narrower reply. GTW-943 deletes it.

## Alternatives considered

- **Keep the per-family query enums (ADR 0007's decision).** Rejected by the user's GTW-934
  ruling. The pattern held for one family and strained at two: every new affordance still
  cost a wire variant on both hosts, and discoverability stopped at "which topics will you
  answer" rather than "what can I call and what does it take".
- **One MCP tool per command.** Rejected: it reintroduces the exact coupling this removes —
  a host command with no client tool is unreachable, which is the failure GTW-760, GTW-766,
  GTW-802 and GTW-808 each had to repair after the fact.
- **A second router for the command arms.** Rejected on a mechanical ground: `NetInbox::drain()`
  is `rx.try_iter().collect()`, so two routers reading the inbox in one frame means whichever
  runs first swallows the other's requests. Pinned by
  `crates/gdtf_app/tests/net_qa/command_set.rs`.
- **Hand-written argument schemas in the courier.** Rejected: that is the drift this design
  removes. A schema written beside the tool is a second account of a shape the host already
  knows, free to go stale on any command change — and the client only ever sends what the
  tool advertises.
- **A shared command list across both hosts.** Rejected: a command reads its host's world.
  Tying each command's facts type to its host's makes a cross-host entry fail to compile at
  the slice literal, which is a stronger guarantee than a convention.
