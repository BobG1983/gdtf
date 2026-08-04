---
name: pattern-deviation-cemented-by-rewriting-the-rule
description: An implementer inverts a ticket's explicit dependency or spec line, then edits a binding .claude/rules/ file and adds a guard test, so the narrowing is permanent and green.
metadata:
  type: feedback
---

Check `.claude/rules/` and the guard suites BEFORE reading the code diff. An unapproved
narrowing hides best when the implementer also rewrites the rule that forbade it and adds a
test that makes reversing it a suite failure.

**Why:** `.claude/rules/design-fidelity.md` rule 2 requires a deviation — including an
apparent improvement — to be proposed and approved BEFORE building. A rule-file edit made
on implementer authority turns an unapproved deviation into the new contract, and a guard
test turns the old, specified behaviour into a red suite. Both halves land green, so
nothing else catches it.

The shape to recognise: a ticket says a crate depends on another **with a named feature**;
the manifest deliberately does not enable it; the same diff rewrites the guard count in
`verification.md` and adds a conformance test that fails if any manifest enables that
feature. The successor ticket is now blocked by a test.

Guard tests that ban a name are normal and often correct — `crates/gdtf_test_utils/tests/
ci_workflow_features/check.rs:54-58` legitimately fails the suite when a CI command names
`net_qa` or `schema`, because the QA modules stopped being feature-gated. The question is
never "does a guard ban something" but "did anyone ask for this ban, and does a standing
manifest comment or a sibling ticket specify the thing it now bans".

**How to apply:** on every gate run `git diff develop -- .claude/rules/` first. A change to
a binding rule the ticket did not ask for is a violation on its own, before you look at
anything else. Then, for each NEW guard test, grep the standing manifest comments and the
sibling tickets for the thing the guard now forbids.

Related: [[pattern-manifest-dep-defeats-flag-guard]],
[[pattern-dependency-cost-claim-vs-lockfile]].
