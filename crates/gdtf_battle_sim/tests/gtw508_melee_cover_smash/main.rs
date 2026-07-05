//! GTW-508 (child GTW-37d of GTW-37) — the LIVE melee-vs-STRUCTURE act: a TU-costed close-combat
//! strike against an adjacent inert Cover / Wall cell, resolved as an UNCONTESTED cover-smash
//! (NO opposed Fight roll, NO `FightRng` draw) that applies multiplied weapon damage to the
//! structure's HP through the EXISTING cover ledger. Proven END-TO-END on the REAL
//! `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH a buffered
//! `MeleeRequested::new_structural` (the same message + constructor the input seam writes).
//!
//! The clause contract this covers:
//!
//! - **C5(a) — a smash reduces cover HP**: a melee strike on an adjacent Cover cell REDUCES that
//!   cell's structure HP in the real `CoverLedger`, spends the attacker's TU, and emits a
//!   `MeleeResolved` (the strike-glyph FX signal). PIN-DISCRIMINATING (fails if the structural
//!   arm is unwired).
//! - **C5(b) — repeated/sufficient smashing destroys it + the cover-destroyed signal fires**:
//!   enough melee strikes deplete the cell to zero, at which point the EXISTING `CoverDestroyed`
//!   signal is emitted (the presenter's GTW-386 rubble-burst FX reacts to it verbatim).
//! - **C5(c) — the structural path takes NO `FightRng` draw**: the outcome is IDENTICAL under two
//!   DIFFERENT battle seeds (an inert structure is not rolled against — no opposed Fight, no
//!   `FightRng`/`ShotRng`/`SeverityRng` draw), so the cover-smash is seed-independent.
//!
//! NO pinned tunable magnitudes: the tests assert HP-DECREASED / destroyed-on-lethal /
//! seed-independence — never a specific `mult_max` value or a specific weapon-damage number.
//!
//! HARNESS NOTE (the gtw507 idiom): the sim crate is the LOW crate, so it cannot dev-dep
//! `gdtf_test_utils` (a cycle). The established sim-crate battle-integration idiom drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

mod harness;
mod smash;
