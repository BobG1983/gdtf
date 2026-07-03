//! The `*Requested` buffered [`Message`] types — the message-driven **input
//! contract** for the landed combat acts (E10.2 / GTW-204; the seventh,
//! [`MoveRequested`], added in GTW-234; the eighth, [`ReloadRequested`], in GTW-275; the
//! ninth, the fieldless [`EndTurnRequested`] turn signal, in GTW-309).
//!
//! Eight [`#[derive(Message)]`](bevy::prelude::Message) buffered messages — mirroring
//! [`crate::bleed::Bleeding`] / [`crate::occupancy_sync::CoverDestroyed`], the buffered
//! `Message` API, NOT the observer `Event` API (`bevy-traps.md` #4). Each carries the
//! act's [`Entity`] actor ref(s) plus the act's OWNED payload. A `Message` cannot hold a
//! borrow, so [`FireRequested`] carries an OWNED [`FireModeSpec`] (now `Copy` again,
//! GTW-260) plus the target [`Cell`] / [`Level`] — the type has **no lifetime
//! parameter**; the [`dispatch_fire`](super::fire::dispatch_fire) system reconstructs the
//! borrow-based [`FireOrder`](crate::fire::FireOrder) `{ mode: &owned_spec, target_cell,
//! target_level }` from the owned payload at the call site.
//!
//! These carry [`Entity`] actor refs (matching the landed `fire(shooter: Entity)` and
//! the downed verbs' actor/target entities) — NOT presenter-facing integer ids. A
//! `*Resolved` integer-id boundary is a LATER epic; E10 relies on component
//! change-detection for the view, so this slice ships no `*Resolved` types.

use bevy::prelude::{Deref, Entity, Message};

use crate::{
    ganger::{Direction, StanceKind},
    metric::{Cell, CellLevel, Level},
    weapon::{DamageType, FireModeSpec},
};

/// The requested **aim flag** carried by a [`SetAimingRequested`] — `true` for aimed
/// fire, `false` for hip-fired.
///
/// A no-bare-types newtype over the aim-mode `bool` payload (a domain value — the
/// requested aim mode, not framework plumbing): a private inner + a derived [`Deref`],
/// the crate's newtype house style. Distinct from the
/// [`Aiming`](crate::ganger::Aiming) *component* (the ganger's current aim state): this
/// is the *requested* value the dispatch verb sets the component to.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AimRequest(bool);

impl AimRequest {
    /// Build a requested aim flag — `true` to aim, `false` to hip-fire.
    #[must_use]
    pub const fn new(aim: bool) -> Self {
        Self(aim)
    }
}

/// A **fire** act was requested — fire `mode` at `(target_cell, target_level)` for
/// `shooter`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] shooter ref plus the act's OWNED payload: a [`FireModeSpec`] (owned by value
/// — a `Message` cannot hold a borrow; `Copy` again, GTW-260) plus the target
/// [`Cell`] / [`Level`]. The type has **no
/// lifetime parameter**; [`dispatch_fire`](super::fire::dispatch_fire) reconstructs the
/// borrow-based [`FireOrder`](crate::fire::FireOrder) `{ mode: &mode, target_cell,
/// target_level }` from this owned payload. The shooter is a Bevy [`Entity`] handle —
/// framework plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, PartialEq)]
pub struct FireRequested {
    /// The firing entity (armed shooter).
    pub shooter:      Entity,
    /// The selected fire mode's per-mode numbers — OWNED (no borrow), so the message has
    /// no lifetime; the dispatch system borrows it into a [`FireOrder`](crate::fire::FireOrder).
    pub mode:         FireModeSpec,
    /// The target cell the player aimed at (the §2 aim cell's x/y).
    pub target_cell:  Cell,
    /// The target storey the player aimed at (the aim cell's z).
    pub target_level: Level,
}

