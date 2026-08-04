---
name: pattern-mechanism-shipped-instead-of-evidence
description: A test that programs a fake child and then asserts against what it programmed proves the host's half only — it is never evidence about the real binary.
metadata:
  type: feedback
---

When a ticket asks for a reading taken off the real running binary, a fixture that both
sends the reply and is asserted against it cannot supply that reading. Shipping new
machinery next to such a fixture is not evidence either.

**Why:** `bins/gdtf_qa_mcp/tests/loopback.rs` binds a loopback listener and answers every
frame from a hand-written table — `answer()` (`:107-122`) returns a catalogue named
`fake-game` with an empty command list — and
`a_catalogue_round_trips_through_the_real_client` (`:125`) asserts the rendered text
contains `fake-game` (`:143`). That is a good test of the MCP host: it pins that the client
sends the handshake first and the tool request second (`:146-155`), which is the host's own
behaviour. It says nothing about what the game or editor actually answers, and it would
pass unchanged if neither binary could answer at all.

**How to apply:** ask which test goes red if the real binary answers differently. If the
answer is "none, the fixture writes the answer", the evidence clause is unmet however much
new code shipped beside it. Watch for the swap where a probe, a reply field, or a readiness
phase is *added* in place of the reading that was asked for — a new mechanism is not a
measurement. Compare the diff's file list against what the ticket asked for before
accepting it. Related: [[pattern-canned-fixture-carries-the-unasserted-value]].
