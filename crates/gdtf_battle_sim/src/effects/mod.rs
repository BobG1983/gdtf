//! **Authorable effects** (GTW-558, child of GTW-551 → GTW-17) — the families of
//! conceptually-isolated, data-authored EFFECTS the sim can apply to an entity, plus the
//! consequence-TICKING runtimes that drive them (GTW-638 re-homed the per-round clocks —
//! [`bleed`], [`dot`], and the fields / on-death mechanics — here from the dissolved
//! `acts_runtime`, per the palette-and-mechanics-in-one-home ruling).
//!
//! Each palette family holds a closed serde **vocabulary** (the on-disk RON
//! enum), a shared **apply trait** whose one method IS an effect's behaviour, and ONE
//! self-contained file per effect (its magnitude, its isolated `ApplyX` type, its `impl`,
//! its unit test). Where a family's invoking mechanics are themselves an effect runtime
//! (fields, on-death), they live in the SAME sub-module; mechanics owned by another
//! concern stay there (e.g. weapon attachments' registry / commands extension /
//! spawn-applier under [`equipment::attachments`](crate::equipment::attachments)). The
//! dependency still runs one way — mechanics depend on the palette, never the reverse.
//!
//! ## The discipline
//!
//! Adding a new effect to a family = ONE new per-effect file + ONE enum variant + ONE
//! delegation arm (in the family's `effect.rs`) + ONE `mod` line (in the family's `mod.rs`).
//! No central logic `match`, no folder-fn, no authoring step scattered across the codebase.
//! The mechanics NEVER match on the effect enum — they invoke the shared apply trait
//! generically.

/// The **weapon-attachment effect palette** — the closed
/// [`AttachmentEffect`](crate::effects::attachments::AttachmentEffect) vocabulary + one
/// isolated `ApplyX` behaviour per effect (GTW-558; the palette that SUPERSEDES the GTW-542
/// `AttachTag` model). The mechanics that resolve + apply it live under
/// [`equipment::attachments`](crate::equipment::attachments).
pub mod attachments;

/// The **§9 bleed-out clock** — [`tick_bleed`](crate::effects::bleed::tick_bleed) (the
/// per-round [`Wounds`](crate::ganger::Wounds) drain of every
/// [`BleedingOut`](crate::effects::bleed::BleedingOut) Downed
/// ganger + its once-only terminal gate), the
/// [`Bleeding`](crate::effects::bleed::Bleeding) signal, and the
/// [`enemy_phase_started`](crate::effects::bleed::enemy_phase_started) cadence gate
/// (E3.7 / GTW-189; re-homed from the dissolved `acts_runtime`, GTW-638).
pub mod bleed;

/// The **damage-over-time runtime** — the per-turn
/// [`tick_dot`](crate::effects::dot::tick_dot) drain, the
/// [`DotApplied`](crate::effects::dot::DotApplied) boundary message, and the
/// [`apply_dot`](crate::effects::dot::apply_dot) attach/refresh system (GTW-544, child
/// GTW-41e; re-homed from the dissolved `acts_runtime`, GTW-638).
pub mod dot;

/// The **area-damage fields** family — the closed
/// [`FieldEffect`](crate::effects::fields::FieldEffect) vocabulary + one isolated
/// behaviour per consequence of standing in (or placing) an area-damage field (GTW-553),
/// AND the mechanics that invoke it — the per-round clock, the placement registry +
/// lifetime, and the catalog model (GTW-545; merged into this one home by GTW-638).
pub mod fields;

/// The **injury effect palette** — the closed
/// [`InjuryEffect`](crate::effects::injuries::InjuryEffect) vocabulary + one isolated
/// behaviour per effect (GTW-550; the palette that replaces the central ledger match).
/// The ledger STORAGE + roll/apply mechanics that invoke it live under
/// [`damage_resolution::injuries`](crate::damage_resolution::injuries) and
/// [`apply_injury`](crate::acts::apply_injury).
pub mod injuries;

/// The **on-death effects** family — the closed
/// [`OnDeathEffect`](crate::effects::on_death::OnDeathEffect) vocabulary + one isolated
/// behaviour per effect (GTW-552), AND the mechanics that invoke it — the cascade
/// work-queue resolver, the terminal-gate
/// [`OnDeathOccurred`](crate::effects::on_death::OnDeathOccurred) emission, and the
/// [`OnDeath`](crate::effects::on_death::OnDeath) / cover-registry authoring carriers
/// (GTW-547; merged into this one home by GTW-638).
pub mod on_death;
