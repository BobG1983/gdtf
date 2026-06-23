//! The targeting-hint marker types.

use bevy::prelude::Component;

use crate::support_item;

support_item! {
    /// Marker for the ONE targeting-hint [`Text`](bevy::prelude::Text) node (GTW-11).
    ///
    /// A NAMED hint-state marker (no-bare-types: the hint node is a domain UI element, found by
    /// this marker — not a bare `String` scattered across systems). The spawn system tags the one
    /// hint Text with it; [`update_targeting_hint`](super::systems::update_targeting_hint) queries
    /// `With<TargetingHintText>` to find and MUTATE that one node (its text + visibility) in place,
    /// and the despawn system removes it on the `BattleRunning` exit. Plumbing around the framework
    /// Text node (the marker carve-out). It widens to `pub` for the test-support harness via
    /// [`support_item`](crate::support_item) (the stat-block / combat-log marker precedent) and
    /// stays `pub(crate)` (`unreachable_pub`-clean) in the binary.
    #[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
    struct TargetingHintText;
}
