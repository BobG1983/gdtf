use gdtf_battle_sim::weapon::ModeKind;

pub(in crate::states::running::game::battlescape) const MODE_ORDER: [ModeKind; 3] =
    [ModeKind::Single, ModeKind::Burst, ModeKind::Full];

pub(super) fn mode_index(kind: ModeKind) -> usize {
    MODE_ORDER.iter().position(|k| *k == kind).unwrap_or(0)
}

pub(super) fn mode_for_index(index: usize) -> Option<ModeKind> {
    MODE_ORDER.get(index).copied()
}

pub(super) fn mode_label(kind: ModeKind) -> String {
    match kind {
        ModeKind::Full => "auto".to_owned(),
        other => other.to_string(),
    }
}
