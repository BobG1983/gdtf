---
name: pattern-clause-cites-untested-wiring-fn
description: A clause whose evidence is a file:line range in a production wiring fn that no test constructs — name the silent mutation, and rule NOTE only when the clause itself nominates those lines as its evidence.
metadata:
  type: feedback
---

A clause worded "X is built and tested — `serve.rs:13-37`" often cites a top-level wiring fn
that NOTHING executes in the suite. The testable core is a generic inner fn that takes the
already-built structure as an argument, so every test builds its own fixture pair and the
production construction never runs.

**Why:** `run_stdio` (`bins/gdtf_qa_mcp/src/serve.rs:13-37`) is the only place the two
`QaClient`s, the two `HostManager`s and their per-host `lifecycle_config()` are paired. Its
only caller is `bins/gdtf_qa_mcp/src/main.rs:4`. Every test drives the generic inner `run_loop`
(`serve.rs:40`, called from the in-file test at `serve.rs:124`). So swapping
`QaHost::Editor.lifecycle_config()` for the game's at `serve.rs:18`, or crossing `game_link`
with `editor_lifecycle` at `serve.rs:26-27`, compiles and passes the whole suite. The same
shape holds for `QaHost::port_from_env` (`bins/gdtf_qa_mcp/src/hosts/host.rs:62`): it reads
`std::env::var`, so only the `default_port()` fallback is exercised.

**How to apply:** when a clause's evidence is a file:line in a wiring fn, grep whether any test
calls that fn by name. If none does, name the concrete mutation that survives. Rule it a NOTE
when the clause text itself nominates the line range as the evidence and defers live proof to a
follow-up; rule it a violation when the clause claims the behaviour is "tested".

Related: [[pattern-cross-binary-constant-coupling]].
