//! GTW-525 C3 — the `shove` WEAPON-TAG auto-shove, proven END-TO-END on the REAL
//! `setup_battle_on_request` -> `BattleSimPlugin` `Simulate`-band path (the gtw507 idiom):
//!
//! - **QA(6)** a `shove`-tagged MELEE weapon KNOCKS the target back one cell on a CONNECTING
//!   strike (in addition to the damage).
//! - **QA(7)** a `shove`-tagged RANGED weapon KNOCKS the target back on a CONNECTING shot.
//! - **QA(8a)** a NON-`shove` MELEE weapon never shoves (the same connecting strike leaves the
//!   target's cell UNCHANGED).
//! - **QA(8b)** a MISSED melee strike (the opposed roll lost) never shoves — even with a
//!   `shove`-tagged weapon.
//! - **QA(8c)** a NON-`shove` RANGED weapon never shoves on a CONNECTING shot — the negative
//!   discriminator for the fire-site tag-read (`weapon_shoves`): the shot connects (the same
//!   point-blank geometry as QA(7)) but the untagged gun leaves the target's cell UNCHANGED.
//! - **QA(8d)** a MISSED shot (the round strikes an empty lane, never a ganger) never shoves —
//!   even with a `shove`-tagged gun: the negative discriminator for the fire-site connect-read
//!   (`struck_ganger`).
//!
//! The auto-shove is bundled into the attack (no extra input): the connecting melee strike
//! (`dispatch_melee`) / ranged shot (`dispatch_fire`) writes an internal `ShoveRequested`
//! (`ShoveSource::Weapon`) that `dispatch_shove` (ordered `.after` both) resolves the SAME
//! frame through the SHARED shove verb. No pinned tunable magnitude — the assert is the
//! knock-back RELATION (the target's cell moved / did not move), never a balance number.
//!
//! QA(8a)/(8b) discriminate the MELEE connect-hook's tag-read + miss-gate; QA(8c)/(8d)
//! discriminate the SEPARATE RANGED connect-hook — `dispatch_fire`'s `weapon_shoves` tag-guard
//! and its `struck_ganger` connect-gate. Each fire-site clause has its own negative, so
//! dropping either half of the `(weapon_shoves, struck_ganger)` guard fails a test.

mod harness;
mod melee_tag;
mod ranged_tag;
