//! One def per row `view_key_for` selects, and the key each row must return.

use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    def::{
        LeavesBehind, TerrainDef, TerrainDisplayName, TerrainPresenterKind, TerrainSimKind,
        TerrainTag, TerrainUuid, TerrainView, TerrainViewArt, TerrainViews,
    },
    openable::OpenState,
    piece::TerrainGraphicKey,
    slab::SlabHp,
    terrain::facing::{TerrainCorner, TerrainFacing},
    weapon::WeaponName,
};

use super::{LinkEnd, view_key_for};

fn key(name: &str) -> TerrainGraphicKey {
    TerrainGraphicKey::new(name.to_owned())
}

fn row(view: TerrainView, sprite: &str) -> TerrainViewArt {
    TerrainViewArt {
        view,
        sprite: key(sprite),
    }
}

fn def_with(
    sim_kind: TerrainSimKind,
    tags: Vec<TerrainTag>,
    rows: Vec<TerrainViewArt>,
) -> TerrainDef {
    TerrainDef {
        key: TerrainUuid::generate(),
        display_name: TerrainDisplayName::new("Row Fixture".to_owned()),
        presenter_kind: TerrainPresenterKind::Wall,
        sim_kind,
        tags,
        views: TerrainViews::new(rows),
        on_death: Vec::new(),
        blocks_pathing: None,
        blocks_los: None,
        leaves_behind: LeavesBehind::Nothing,
    }
}

fn wall_kind() -> TerrainSimKind {
    TerrainSimKind::Wall {
        hp:               CoverHp::new(10),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
        height_band:      HeightBand::High,
    }
}

fn cover_kind() -> TerrainSimKind {
    TerrainSimKind::Cover {
        hp:               CoverHp::new(10),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
        height_band:      HeightBand::Low,
    }
}

fn emplacement_kind() -> TerrainSimKind {
    TerrainSimKind::Emplacement {
        hp:               CoverHp::new(10),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
        height_band:      HeightBand::High,
        mounted_weapon:   WeaponName::new("mount".to_owned()),
        entry_sides:      Vec::new(),
    }
}

fn slab_kind() -> TerrainSimKind {
    TerrainSimKind::Slab {
        hp:               SlabHp::new(10),
        armor_protection: ArmorProtection::new(0),
        armor_hardness:   ArmorHardness::new(0),
    }
}

#[test]
fn an_openable_def_picks_shut_or_open_on_its_facing() {
    let door = def_with(
        wall_kind(),
        vec![TerrainTag::Openable],
        vec![
            row(TerrainView::Shut(TerrainFacing::East), "shut_east"),
            row(TerrainView::Open(TerrainFacing::East), "open_east"),
            row(TerrainView::Edge(TerrainFacing::East), "edge_east"),
        ],
    );

    assert_eq!(
        view_key_for(&door, TerrainFacing::East, Some(OpenState::Open), None),
        Some(&key("open_east")),
        "an Openable def standing open draws its Open row for its own facing",
    );
    assert_eq!(
        view_key_for(&door, TerrainFacing::East, Some(OpenState::Closed), None),
        Some(&key("shut_east")),
        "an Openable def standing shut draws its Shut row",
    );
    assert_eq!(
        view_key_for(&door, TerrainFacing::East, None, None),
        Some(&key("shut_east")),
        "an Openable def with no OpenState draws its Shut row, never the wall Edge row its \
         `sim_kind: Wall` would give a kind-first read",
    );
}

#[test]
fn a_stair_def_picks_its_end_of_the_link_and_from_below_as_a_tile() {
    let stair = def_with(
        slab_kind(),
        vec![TerrainTag::Stair],
        vec![
            row(TerrainView::FromBelow(TerrainFacing::North), "up_north"),
            row(TerrainView::FromAbove(TerrainFacing::North), "down_north"),
            row(TerrainView::Single, "one_floor"),
        ],
    );

    assert_eq!(
        view_key_for(&stair, TerrainFacing::North, None, Some(LinkEnd::Lower)),
        Some(&key("up_north")),
        "a Stair def on the link's lower end draws its FromBelow row",
    );
    assert_eq!(
        view_key_for(&stair, TerrainFacing::North, None, Some(LinkEnd::Upper)),
        Some(&key("down_north")),
        "a Stair def on the link's upper end draws its FromAbove row",
    );
    assert_eq!(
        view_key_for(&stair, TerrainFacing::North, None, None),
        Some(&key("up_north")),
        "a Stair def resolved as a tile rather than a link endpoint draws its FromBelow row, \
         never the `Single` row its `sim_kind: Slab` would give a kind-first read",
    );
}

#[test]
fn a_plain_wall_picks_its_edge_and_never_a_corner() {
    let corner_rows = TerrainCorner::ALL
        .into_iter()
        .map(|corner| row(TerrainView::Corner(corner), "corner"));
    let rows = TerrainFacing::ALL
        .into_iter()
        .map(|facing| row(TerrainView::Edge(facing), edge_name(facing)))
        .chain(corner_rows)
        .collect();
    let wall = def_with(wall_kind(), Vec::new(), rows);

    for facing in TerrainFacing::ALL {
        assert_eq!(
            view_key_for(&wall, facing, None, None),
            Some(&key(edge_name(facing))),
            "a plain Wall def draws the Edge row for the facing it carries ({facing:?})",
        );
        assert_ne!(
            view_key_for(&wall, facing, None, None),
            Some(&key("corner")),
            "no facing may select a Corner row: choosing a corner needs a neighbour scan this \
             resolver does not do ({facing:?})",
        );
    }
}

const fn edge_name(facing: TerrainFacing) -> &'static str {
    match facing {
        TerrainFacing::North => "edge_north",
        TerrainFacing::East => "edge_east",
        TerrainFacing::South => "edge_south",
        TerrainFacing::West => "edge_west",
    }
}

#[test]
fn cover_and_an_emplacement_pick_the_facing_row() {
    for sim_kind in [cover_kind(), emplacement_kind()] {
        let def = def_with(
            sim_kind,
            Vec::new(),
            vec![
                row(TerrainView::Facing(TerrainFacing::South), "faced_south"),
                row(TerrainView::Facing(TerrainFacing::West), "faced_west"),
            ],
        );
        assert_eq!(
            view_key_for(&def, TerrainFacing::South, None, None),
            Some(&key("faced_south")),
            "a Cover or Emplacement def draws the Facing row for the facing it carries",
        );
        assert_eq!(
            view_key_for(&def, TerrainFacing::West, None, None),
            Some(&key("faced_west")),
            "turning the piece selects the other Facing row",
        );
    }
}

#[test]
fn a_plain_slab_picks_its_one_view_at_every_facing() {
    let slab = def_with(
        slab_kind(),
        Vec::new(),
        vec![row(TerrainView::Single, "one_floor")],
    );
    for facing in TerrainFacing::ALL {
        assert_eq!(
            view_key_for(&slab, facing, None, None),
            Some(&key("one_floor")),
            "a plain Slab def draws its one Single row whichever way it is turned ({facing:?})",
        );
    }
}

#[test]
fn a_def_short_of_the_selected_row_answers_nothing() {
    let wall = def_with(
        wall_kind(),
        Vec::new(),
        vec![row(TerrainView::Edge(TerrainFacing::North), "edge_north")],
    );
    assert_eq!(
        view_key_for(&wall, TerrainFacing::South, None, None),
        None,
        "a def that authors no row for the selected view answers nothing, so the caller leaves \
         the tile on the stamp the static draw gave it",
    );
}
