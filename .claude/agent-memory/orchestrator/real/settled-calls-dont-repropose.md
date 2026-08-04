---
name: settled-calls-dont-repropose
description: Closed decisions — every agent runs opus, no macOS fast-linker swap, no mechanical style-lint enforcement; do not raise them again as if new.
metadata:
  type: feedback
---

Each of these was proposed, considered, and closed by the user. Do not reopen them.

**Every agent runs `opus`.** Sonnet was tried and rejected: more turns-to-correct made it more
expensive overall, not less. Judge a model by turns-to-correct, not by per-token price. All six
agent definitions carry `model: opus` (`.claude/agents/*.md`), and every `agent()` call in
`.claude/workflows/build-ticket.js` passes `model: 'opus'` — there is no other model anywhere in the
kit. Never pin `fable`: access is exhausted and the call dies outright.

**No macOS fast-linker swap.** Apple's `ld-prime` is already fast. `.cargo/config.toml` sets a
`linker` only for `x86_64-pc-windows-msvc` (`rust-lld.exe`); the Apple targets carry
`-Zshare-generics=y -Zthreads=0` and no linker override, which is correct as-is. Slow test cycles
here come from the number of test binaries, not from link speed.

**No mechanical style-lint enforcement.** A `syn`-based checker for the no-bare-types rule was built
and then retired by the user after its false-positive maintenance cost outweighed what it caught.
`.claude/rules/no-bare-types.md`, "Enforcement", says the rule is enforced by the `design-gate`
review during `/gate` and by no mechanical test, and that rebuilding mechanical enforcement needs
new evidence first — a one-crate pilot measuring false-positive rate and caught-bug rate.

**Why:** re-proposing a closed call costs the user the same argument twice and reads as not having
listened. Raise a concern once, then build what was asked.

Related: [[dont-retune-what-the-user-tuned]], [[egui-editor-bevyui-game-never-cross]].
