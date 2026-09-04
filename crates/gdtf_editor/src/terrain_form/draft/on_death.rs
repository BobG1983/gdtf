//! The terrain draft's on-death list: read it, replace it, and the form's own row buttons.

use gdtf_battle_sim::effects::on_death::OnDeathEffect;

use super::fields::TerrainDraft;

impl TerrainDraft {
    /// On-death effects, in the authored order.
    #[must_use]
    pub fn on_death(&self) -> &[OnDeathEffect] {
        &self.on_death
    }

    /// Replace the whole on-death list.
    pub fn replace_on_death(&mut self, effects: Vec<OnDeathEffect>) {
        self.on_death = effects;
    }

    /// Append one effect, the way the form's Add button appends.
    pub fn add_on_death(&mut self, effect: OnDeathEffect) {
        self.on_death.push(effect);
    }

    /// Remove the effect at an index. Answers whether one was removed.
    pub fn remove_on_death(&mut self, index: usize) -> bool {
        if index >= self.on_death.len() {
            return false;
        }
        self.on_death.remove(index);
        true
    }

    /// Rewrite the effect at an index. Answers whether one was written.
    pub fn set_on_death_at(&mut self, index: usize, effect: OnDeathEffect) -> bool {
        match self.on_death.get_mut(index) {
            Some(slot) => {
                *slot = effect;
                true
            }
            None => false,
        }
    }

    /// Move the effect at an index a slot toward the start. Answers whether it moved.
    pub fn move_on_death_up(&mut self, index: usize) -> bool {
        if index == 0 || index >= self.on_death.len() {
            return false;
        }
        self.on_death.swap(index - 1, index);
        true
    }

    /// Move the effect at an index a slot toward the end. Answers whether it moved.
    pub fn move_on_death_down(&mut self, index: usize) -> bool {
        if index + 1 >= self.on_death.len() {
            return false;
        }
        self.on_death.swap(index, index + 1);
        true
    }
}
