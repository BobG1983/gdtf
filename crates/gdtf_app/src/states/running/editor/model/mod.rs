//! The in-app gang-editor's EDITABLE gang model (GTW-420).
//!
//! [`EditableGang`] is a NEW editor-side, in-memory editable model — distinct from the
//! immutable [`GangRoster`](gdtf_battle_sim::GangRoster) asset the `Load` flow builds. It
//! is the working copy the editor screen reads and mutates: a gang [`GangName`](gdtf_battle_sim::GangName) plus a list
//! of [`EditableMember`]s. It is inserted as a [`Resource`](bevy::prelude::Resource) `OnEnter(DebugEditor)` (seeded
//! from a loaded gang via the [`GangRegistry`](gdtf_battle_sim::GangRegistry), or empty when
//! none is present) and removed `OnExit(DebugEditor)`, per the project's
//! state-scoped-resource convention (`bevy-traps.md` #1) — so every system reading it guards
//! with `run_if(resource_exists::<EditableGang>)` / `Option<Res<…>>`.
//!
//! Member shape: a name plus the eight direct attributes and the weapon / armor keys. The
//! GTW-420 scaffold seeded each as a minimal DEFAULT record so the list shell could show one row
//! per member; GTW-425 adds the per-member inline editing on top — the name field, weapon / armor
//! dropdowns, and delete all mutate THIS model (the EXPANDED per-member stat table is GTW-428).
//!
//! Both types are declared through [`crate::support_item!`] (and their inherent methods too),
//! so they are `pub` under the `test-support` feature — the headless tests name them through
//! [`crate::test_support`](crate::test_support) — and `pub(crate)` in the binary build, keeping
//! it `unreachable_pub`-clean (the [`LoadedSituation`](crate::states::LoadedSituation)
//! precedent).

mod gang;
mod member;

// editor/mod.rs's test_support ledger publicly re-exports both model types through
// `model::`, so these re-exports must widen in lockstep with their `support_item!`
// definitions (a capped `pub(in ...)` re-export would hit E0365 under `test-support`).
crate::support_use!(gang::EditableGang;);
crate::support_use!(member::EditableMember;);
