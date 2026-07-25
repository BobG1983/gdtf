//! The §7 melee STRIKE resolution verb — the pure, render-free function that sequences
//! the connecting-hit synthesis for a live melee act (GTW-507, child GTW-37c of GTW-37;
//! routed through the shared wound core by GTW-821).
//!
//! `docs/combat/resolution.md` §7 designs a melee attack as an **opposed roll** whose
//! relative margin scales the blow as a multiplier; a connecting hit then runs the normal
//! §5 damage → §6 wound steps, and — like every other wound — draws its named injury
//! (`docs/combat/wounds-and-roster.md`). This verb COMPOSES the already-landed combat-math
//! pieces in that exact order — it REIMPLEMENTS NONE of them:
//!
//! 1. [`roll_body_part`](crate::hit_location::roll_body_part) — the §4 weighted part roll
//!    (the defender's struck location), drawn from the injected [`ShotRng`](crate::rng::ShotRng).
//! 2. [`opposed_fight`](crate::melee::opposed_fight) — the §7 opposed-Fight (GTW-506), two
//!    [`FightRng`](crate::rng::FightRng) draws → a [`FightOutcome`](crate::melee::FightOutcome).
//! 3. on a **miss** (`!connect`) — NO damage, NO further draw: the verb returns a no-effect
//!    [`MeleeStrike`] (the §7 "connect if atk > def" gate).
//! 4. on a **connect** — the struck worn piece (or bare flesh) is resolved and
//!    [`melee_damage_mult`](crate::melee::melee_damage_mult) (GTW-506) turns the §7 margin
//!    into the blow's [`MeleeDamageMult`](crate::melee::MeleeDamageMult).
//! 5. `synthesize_wound` — the ONE shared
//!    §5 → §6 → §8 fold the ranged fire path and the fall path also run: `resolve_hit` (§5)
//!    → [`apply_melee_multiplier`](crate::melee::apply_melee_multiplier) over the RESOLVED
//!    hit (§7, the post-armor placement GTW-506/507 chose) → `roll_severity` (§6, one
//!    [`SeverityRng`](crate::rng::SeverityRng) draw) → `apply_hit` (§6) → `roll_injury` (§8,
//!    one [`InjuryRng`](crate::rng::InjuryRng) draw on a non-graze / non-fatal wound).
//!
//! The §8 injury draw samples the **melee** per-source weighting tables — the strike builds a
//! blow carrying [`DamageContext::Melee`](crate::injuries::DamageContext::Melee) (GTW-452's
//! `weighting/<category>.melee.weighting.ron` content), and the rolled injury rides out on the
//! [`MeleeStrike`] verdict for [`dispatch_melee`](crate::acts::dispatch_melee) to bridge into
//! the EXISTING [`InjuryInflicted`](crate::acts::InjuryInflicted) message, exactly as the fire
//! and fall paths bridge theirs.
//!
//! Pure model logic: no systems, no `&mut World`, no ECS trigger, no pixel. The owning
//! [`dispatch_melee`](crate::acts::dispatch_melee) system (GTW-507) assembles the borrow-views
//! from queried components, gates 8-adjacency + LOS + alive + opposing faction, spends the
//! weapon's fight-mode TU, and emits the presenter signal — this verb is the combat core it
//! calls once the gates pass.
//!
//! ## Module layout
//!
//! Wiring-only `mod.rs`; every concern lives in a focused submodule:
//!
//! - `inputs` — what the caller assembles ([`MeleeWeaponHit`] / [`Combatants`] /
//!   [`MeleeStrikeEnv`]);
//! - `verdict` — what the caller reads back ([`MeleeStrike`]);
//! - `resolve` — the sequence itself ([`resolve_melee_strike`] + its armor-input resolution).

mod inputs;
mod resolve;
mod verdict;

pub use inputs::{Combatants, MeleeStrikeEnv, MeleeWeaponHit};
pub use resolve::resolve_melee_strike;
pub use verdict::MeleeStrike;