impl FireRequested {
    /// Build a fire request for `shooter` firing `mode` at `(target_cell, target_level)`.
    #[must_use]
    pub const fn new(
        shooter: Entity,
        mode: FireModeSpec,
        target_cell: Cell,
        target_level: Level,
    ) -> Self {
        Self {
            shooter,
            mode,
            target_cell,
            target_level,
        }
    }
}

/// A **set-aiming** act was requested — set `actor`'s aim flag to `aim`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`AimRequest`] flag. [`dispatch_set_aiming`](super::posture::dispatch_set_aiming)
/// calls [`set_aiming`](crate::posture::set_aiming) (which spends NO TU — toggling aim is
/// free, `posture.rs`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetAimingRequested {
    /// The acting ganger whose [`Aiming`](crate::ganger::Aiming) flag is set.
    pub actor: Entity,
    /// The requested aim value the flag is set to.
    pub aim:   AimRequest,
}

impl SetAimingRequested {
    /// Build a set-aiming request for `actor` to the requested aim flag.
    #[must_use]
    pub const fn new(actor: Entity, aim: AimRequest) -> Self {
        Self { actor, aim }
    }
}

/// A **set-stance** act was requested — change `actor`'s stance to `stance`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`StanceKind`]. [`dispatch_set_stance`](super::posture::dispatch_set_stance) calls
/// [`set_stance`](crate::posture::set_stance), which charges the
/// [`StanceChangeTu`](crate::tuning::StanceChangeTu) tuning leaf ONLY on a real change
/// (a no-op, no charge, when the actor already holds `stance`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetStanceRequested {
    /// The acting ganger whose [`Stance`](crate::ganger::Stance) is changed.
    pub actor:  Entity,
    /// The requested posture to change to.
    pub stance: StanceKind,
}

impl SetStanceRequested {
    /// Build a set-stance request for `actor` to change to `stance`.
    #[must_use]
    pub const fn new(actor: Entity, stance: StanceKind) -> Self {
        Self { actor, stance }
    }
}

/// A **set-facing** act was requested — turn `actor` to face `facing`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor ref plus the requested
/// [`Direction`]. [`dispatch_set_facing`](super::posture::dispatch_set_facing) calls
/// [`set_facing`](crate::posture::set_facing), which charges the
/// [`TurnTu`](crate::tuning::TurnTu) tuning leaf ONLY on a real turn (a no-op, no charge,
/// when the actor already faces `facing`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetFacingRequested {
    /// The acting ganger whose [`Facing`](crate::ganger::Facing) is turned.
    pub actor:  Entity,
    /// The requested direction to turn to.
    pub facing: Direction,
}

impl SetFacingRequested {
    /// Build a set-facing request for `actor` to turn to `facing`.
    #[must_use]
    pub const fn new(actor: Entity, facing: Direction) -> Self {
        Self { actor, facing }
    }
}

/// A **stabilize-downed** act was requested — `actor` stabilizes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_stabilize_downed`](super::downed::dispatch_stabilize_downed) assembles the
/// [`Actor`](crate::downed_acts::Actor) / [`DownedTarget`](crate::downed_acts::DownedTarget)
/// bundles from the queried components and calls
/// [`stabilize_downed`](crate::downed_acts::stabilize_downed), whose faction gate
/// ([`can_stabilize`](crate::downed_acts::can_stabilize)) holds end-to-end — only an
/// 8-adjacent alive ALLY sets the target's [`Stabilized`](crate::ganger::Stabilized) flag
/// (the target stays Downed).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StabilizeDownedRequested {
    /// The acting (would-be stabilizer) ganger.
    pub actor:  Entity,
    /// The downed target to stabilize.
    pub target: Entity,
}

