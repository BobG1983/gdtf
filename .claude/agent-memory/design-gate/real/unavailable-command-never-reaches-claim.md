---
name: unavailable-command-never-reaches-claim
description: A command whose availability is not Available is refused at route time, so its arguments are never decoded and its reply is never parked — clauses about BadArguments or the deadline race are unreachable on the menu fixture.
metadata:
  type: feedback
---

`admit` checks the name, then the riders, then availability, and returns `Admission::Unavailable`
before anything else runs — `crates/gdtf_qa_command/src/dispatch/admit.rs:88-106` (refusals at
`:98` and `:103`), called from `crates/gdtf_app/src/dev/net_qa/router/route.rs:31`, which replies
`unavailable_reply(refusal)` at `:35`. A refused call never reaches the inbox, so `claim_calls`
(`crates/gdtf_qa_command/src/dispatch/claim.rs:23-36`) never sees it. Only admitted rows get their
arguments decoded (`:31`, with `bad_arguments` at `:15-20` replied at `:33`) or parked (`:32`,
`push_new` into `PendingQueue<CommandCall<C>>`). Both are unreachable for a command whose
availability word is not `Available` in the fixture's resting state.

Two landed cases stop proving anything the moment they are copied for a state-gated command:

- `crates/gdtf_app/tests/net_qa/commands.rs:129`
  (`an_unknown_argument_is_bad_arguments_with_the_schema`) — the copy answers
  `Unavailable { WrongState }` instead of `BadArguments`, so "the refusal carries the published
  schema" can never be met there.
- `crates/gdtf_app/tests/net_qa/deadline.rs:12`
  (`an_admitted_run_is_answered_by_its_handler_not_the_deadline_sweep`) — the copy has no parked
  call, so there is no handler-versus-sweep race to measure. It passes with the handler deleted.

**Why:** both run on the real-socket fixture `game_app_listening()`
(`crates/gdtf_app/tests/net_qa/socket_support.rs:63-70`, reached through `exchange` at
`command_exchange.rs:34`), which rests at `AppState::Running` with no battle. They pass today only
because the game publishes one command, `app.phase`, whose availability is always `Available`
(`crates/gdtf_app/src/dev/net_qa/commands/read/app_phase.rs:41`; the one-entry list is
`commands/set.rs:7`). No command in the tree is gated on state yet, so this is a contract-audit
trap, not a live defect. A ticket that asks that suite for both an accepted `{}` case and a
`BadArguments` case while another clause says the same suite answers `Unavailable` has written two
clauses that contradict each other.

**How to apply:** for any command that is not `Available` everywhere, check its availability word
against the fixture's resting state before crediting a `BadArguments`, an "is accepted", or a
deadline clause. Require the case to run on a fixture actually in the gated state — today that is
the injected-inbox path: `crates/gdtf_app/tests/net_qa/battle_fixture.rs:27`
(`menu_app_with_net_qa`, which asserts it rests at `RunningState::Menu` at `:46-51`) plus
`request_battle` at `:55`, as used by `crates/gdtf_app/tests/net_qa/app_phase_depth.rs:55`. A
deadline case also needs that fixture driven past `DEADLINE_BUDGET`, four frames
(`crates/gdtf_net_qa_transport/src/pending/deadline.rs:1`). No socket fixture runs inside a battle
— every socket test goes through `game_app_listening()`.

Related: [[pattern-adding-a-command-breaks-three-exactly-one-assertions]],
[[pattern-deferred-act-has-no-refusal-path]].
