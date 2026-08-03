use gdtf_battle_sim::prelude::Position;

pub type CellOrderKey = (i32, i32, i32);

#[must_use]
pub fn cell_order_key(position: &Position) -> CellOrderKey {
    (position.z, position.y, position.x)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CycleDirection {
            Next,
            Prev,
}

#[must_use]
pub fn cycle_player_selection(
    ordered: &[bevy::prelude::Entity],
    current: Option<bevy::prelude::Entity>,
    direction: CycleDirection,
) -> Option<bevy::prelude::Entity> {
    let count = ordered.len();
    if count == 0 {
        return None;
    }
    let current_index = current.and_then(|entity| ordered.iter().position(|e| *e == entity));
    let next_index = match (current_index, direction) {
        (None, CycleDirection::Next) => 0,
        (None, CycleDirection::Prev) => count - 1,
        (Some(index), CycleDirection::Next) => (index + 1) % count,
        (Some(index), CycleDirection::Prev) => (index + count - 1) % count,
    };
    ordered.get(next_index).copied()
}

#[cfg(test)]
mod test {
    use bevy::prelude::{Entity, World};

    use super::{CycleDirection, cycle_player_selection};

                            fn distinct_entities(n: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..n).map(|_| world.spawn_empty().id()).collect()
    }

            fn ordered() -> [Entity; 3] {
        let ids = distinct_entities(3);
        [ids[0], ids[1], ids[2]]
    }

        #[test]
    fn next_advances_and_wraps() {
        let gang = ordered();
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[0]), CycleDirection::Next),
            Some(gang[1]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[1]), CycleDirection::Next),
            Some(gang[2]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[2]), CycleDirection::Next),
            Some(gang[0]),
            "Next wraps the last to the first",
        );
    }

        #[test]
    fn prev_advances_and_wraps() {
        let gang = ordered();
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[0]), CycleDirection::Prev),
            Some(gang[2]),
            "Prev wraps the first to the last",
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[2]), CycleDirection::Prev),
            Some(gang[1]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(gang[1]), CycleDirection::Prev),
            Some(gang[0]),
        );
    }

        #[test]
    fn from_none_picks_first_or_last() {
        let gang = ordered();
        assert_eq!(
            cycle_player_selection(&gang, None, CycleDirection::Next),
            Some(gang[0]),
            "Next from None -> first",
        );
        assert_eq!(
            cycle_player_selection(&gang, None, CycleDirection::Prev),
            Some(gang[2]),
            "Prev from None -> last",
        );
    }

            #[test]
    fn non_member_current_falls_back_to_end() {
        let ids = distinct_entities(4);
        let gang = [ids[0], ids[1], ids[2]];
        let stranger = ids[3];
        assert_eq!(
            cycle_player_selection(&gang, Some(stranger), CycleDirection::Next),
            Some(gang[0]),
        );
        assert_eq!(
            cycle_player_selection(&gang, Some(stranger), CycleDirection::Prev),
            Some(gang[2]),
        );
    }

        #[test]
    fn empty_gang_is_none() {
        assert_eq!(
            cycle_player_selection(&[], None, CycleDirection::Next),
            None,
        );
        assert_eq!(
            cycle_player_selection(&[], Some(Entity::PLACEHOLDER), CycleDirection::Prev),
            None,
        );
    }
}
