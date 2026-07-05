//! [`PlacedGanger`] + [`Placement`] — the GTW-414 schema-v2 placement half: WHICH
//! roster member fights, WHERE, and on WHICH side.

use serde::Deserialize;

use crate::{
    ganger::{Aiming, Facing, Faction, GangName, GangerName, LifeState, Stance},
    metric::CellLevel,
};

/// One **placed ganger** — the GTW-414 v2 `Situation`-side ganger reference: WHICH
/// roster member fights, WHERE, and on WHICH side.
///
/// The placement half of the schema-v2 split (GTW-414). It carries NO identity /
/// attributes / equipment of its own — those come from the gang roster: it references
/// its [`gang`](PlacedGanger::gang) ([`GangName`], a [`GangRegistry`](crate::ganger::GangRegistry)
/// key) and the [`member`](PlacedGanger::member) within it (by [`GangerName`]), and
/// [`setup_battle`](crate::situation::setup_battle) resolves `(gang, member)` to a
/// [`GangMember`](crate::ganger::GangMember) (eight attributes + weapon + armor keys). The
/// `Situation` supplies the rest: the `(cell, level)` placement
/// ([`at`](PlacedGanger::at)), the posture / facing / aim / life fields, and the
/// [`faction`](PlacedGanger::faction) the ganger fights for in THIS battle — so the same
/// faction-agnostic gang roster can be fielded on any side at any position.
///
/// `PartialEq` + `Eq` (every field — the gang/member names, the `(cell, level)`, and the
/// fieldless posture enums — has a total `Eq`, unlike the f32-bearing roster
/// [`GangMember`](crate::ganger::GangMember)). Not `Hash`: `(cell, level)`-keyed
/// de-duplication ([`has_stacked_gangers`](crate::situation::has_stacked_gangers)) hashes
/// [`at`](PlacedGanger::at), never the whole struct. `Clone` (owned `String`-backed
/// names). Derives [`Deserialize`] so an authored situation `.ron` names
/// each placement as a self-describing record (the value graph all flows through the
/// landed newtype/enum serde derives — render-free, pixel-free).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlacedGanger {
    /// The **gang** this ganger's roster comes from — a [`GangName`] resolved against the
    /// [`GangRegistry`](crate::ganger::GangRegistry) at
    /// [`setup_battle`](crate::situation::setup_battle). A gang absent from the registry
    /// is a handled
    /// [`BattleSetupError::GangNotFound`](crate::situation::BattleSetupError::GangNotFound)
    /// error (no panic). Authored as a bare string ([`GangName`] is `#[serde(transparent)]`).
    pub gang:       GangName,
    /// The **member** within the gang this ganger IS — a [`GangerName`] resolved against
    /// the gang's roster at setup (`gang.member(member)`). A member absent from the gang
    /// is a handled
    /// [`BattleSetupError::GangMemberNotFound`](crate::situation::BattleSetupError::GangMemberNotFound)
    /// error (no panic). Authored as a bare string ([`GangerName`] is `#[serde(transparent)]`).
    pub member:     GangerName,
    /// The `(cell, level)` the ganger spawns at — its [`Position`](crate::ganger::Position).
    pub at:         CellLevel,
    /// The ganger's gang (faction) identity for THIS battle — the side assignment the
    /// situation makes (NOT part of the roster; the roster is faction-agnostic).
    pub faction:    Faction,
    /// The ganger's facing.
    pub facing:     Facing,
    /// The ganger's stance (posture).
    pub stance:     Stance,
    /// The ganger's aim-mode flag.
    pub aiming:     Aiming,
    /// The ganger's terminal life state.
    pub life_state: LifeState,
}

impl PlacedGanger {
    /// Build a placed ganger from its gang/member refs and a [`Placement`] — the public
    /// constructor (house style) so the setup, the test builders, and the presenter can
    /// build one without reaching the fields piecemeal.
    ///
    /// The six situation-supplied placement fields (`at` / `faction` / `facing` / `stance`
    /// / `aiming` / `life_state`) are grouped into the named [`Placement`] argument (a real
    /// named type per no-bare-types, NOT a tuple), keeping the constructor's parameter list
    /// under clippy's argument-count gate while the struct itself stays flat for serde.
    #[must_use]
    pub const fn new(gang: GangName, member: GangerName, placement: Placement) -> Self {
        Self {
            gang,
            member,
            at: placement.at,
            faction: placement.faction,
            facing: placement.facing,
            stance: placement.stance,
            aiming: placement.aiming,
            life_state: placement.life_state,
        }
    }
}

/// The six **situation-supplied placement fields** a [`PlacedGanger`] carries — grouped
/// into one named type so [`PlacedGanger::new`] takes a single placement argument instead
/// of six positional ones (no-bare-types: a real named grouping struct, never a tuple).
///
/// This is the WHERE + WHICH-SIDE half of the GTW-414 schema-v2 split: the cell/level the
/// ganger spawns at, the [`faction`](Placement::faction) it fights for THIS battle, and its
/// posture / facing / aim / life fields. The roster half (identity + attributes + weapon /
/// armor) comes from the gang [`GangMember`](crate::ganger::GangMember), not here. A pure
/// in-code constructor helper — it is NOT itself authored: the situation `.ron` authors
/// these as flat fields on the [`PlacedGanger`] record, and the test builders /
/// [`GangerSpawn::split`](crate::situation::GangerSpawn::split) assemble a `Placement` to call the constructor.
///
/// All fields are `Copy`, so the struct is `Copy` (a cheap by-value placement bundle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    /// The `(cell, level)` the ganger spawns at — its [`Position`](crate::ganger::Position).
    pub at:         CellLevel,
    /// The faction (side) the ganger fights for in THIS battle (placement, not roster).
    pub faction:    Faction,
    /// The ganger's facing.
    pub facing:     Facing,
    /// The ganger's stance (posture).
    pub stance:     Stance,
    /// The ganger's aim-mode flag.
    pub aiming:     Aiming,
    /// The ganger's terminal life state.
    pub life_state: LifeState,
}

impl Placement {
    /// Build a placement bundle from its six situation-supplied fields — the shape
    /// [`GangerSpawn::split`](crate::situation::GangerSpawn::split) and the test builders assemble to call
    /// [`PlacedGanger::new`].
    #[must_use]
    pub const fn new(
        at: CellLevel,
        faction: Faction,
        facing: Facing,
        stance: Stance,
        aiming: Aiming,
        life_state: LifeState,
    ) -> Self {
        Self {
            at,
            faction,
            facing,
            stance,
            aiming,
            life_state,
        }
    }
}