impl StabilizeDownedRequested {
    /// Build a stabilize-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// An **execute-downed** act was requested — `actor` executes the downed `target`.
///
/// A buffered [`Message`] carrying the [`Entity`] actor + target refs.
/// [`dispatch_execute_downed`](super::downed::dispatch_execute_downed) assembles the
/// [`Actor`](crate::downed_acts::Actor) / [`DownedTarget`](crate::downed_acts::DownedTarget)
/// bundles from the queried components and calls
/// [`execute_downed`](crate::downed_acts::execute_downed), whose faction gate
/// ([`can_execute`](crate::downed_acts::can_execute)) holds end-to-end — only an
/// 8-adjacent alive ENEMY transitions the target to
/// [`LifeState::Dead`](crate::ganger::LifeState::Dead).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecuteDownedRequested {
    /// The acting (would-be executor) ganger.
    pub actor:  Entity,
    /// The downed target to execute.
    pub target: Entity,
}

impl ExecuteDownedRequested {
    /// Build an execute-downed request for `actor` over `target`.
    #[must_use]
    pub const fn new(actor: Entity, target: Entity) -> Self {
        Self { actor, target }
    }
}

/// What a [`MeleeRequested`] strike is aimed at — an opposing GANGER, or an adjacent inert
/// STRUCTURE (a Cover / Wall cell) (GTW-508, child GTW-37d of GTW-37).
///
/// A named domain enum (no-bare-types: a melee target is a domain value, not a bare
/// `Entity`-or-`CellLevel` union). The shared act-intent seam routes a structural-melee
/// target the SAME way it routes a ganger target — a melee intent carries either an
/// enemy-actor target ([`Ganger`](Self::Ganger)) or an adjacent-structure cell target
/// ([`Structure`](Self::Structure)); [`dispatch_melee`](super::melee::dispatch_melee)
/// branches on this. The [`Ganger`](Self::Ganger) arm runs the GTW-506/507 opposed-Fight
/// path (unchanged); the [`Structure`](Self::Structure) arm runs the GTW-508 UNCONTESTED
/// cover-smash (no opposed roll, no [`FightRng`](crate::rng::FightRng) draw).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeleeTarget {
    /// An opposing GANGER (the GTW-507 contested path) — the strike runs the §7
    /// opposed-Fight → §5 damage → §6 wound synthesis against this alive, 8-adjacent,
    /// in-LOS enemy. The [`Entity`] is framework plumbing (the only bare type the
    /// no-bare-types rule permits in a payload).
    Ganger(Entity),
    /// An adjacent inert STRUCTURE at this `(cell, level)` (the GTW-508 uncontested
    /// cover-smash) — a Cover or Wall cell the strike smashes with multiplied damage
    /// through the cover ledger, NO opposed roll. A [`CellLevel`] newtype, never a bare
    /// `IVec3`.
    Structure(CellLevel),
}

/// A **melee** act was requested — `attacker` strikes `target` in close combat (GTW-507,
/// child GTW-37c; the adjacent-structure target added by GTW-508, child GTW-37d).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the
/// attacking ganger [`Entity`] plus the [`MeleeTarget`] the strike is aimed at — an
/// opposing GANGER (the GTW-507 contested path) OR an adjacent inert STRUCTURE cell (the
/// GTW-508 uncontested cover-smash). Everything the ganger-vs-ganger verb reads (each
/// ganger's [`Fight`](crate::ganger::Fight), the wielded melee weapon's stats, the
/// defender's stance / armor) lives on the entities; the structural path reads the struck
/// cover's HP + armor from the [`CoverLedger`](crate::cover::CoverLedger) at the target
/// cell — so the message needs only the attacker + the target.
/// [`dispatch_melee`](super::melee::dispatch_melee) drains it, spends the wielded melee
/// weapon's primary fight-mode TU, and — per [`MeleeTarget`] — runs the §7 opposed-Fight →
/// §5 damage → §6 wound pipeline (ganger) or the uncontested §5 damage × `mult_max` →
/// `deplete_cover` cover-smash (structure). The `attacker` is a Bevy [`Entity`] handle —
/// framework plumbing, the only bare type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeRequested {
    /// The attacking (striking) ganger — its [`Fight`](crate::ganger::Fight) + wielded melee
    /// weapon resolve the blow.
    pub attacker: Entity,
    /// What the strike is aimed at — an opposing ganger or an adjacent structure cell.
    pub target:   MeleeTarget,
}

