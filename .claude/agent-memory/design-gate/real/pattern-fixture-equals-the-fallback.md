---
name: pattern-fixture-equals-the-fallback
description: A test whose fixture value equals the code's own fallback cannot tell "used the argument" from "used the default" — the assertion passes even if the argument is ignored.
metadata:
  type: feedback
---

When a clause says "the caller-supplied X is used instead of the ambient default X", check
what VALUE the test supplies. If the fixture IS the ambient default, the assertion is
tautological: a mutation that ignores the argument entirely still passes.

**Why:** `bins/gdtf_qa_mcp/src/lifecycle/launch/spec.rs:77-81`, `resolved_working_dir`, falls
back to `std::env::current_dir()` when no working dir was given. A test that sets
`working_dir` to `current_dir()` therefore proves nothing about the field the whole change
exists for. The companion mistake is asserting only `text.contains("working_dir")` — the KEY
name, not the value.

**How to apply:** demand a fixture provably different from the fallback, and an assertion on
the VALUE. That is the shape in the tree now:
`bins/gdtf_qa_mcp/src/mcp/control/test/launch.rs:49`,
`handle_launch_passes_the_recipe_through_and_reports_it`, takes its directory from
`a_directory_that_is_not_the_hosts()` (`:50`, defined at
`bins/gdtf_qa_mcp/src/mcp/control/test/support.rs:86`), asserts the recipe carries that exact
path (`:73-76`), and asserts the rendered reply contains it (`:86`). Also check the sibling
argument-parser and render tests for the same fixture reuse — `test/render.rs:16` and `:47`
take the same helper.

Related: [[pattern-tautological-assertion-replacing-a-pin]],
[[pattern-canned-fixture-carries-the-unasserted-value]].
