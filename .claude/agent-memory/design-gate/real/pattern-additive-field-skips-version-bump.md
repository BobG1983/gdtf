---
name: pattern-additive-field-skips-version-bump
description: A #[serde(default)] field added to a shipped wire payload meets every acceptance bullet without moving ProtocolVersion::CURRENT, and the rider is dropped silently in one direction.
metadata:
  type: feedback
---

A ticket that needs a new wire field can meet every acceptance bullet with `#[serde(default)]`
and no version bump. Both sides of every test are the same build, so the mismatch case is
never exercised.

**Why:** negotiation is exact equality with no capability exchange — a `Hello` carries the
client's number and `ProtocolVersion::CURRENT` is 14
(`crates/gdtf_qa_protocol/src/message/hello.rs:6-14`) — and the courier and the game are
separately built binaries that can easily be a version apart. `#[serde(default)]` only lets a
NEW decoder read an OLD frame; the other direction loses data. `RunCommand.options`
(`crates/gdtf_qa_protocol/src/message/request.rs:26-28`) has no `deny_unknown_fields`, so an
older host handed a `Run` carrying `options` drops the riders and runs the command anyway —
exactly the silent narrowing the `Unavailable { code: NotBuilt }` refusal exists to prevent.
The rule and its worked case are written down at
`docs/tooling/qa-commands.md:88-98`: the version number covers the shapes of the
`command` vocabulary, a field added to one of them moves it, and adding a COMMAND never does.

**How to apply:** on any change touching `gdtf_qa_protocol`, grep `ProtocolVersion::CURRENT`
and read the rule at `docs/tooling/qa-commands.md:96-98`. If a shipped payload
gained or changed a field and CURRENT did not move, ask (a) does the rule cover that shape,
and (b) is the exception written anywhere binding — a `Proposed` decision record is not the
contract while the accepted one says the opposite. Check the round-trip pins too: "old frame →
new decoder" is the safe direction and usually the only one tested
(`crates/gdtf_qa_protocol/src/message/test/round_trip.rs:84`,
`crates/gdtf_qa_protocol/src/command/test/timing.rs:25`).

Related: [[pattern-adding-a-command-reddens-three-more-assertions]].
