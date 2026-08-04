---
name: dont-build-tooling-to-prove-the-obvious
description: Never build MCP tooling or a capture mechanism to prove what reading the code or a test already proves — pick the cheapest sufficient evidence and read the existing tests first
metadata:
  type: feedback
---

Never build MCP tooling, or engineer a capture mechanism, to prove something that reading the code or
a test trivially proves. This is a user ruling.

The evidence ranking, best to worst (user's words):

    integration test  ≈  unit test  >  reading the code  >>  adding MCP tooling to "test"

Tests are the BEST evidence, not an expensive fallback — an integration test and a unit test are
worth about the same, and both beat reading the code. Building MCP tooling in order to test something
is the WORST option, below simply reading the code. It is not a step up the ladder; it is off the
ladder. Existing MCP tools are fine for DRIVING the app — do not BUILD tooling to obtain evidence.

**Why:** one ticket inherited the obligation "read the editor's `Load`-phase topic list off a running
process". The editor's asset pass ends in a few frames, so polling caught `Editing` every time across
three launches. Instead of asking whether that evidence was needed, a whole launch-time readout was
engineered — a new module, a new launch option, plus tests — and it still could not close the
obligation, so it spawned a follow-up ticket. All of it was already covered by a test that runs the
editor's own plugins. The user called it overblown and cut the mechanism; none of those symbols
survive in the tree.

That test does survive, at `crates/gdtf_content_editor/tests/net_qa_hello/`. `harness.rs` binds a
real loopback listener with `NetQaEditorPlugin::listening(NetQaPort::new(0))`; `client.rs` connects a
real `TcpStream` speaking the real framing codec; `load_case.rs` connects BEFORE the app runs a single
frame and asserts the editor is still in `EditorState::Load`, which makes the observation
deterministic. A spawned-process poll is a race; this is not. The only difference was
`std::process::Command` versus same-process, which cannot affect phase-gated routing.

`docs/tooling/qa-commands.md` now states the same rule for the QA channel: "A QA command that exists
to read internal state so an agent can verify correctness is a test that costs more and proves less.
Write the test instead."

**How to apply:**

- Before accepting any "prove it live" obligation, READ THE EXISTING TESTS. If one already exercises
  the real code path over the real transport, the obligation is discharged — say so and move on.
- "Prove it against the real binary" exists because a `MinimalPlugins` stand-in once fabricated a
  default mode. It does NOT mean every claim needs a spawned process. A test that builds the real app
  and speaks the real protocol is not a stand-in. Applying the rule past its purpose is its own
  failure.
- If a clause demands evidence that is a RACE to capture, the clause is wrong — stop and challenge
  it, do not engineer around it.
- A LOG LINE is almost never the right proof of a state. One clause demanded the editor's startup
  line naming its bound port. The line asserts only two facts — the QA channel is on, and the port —
  and the run had already collected better proof of both: `lsof +c 0` showing the socket genuinely
  LISTENing, and a query answered THROUGH that listener. A print saying "I am listening" is weaker
  than a query answered by the thing listening. (Child log tails are now readable on the success path
  via the `logs` MCP tool — `HostManager::child_output` reads the tail from the running child — so
  the point is that the log line was never the right proof, not that it was unreachable.)
- The tell generalizes: when a clause names an *observation* (a log line, a frame, a startup-phase
  snapshot) rather than a *fact*, restate it as the fact and ask what the cheapest proof of that fact
  is. Usually something the run already does.

Related: [[probe-the-app-dont-grep]], [[no-ci-run-urls-as-evidence]],
[[quote-the-ticket-spec-the-files]].