impl MeleeRequested {
    /// Build a melee request for `attacker` striking the opposing GANGER `target` (the
    /// GTW-507 contested path) — the ganger-vs-ganger form.
    #[must_use]
    pub const fn new(attacker: Entity, target: Entity) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Ganger(target),
        }
    }

    /// Build a melee request for `attacker` smashing the adjacent STRUCTURE at `at` (the
    /// GTW-508 uncontested cover-smash) — the melee-vs-structure form.
    #[must_use]
    pub const fn new_structural(attacker: Entity, at: CellLevel) -> Self {
        Self {
            attacker,
            target: MeleeTarget::Structure(at),
        }
    }
}

/// A **melee** act RESOLVED — the presenter-facing output signal that a melee strike landed at
/// `at`, carrying the weapon's `damage` type for the strike FX (GTW-507).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) emitted by
/// [`dispatch_melee`](super::melee::dispatch_melee) once per CONNECTING melee hit (a missed
/// opposed roll deals no damage and emits nothing). It mirrors the structural output signals
/// ([`crate::occupancy_sync::CoverDestroyed`] / [`ShotFired`](crate::shot_fired::ShotFired)):
/// it carries ONLY what the presenter's strike-glyph FX needs — the target cell the strike
/// landed at ([`CellLevel`]) and the wielded melee weapon's [`DamageType`] (the FX color/role
/// selector) — never combat math. The presenter reads it through a
/// [`MessageReader`](bevy::prelude::MessageReader) (the one-way sim → presenter dep; the sim
/// never reads the presenter). The HP/wound mutations themselves are applied to the target's
/// components and observed by the presenter via change-detection + the existing wound/injury
/// signals; this signal is the dedicated *strike-landed* moment.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeleeResolved {
    /// The `(cell, level)` the strike landed at — the struck target's cell (where the strike
    /// glyph draws). A [`CellLevel`] newtype, never a bare `IVec3`.
    pub at:     CellLevel,
    /// The wielded melee weapon's [`DamageType`] — the FX role/color selector (the strike
    /// glyph picks its tint/tile from this, the way [`ShotFired`](crate::shot_fired::ShotFired)
    /// does). A domain enum, never a bare label.
    pub damage: DamageType,
}

impl MeleeResolved {
    /// Build a melee-resolved signal for a connecting strike at `at` with the weapon's
    /// `damage` type.
    #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}

/// A **shove** act was requested — `shover` knocks `target` back one cell (GTW-525).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the
/// shoving ganger [`Entity`] + the shoved target [`Entity`]. The deliberate SHOVE act (any
/// ganger, adjacent to an opposing alive target) writes this from the input seam
/// (the GTW-571 per-act contextual seam: `PendingContextualIntents<ShoveAct>` ->
/// `ShoveRequested`, GTW-525 C4); the WEAPON-TAG auto-shove hooks
/// write it internally on a connecting attack (a melee strike OR a ranged shot connect,
/// GTW-525 C3). [`dispatch_shove`](super::shove::dispatch_shove) drains it, and — per the
/// [`ShoveSource`] — either gates the deliberate act (8-adjacency + opposing + alive) and
/// spends the [`ShoveTu`](crate::tuning::ShoveTu) leaf, or trusts the connect that already
/// earned the auto-shove (no gate, no TU). Both route the SHARED shove verb
/// ([`resolve_shove`](super::shove::resolve_shove)): pure one-cell displacement away from the
/// shover, with the unsupported=>fall arm through the shared GTW-523 fall path. A shove deals
/// NO wound of its own — the fall, if any, does the harm.
///
/// The `shover` / `target` are Bevy [`Entity`] handles — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShoveRequested {
    /// The shoving ganger — the target is knocked back one cell directly AWAY from it.
    pub shover: Entity,
    /// The shoved target ganger.
    pub target: Entity,
    /// Whether this is the DELIBERATE shove act (gate + TU) or a WEAPON-TAG auto-shove
    /// (already earned by a connecting attack — no gate re-check, no TU).
    pub source: ShoveSource,
}

