---
name: gate-untested-surfaces-inventory
description: Tests-lens list of the places in gdtf with no automated backstop at all, so a change there is always silent.
metadata:
  type: feedback
---

On the tests lens, name the change that slips past for every new behaviour. Two places in
this repo have no automated backstop at all, so anything done to them stays green.

1. **`.cargo/config.toml` aliases.** No test reads the file — grepping `crates/` and `bins/`
   for `.cargo/config` finds nothing. The aliases at `.cargo/config.toml:9-22` (`drun`,
   `dbuild`, `edrun`, `edbuild`, `dcheck`, `dclippy`, `dtest`, `doc-full`) are what "green"
   means and what a developer runs. Delete or narrow one and the whole suite still passes.
2. **Plugin wiring reached only through `from_env`.** Both hosts add their QA plugin from a
   single line no test executes: `crates/gdtf_content_editor/src/app.rs:26`
   (`NetQaEditorPlugin::from_env()`) and `crates/gdtf_app/src/dev/plugin.rs:26`
   (`NetQaPlugin::from_env()`). Every test enters through a different constructor —
   `NetQaEditorPlugin::listening` (`crates/gdtf_content_editor/tests/net_qa_hello/harness.rs:9`
   and the two other editor QA suites), `NetQaPlugin::listening`
   (`crates/gdtf_app/tests/net_qa/socket_support.rs:64`), `NetQaPlugin::with_channels`
   (`crates/gdtf_app/tests/net_qa/battle_fixture.rs:44`). Delete either `add_plugins` line and
   the suite stays green while the real binary opens no port.

**Why:** the app wrappers themselves are built only by their binaries. `MapEditorApp::new` has
one caller, `bins/gdtf_content_editor/src/main.rs:6`; `GdtfApp::new` has one caller,
`bins/grimdark_turfwar/src/main.rs:6`. A test harness that assembles the app differently from
the binary always leaves the binary's own lines unproved.

**How to apply:** when a ticket's own words name a manual run as the evidence for a clause,
that is legitimate and not a test violation — but say plainly which change stays silent
afterwards, so the gap is on the record rather than assumed covered. Both entries above are
facts about the tree and they rot: re-check them before citing, and delete one the moment a
test covers it.

Related: [[gate-manifest-guard-vacuity-check]].
