//! GTW-549 (child GTW-551) — DATA-DRIVEN WEAPON ATTACHMENTS, proven on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` spawn + `apply_pending_attachments` path
//! (SUPERSEDES the GTW-542 `AttachTag`-enum model):
//!
//! - **RON round-trip** — an `AttachmentSpec` parses an `effects:` list from RON (each effect
//!   keyed by variant name with its per-item magnitude); a weapon references items BY KEY in
//!   its `attachments:` list (an omitted field defaults to an EMPTY list).
//! - **Each effect applies to the CORRECT stat on the REAL spawn** — a ganger wielding an
//!   attachment-bearing weapon has, after the post-spawn application: `Silenced` present
//!   (`Silence`), `Accuracy` RAISED (`Aim` → the HEADLINE fix, a sight boosts AIM not
//!   stability), `WeaponBraceBonus` present (`Stability` → the brace bonus), `Magazine.size`
//!   grown (`ExtraAmmo`), `Magazine.reload_tu` lowered (`ReloadTime`).
//! - **The Silenced dual-producer gate** — a `Silence` attachment yields NO `SuppressionApplied`
//!   where an identical un-silenced shot does; the shared `shooter_weapon_silenced` gate reads
//!   the wielded ranged weapon's tag.
//! - **IDENTITY** — an empty `attachments` list spawns a weapon with no attachment effects
//!   (no `Silenced`, no `WeaponBraceBonus`, un-rewritten stats).
//! - **Loader tests do NOT pin shipped magnitudes** — every assert checks presence / relative
//!   direction against a distinctive inline baseline, never a shipped number.

mod effects;
mod harness;
mod parsing;
mod silenced;