/// What triggered a [`ShoveRequested`] (GTW-525) — the DELIBERATE shove act, or a WEAPON-TAG
/// auto-shove bundled into a connecting attack.
///
/// A named domain enum (no-bare-types: the trigger is a domain value, not a bare `bool`). The
/// single [`dispatch_shove`](super::shove::dispatch_shove) system resolves both through the
/// SAME shared shove verb, but the source decides the GATE + COST: a [`Deliberate`](Self::Deliberate)
/// shove re-checks 8-adjacency + opposing + alive and spends the [`ShoveTu`](crate::tuning::ShoveTu)
/// leaf (a fresh act); a [`Weapon`](Self::Weapon) auto-shove was already earned by the
/// connecting attack (the connect gates ran; the attack's own TU was spent), so it skips the
/// gate and the charge — it only displaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShoveSource {
    /// The DELIBERATE shove act — any ganger's context-sensitive melee shove. Gated
    /// (8-adjacency + opposing faction + alive) and TU-costed
    /// ([`ShoveTu`](crate::tuning::ShoveTu)).
    Deliberate,
    /// A WEAPON-TAG (`shove`) auto-shove — bundled into a CONNECTING melee strike OR ranged
    /// shot. Pre-earned by the connect (no gate re-check, no TU); pure displacement.
    Weapon,
}

impl ShoveRequested {
    /// Build a DELIBERATE shove request for `shover` knocking `target` back (the input-seam
    /// contextual-act form) — the gated, TU-costed act.
    #[must_use]
    pub const fn new(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Deliberate,
        }
    }

    /// Build a WEAPON-TAG auto-shove request for `shover` knocking `target` back on a
    /// connecting attack (GTW-525 C3) — the un-gated, TU-free form the melee / fire connect
    /// hooks write internally.
    #[must_use]
    pub const fn new_weapon(shover: Entity, target: Entity) -> Self {
        Self {
            shover,
            target,
            source: ShoveSource::Weapon,
        }
    }
}

/// An **open-door** act was requested — `actor` opens the adjacent closed `door` (GTW-315).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the acting
/// ganger [`Entity`] + the openable-piece (door / hatch) [`Entity`]. The player-only
/// contextual Open-Door button writes this from the input seam when the selected ganger is
/// adjacent to a CLOSED door (F4 player-only; detect offers CLOSED doors only).
/// [`dispatch_open_door`](super::open_door::dispatch_open_door) drains it and RE-GATES in the
/// sim (the input layer's offer is advisory, never authoritative): the actor exists + can
/// afford the [`OpenDoorTu`](crate::tuning::OpenDoorTu) leaf, and the `door` entity carries an
/// [`OpenState`](crate::terrain::openable::OpenState) that is CLOSED and is 8-adjacent to the
/// actor. On pass it spends the [`OpenDoorTu`](crate::tuning::OpenDoorTu) leaf off the actor and
/// writes a [`SetOpenable::toggle`](crate::terrain::openable::SetOpenable::toggle) for the door
/// — REUSING the GTW-503 open mechanism verbatim (it never flips
/// [`OpenState`](crate::terrain::openable::OpenState) directly). The door TOGGLING open then
/// clears its path + vision block through the existing GTW-501 / GTW-502 change-detection (the
/// documented one-frame settle).
///
/// The `actor` / `door` are Bevy [`Entity`] handles — framework plumbing, the only bare type
/// the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenDoorRequested {
    /// The acting ganger opening the door — the [`OpenDoorTu`](crate::tuning::OpenDoorTu) leaf
    /// is spent off its TU pool, and it is the 8-adjacency reference for the gate.
    pub actor: Entity,
    /// The openable terrain piece (door / hatch) to open — gated CLOSED + 8-adjacent, then
    /// flipped open through the shared GTW-503 [`SetOpenable`](crate::terrain::openable::SetOpenable)
    /// mechanism.
    pub door:  Entity,
}

