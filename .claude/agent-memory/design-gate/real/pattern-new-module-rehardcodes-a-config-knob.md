---
name: pattern-new-module-rehardcodes-a-config-knob
description: A new sibling module declares a bare Duration const for a timing knob the crate's config already carries as a newtype — a bare-type violation plus a knob that can now drift.
metadata:
  type: feedback
---

When a ticket adds a NEW module beside one that already threads a typed config
(`LifecycleConfig` → `PollInterval` / `KillGrace` / `ProbeTimeout`), check every `const` the new
module declares against that config's fields.

**Why:** `bins/gdtf_qa_mcp/src/lifecycle/orphan.rs` first shipped with `wait_until_free` sleeping
a local `const FREE_POLL_STEP: Duration = Duration::from_millis(50)`, while its sibling
`manager.rs:125` sleeps `*self.config.poll_interval()`. It threaded `kill_grace` and
`probe_timeout` into `OrphanTarget` but not the poll step. Two failures at once: a bare std
`Duration` carrying domain meaning (`no-bare-types.md` rule 1 names `Duration` explicitly), and a
tuning knob living in two places that can drift.

The fix is in the tree — `FREE_POLL_STEP` is gone, `OrphanTarget` carries `poll: PollInterval`
(`orphan.rs:46`), `OrphanTarget::from_config` threads all three knobs from `LifecycleConfig`
(`orphan.rs:70`), and `wait_until_free` sleeps `*target.poll()` (`orphan.rs:182`). The house style
it should have followed from the start: `PollInterval` at `lifecycle/values.rs:136`, its default
at `lifecycle/config.rs:11`.

**How to apply:** for every new-file `const` of a primitive, `Duration`, or `String` type, grep
the crate's config and values modules for a newtype of that concept before accepting it. Test-band
consts are exempt; src-band ones are not. Note that `gdtf_qa_mcp` has no bevy dependency, so its
newtypes hand-write `impl Deref` — that part is house style, not a violation.
