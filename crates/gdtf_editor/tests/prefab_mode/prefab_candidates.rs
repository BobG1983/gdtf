use bevy::asset::uuid::Uuid;
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry, PrefabSpec,
    SpawnRole, ThemeUuid,
};
use gdtf_editor::prefab_candidates;

const THEME: ThemeUuid = ThemeUuid::new(Uuid::from_u128(0x0132_2100_0001));

// One name two prefabs share, so the sort key has to reach past its first component.
const SHARED_NAME: &str = "deployment";

// Five names, listed sorted, so the fixture can insert them backwards.
const SORTED_NAMES: [&str; 5] = ["alpha", "bravo", "charlie", "delta", "echo"];

fn size(span: u8) -> GridSize {
    GridSize::new(
        GridWidth::new(span),
        GridHeight::new(span),
        GridLevels::new(1),
    )
    .unwrap_or_else(|_| GridSize::default())
}

fn spec(span: u8, role: SpawnRole) -> PrefabSpec {
    PrefabSpec::new(THEME, size(span), role, Vec::new())
}

fn named(name: &str, spec: PrefabSpec) -> Prefab {
    Prefab::new(PrefabName::new(name.to_owned()), spec)
}

fn registry(prefabs: impl IntoIterator<Item = Prefab>) -> PrefabRegistry {
    let mut registry = PrefabRegistry::default();
    for prefab in prefabs {
        registry.insert(prefab);
    }
    registry
}

#[test]
fn one_key_holding_five_prefabs_answers_them_name_sorted() {
    let mut backwards: Vec<&str> = SORTED_NAMES.to_vec();
    backwards.reverse();
    let registry = registry(
        backwards
            .iter()
            .map(|name| named(name, spec(3, SpawnRole::Fill))),
    );

    let listed: Vec<String> = registry
        .iter()
        .map(|prefab| (**prefab.name()).clone())
        .collect();
    assert_eq!(
        listed, backwards,
        "the five share one PrefabKey, so they sit in one bucket in insertion order. Without \
         that this case would not prove the sort runs",
    );

    let rows: Vec<String> = prefab_candidates(&registry)
        .into_iter()
        .map(|(prefab, _)| (**prefab.name()).clone())
        .collect();
    assert_eq!(
        rows,
        SORTED_NAMES.to_vec(),
        "the picker sorts its rows by name, so handing back the registry's own bucket order \
         fails here",
    );
}

#[test]
fn two_roles_under_one_name_answer_two_rows_player_first() {
    let registry = registry([
        named(SHARED_NAME, spec(3, SpawnRole::Enemy)),
        named(SHARED_NAME, spec(3, SpawnRole::Player)),
    ]);
    let rows = prefab_candidates(&registry);

    let [(player, player_label), (enemy, enemy_label)] = rows.as_slice() else {
        unreachable!("two prefabs under two PrefabKeys are two rows, never one: {rows:?}");
    };
    assert_eq!(
        (player.spec().role, enemy.spec().role),
        (SpawnRole::Player, SpawnRole::Enemy),
        "the name, the size and the theme all tie, so the role decides the order, and the \
         player's own deployment area is listed first",
    );
    assert_ne!(
        player_label, enemy_label,
        "a label built from the name and the size alone reads the same for both, so an author \
         could not tell which row is which: {player_label} / {enemy_label}",
    );
}

#[test]
fn two_sizes_under_one_name_answer_two_rows_smallest_first() {
    let registry = registry([
        named(SHARED_NAME, spec(6, SpawnRole::Fill)),
        named(SHARED_NAME, spec(3, SpawnRole::Fill)),
    ]);
    let rows = prefab_candidates(&registry);

    let [(small, small_label), (large, large_label)] = rows.as_slice() else {
        unreachable!(
            "two prefabs under two grid sizes are two rows, so nothing may fold them together \
             by name: {rows:?}"
        );
    };
    assert_eq!(
        (*small.spec().size.width(), *large.spec().size.width()),
        (3, 6),
        "the names tie, so the grid extent decides the order",
    );
    assert_ne!(
        small_label, large_label,
        "the label carries the extent, so two same-named prefabs of different sizes read \
         apart: {small_label} / {large_label}",
    );
}