impl OpenDoorRequested {
    /// Build an open-door request for `actor` opening `door`.
    #[must_use]
    pub const fn new(actor: Entity, door: Entity) -> Self {
        Self { actor, door }
    }
}

/// An **enter-emplacement** act was requested — `actor` mans the adjacent VACANT `emplacement`
/// (GTW-543, child GTW-41c).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the acting
/// ganger [`Entity`] + the emplacement-piece [`Entity`]. The player-only contextual Enter button
/// writes this from the input seam when the selected ganger is 8-adjacent to a VACANT emplacement.
/// [`dispatch_enter_emplacement`](super::enter_emplacement::dispatch_enter_emplacement) drains it
/// and RE-GATES in the sim (the input layer's offer is advisory, never authoritative): the actor
/// exists + can afford the [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf, and the
/// `emplacement` entity is [`EmplacementState::Vacant`](crate::terrain::emplacement::EmplacementState)
/// and 8-adjacent to the actor. On pass it spends the
/// [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf off the actor and writes a
/// [`SetEmplacement::occupy`](crate::terrain::emplacement::SetEmplacement::occupy) — REUSING the
/// GTW-543 toggle mechanism verbatim (it never flips
/// [`EmplacementState`](crate::terrain::emplacement::EmplacementState) directly). The occupy then
/// forces the occupant's cover band + spawns the mounted gun through
/// [`apply_emplacement_toggle`](crate::terrain::emplacement::apply_emplacement_toggle).
///
/// The `actor` / `emplacement` are Bevy [`Entity`] handles — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnterEmplacementRequested {
    /// The acting ganger manning the emplacement — the
    /// [`EnterEmplacementTu`](crate::tuning::EnterEmplacementTu) leaf is spent off its TU pool,
    /// and it is the 8-adjacency reference for the gate.
    pub actor:       Entity,
    /// The weapon-emplacement terrain piece to man — gated VACANT + 8-adjacent, then occupied
    /// through the shared GTW-543 [`SetEmplacement`](crate::terrain::emplacement::SetEmplacement)
    /// mechanism.
    pub emplacement: Entity,
}

impl EnterEmplacementRequested {
    /// Build an enter-emplacement request for `actor` manning `emplacement`.
    #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}

/// An **exit-emplacement** act was requested — `actor` dismounts the `emplacement` it is manning
/// (GTW-543, child GTW-41c).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the acting
/// ganger [`Entity`] + the emplacement-piece [`Entity`]. The player-only contextual Exit button
/// writes this from the input seam when the selected ganger IS the emplacement's occupant. Exit is
/// a SEPARATE TU-costed context action — there is NO force-eject (a ganger leaves the mount only
/// by spending [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu)).
/// [`dispatch_exit_emplacement`](super::enter_emplacement::dispatch_exit_emplacement) drains it and
/// RE-GATES: the `emplacement`'s
/// [`EmplacementOccupant`](crate::terrain::emplacement::EmplacementOccupant) IS the `actor` and the
/// actor can afford the [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu) leaf. On pass it
/// spends that leaf off the actor and writes a
/// [`SetEmplacement::vacate`](crate::terrain::emplacement::SetEmplacement::vacate) — the toggle
/// then restores the occupant's stance band + despawns the mounted gun.
///
/// The `actor` / `emplacement` are Bevy [`Entity`] handles — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExitEmplacementRequested {
    /// The acting ganger dismounting — the [`ExitEmplacementTu`](crate::tuning::ExitEmplacementTu)
    /// leaf is spent off its TU pool, and it must BE the emplacement's recorded occupant.
    pub actor:       Entity,
    /// The weapon-emplacement terrain piece to dismount — gated so its recorded occupant IS the
    /// actor, then vacated through the shared GTW-543
    /// [`SetEmplacement`](crate::terrain::emplacement::SetEmplacement) mechanism.
    pub emplacement: Entity,
}

