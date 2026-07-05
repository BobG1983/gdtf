//! GTW-543 (child GTW-41c of GTW-41) — the LIVE weapon-emplacement act: a ganger ENTERS an
//! adjacent VACANT emplacement (a TU-costed context act), which mans the mount (state Occupied,
//! occupant band forced HIGH, the bolted-down mounted gun spawned + wielded), FIRES that mounted
//! gun through the real fire path (the ranged read PREFERS the `MountedWeapon` while occupied, the
//! `EmplacementStability` seam steadies it), then EXITS (a separate TU-costed act: band restored, the
//! mounted-weapon edge despawned, the occupant's own gun resolves again). Proven END-TO-END on the
//! REAL `setup_battle_on_request` → `BattleSimPlugin` `Simulate`-band path, driven THROUGH the same
//! buffered `EnterEmplacementRequested` / `ExitEmplacementRequested` / `FireRequested` messages the
//! input seam writes.
//!
//! Clause contract:
//!
//! - **ENTER**: an 8-adjacent actor with enough TU mans a VACANT emplacement — state flips to
//!   Occupied, the actor's TU drops by the `enter_emplacement_tu` leaf, the occupant band is forced
//!   HIGH, and the mounted gun (a `MountedWeapon`-marked entity resolved from the emplacement's
//!   `MountedWeaponKey`) is spawned + wielded by the occupant.
//! - **FIRE**: while occupied, the occupant's fire resolves the MOUNTED weapon (the `ShotFired`
//!   carries the MOUNTED gun's `DamageType`, distinct from the ganger's own gun), through the real
//!   `dispatch_fire` → `fire()` path, seeded-deterministically (two same-seed runs agree).
//! - **EXIT**: the occupant dismounts (a separate TU-costed act) — state back to Vacant, the mount
//!   edge despawned (the occupant no longer wields a `MountedWeapon`), the band restored from
//!   stance, and TU dropped by the `exit_emplacement_tu` leaf.
//! - **Gate rejections**: an unaffordable-TU enter, a non-adjacent enter, and an already-occupied
//!   enter (no force-eject) each leave the state untouched and spend no TU.
//!
//! NO pinned tunable magnitudes: the tests assert TU-DROPPED-BY-THE-LEAF (read from the tuning
//! resource) / state-flipped / band-forced / weapon-resolved — never a specific TU number or
//! damage figure.
//!
//! HARNESS NOTE (the gtw508 idiom): the sim crate is the LOW crate, so it drives
//! `setup_battle_on_request` via a `SetupBattleRequested` message against a `MinimalPlugins` +
//! `AssetPlugin` + `ScenePlugin` + `BattleSimPlugin` app — the EXACT production wiring.

mod enter_exit;
mod harness;
mod mounted_fire;
mod rejections;