impl ExitEmplacementRequested {
    /// Build an exit-emplacement request for `actor` dismounting `emplacement`.
    #[must_use]
    pub const fn new(actor: Entity, emplacement: Entity) -> Self {
        Self { actor, emplacement }
    }
}

/// A **throw-grenade** act was requested — `thrower` lobs its
/// [`TrajectoryStyle::Arc`](crate::weapon::TrajectoryStyle) grenade at the `target` cell
/// (GTW-546, child GTW-41d).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying the
/// throwing ganger [`Entity`] + the target [`CellLevel`] the grenade is lobbed at. The
/// player-only contextual Throw button writes this from the input seam (the GTW-571 per-act
/// contextual queue -> `ThrowGrenadeRequested`) when the selected ganger wields an
/// `Arc` weapon. The throw is BLIND — there is NO line-of-sight / facing / arc gate (a lob
/// need not see its target), so the target payload is a CELL AT RANGE (a [`CellLevel`], like
/// [`MoveRequested::dest`]), never an 8-adjacent entity.
/// [`dispatch_throw_grenade`](super::throw_grenade::dispatch_throw_grenade) drains it and
/// RE-GATES in the sim: the thrower exists + wields an `Arc` weapon with a loaded round + can
/// afford the [`ThrowTu`](crate::tuning::ThrowTu) leaf. On pass it spends that leaf + one
/// magazine round, marches the deterministic arc ([`march_arc`](crate::march::march_arc)) —
/// blocked by an intact roof, passing holes / windows — and fans the weapon's
/// [`HitType::Blast`](crate::weapon::HitType) at the landing through the GTW-541 resolver.
///
/// The `thrower` is a Bevy [`Entity`] handle — framework plumbing, the only bare type the
/// no-bare-types rule permits in a payload; `target` is the landed [`CellLevel`] newtype.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowGrenadeRequested {
    /// The throwing ganger — the [`ThrowTu`](crate::tuning::ThrowTu) leaf + one magazine
    /// round are spent off it, and its wielded `Arc` weapon supplies the blast stats.
    pub thrower: Entity,
    /// The target `(cell, level)` the grenade is lobbed at — the arc march's aim; the blast
    /// fans from the arc's landing (the target cell, unless an intact roof intercepts).
    pub target:  CellLevel,
}

impl ThrowGrenadeRequested {
    /// Build a throw-grenade request for `thrower` lobbing at the `target` cell.
    #[must_use]
    pub const fn new(thrower: Entity, target: CellLevel) -> Self {
        Self { thrower, target }
    }
}

/// A **throw-grenade** act RESOLVED — the presenter-facing output signal that a lobbed grenade
/// landed at `at`, carrying the weapon's `damage` type for the impact FX (GTW-546).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) emitted by
/// [`dispatch_throw_grenade`](super::throw_grenade::dispatch_throw_grenade) once per resolved
/// throw (at the arc's LANDING cell — the target cell, or the roof-block cell if an intact
/// roof intercepted). It mirrors [`MeleeResolved`] / [`ShotFired`](crate::shot_fired::ShotFired):
/// it carries ONLY what the presenter's impact / blast FX needs — the landing cell
/// ([`CellLevel`]) and the grenade's [`DamageType`] (the FX color/role selector) — never combat
/// math. The presenter reads it through a [`MessageReader`](bevy::prelude::MessageReader) (the
/// one-way sim → presenter dep). The blast's HP/wound mutations are applied to the struck
/// gangers' components and observed via change-detection; this signal is the dedicated
/// *grenade-landed* moment (the seam/app + presenter phase draws it).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrowResolved {
    /// The `(cell, level)` the grenade landed at — the arc's landing (the impact / blast FX
    /// draws here). A [`CellLevel`] newtype, never a bare `IVec3`.
    pub at:     CellLevel,
    /// The grenade's [`DamageType`] — the FX role/color selector (the impact glyph picks its
    /// tint/tile from this, the way [`MeleeResolved`] does). A domain enum, never a bare label.
    pub damage: DamageType,
}

impl ThrowResolved {
    /// Build a throw-resolved signal for a grenade landing at `at` with the weapon's `damage`
    /// type.
    #[must_use]
    pub const fn new(at: CellLevel, damage: DamageType) -> Self {
        Self { at, damage }
    }
}

/// A **move** act was requested — step `actor` one cell to `dest`.
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), carrying the
/// [`Entity`] actor ref plus the destination [`CellLevel`] (the `(cell, level)` to step
/// to). The payload is OWNED and `Copy` ([`CellLevel`] is `Copy`), so the type has **no
/// lifetime parameter** — mirroring [`SetFacingRequested`] / [`SetStanceRequested`]. The
/// actor is a Bevy [`Entity`] handle — framework plumbing, the only bare type the
/// no-bare-types rule permits in a payload; `dest` is the landed [`CellLevel`] newtype.
/// [`dispatch_move`](super::movement::dispatch_move) drains this and, per message, plans a
/// reachable affordable route and attaches a
/// [`WalkInProgress`](crate::move_acts::WalkInProgress) the landed
/// [`advance_walk`](crate::move_acts::advance_walk) walk drives, each step's TU cost being
/// the entered cell's terrain movement cost.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MoveRequested {
    /// The acting ganger to step.
    pub actor: Entity,
    /// The destination `(cell, level)` to step the actor to (one cell, no pathfinding).
    pub dest:  CellLevel,
}

impl MoveRequested {
    /// Build a move request for `actor` to step to `dest`.
    #[must_use]
    pub const fn new(actor: Entity, dest: CellLevel) -> Self {
        Self { actor, dest }
    }
}

/// A **reload** act was requested — refill `actor`'s magazine, charging the weapon's
/// per-weapon reload TU cost (GTW-275).
///
/// A buffered [`Message`] carrying ONLY the [`Entity`] actor ref — the cost (the
/// [`ReloadTu`](crate::magazine::ReloadTu)) and the target fill (the
/// [`MagazineSize`](crate::weapon::MagazineSize)) both live on the actor's own
/// [`Magazine`](crate::magazine::Magazine) grouping component, so the message needs no
/// payload (the [`SetStanceRequested`] shape, minus the requested value).
/// [`dispatch_reload`](super::reload::dispatch_reload) fetches the actor's
/// `(&mut Magazine, &mut Tu, &LifeState)`, gates on alive + affordable, then spends the
/// magazine's own `reload_tu` and refills it to full. The actor is a Bevy [`Entity`]
/// handle — framework plumbing, the only bare type the no-bare-types rule permits in a
/// payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReloadRequested {
    /// The acting ganger whose [`Magazine`](crate::magazine::Magazine) is reloaded.
    pub actor: Entity,
}

impl ReloadRequested {
    /// Build a reload request for `actor`.
    #[must_use]
    pub const fn new(actor: Entity) -> Self {
        Self { actor }
    }
}

/// An **end-turn** act was requested — the active team passes control to the other team
/// (GTW-309).
///
/// A buffered [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`) carrying **NO
/// payload**. End-turn is a GLOBAL turn signal, not a per-ganger act: which team's turn is
/// ending is tracked by the [`ActiveFaction`](crate::turn::ActiveFaction) resource, NOT a
/// message field — so this is deliberately FIELDLESS (no `{ actor: Entity }`), unlike the
/// per-ganger [`ReloadRequested`] / [`MoveRequested`]. A fieldless unit struct carries no
/// domain value, so the no-bare-types rule — which wraps *values* — does not apply; the
/// type's identity IS the signal. [`dispatch_end_turn`](crate::turn::dispatch_end_turn)
/// drains this and advances the turn cycle: it hands the turn to the other team (running
/// that team's turn-start TU regen) and STOPS there (GTW-70 removed the auto-pass). The
/// enemy turn is then driven by the GTW-70 enemy-AI brain
/// ([`enemy_ai_turn`](crate::ai::enemy_ai_turn)), which emits its OWN `EndTurnRequested` to
/// hand control back to the player.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EndTurnRequested;
