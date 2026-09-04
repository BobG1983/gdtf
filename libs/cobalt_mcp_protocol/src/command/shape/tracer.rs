//! Running state of a shape trace: definitions found, types entered, enums still open.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    doc::{RonShape, ShapeBody, ShapeDef, ShapeDoc, ShapeVariant, VariantShape},
    fault::{FaultDetail, ShapeFault, ShapeFaultKind},
    names::ShapeName,
};

struct EnumProgress {
    variants: Vec<ShapeName>,
    bodies:   Vec<Option<VariantShape>>,
    reaches:  Vec<BTreeSet<ShapeName>>,
}

impl EnumProgress {
    fn new(variants: &'static [&'static str]) -> Self {
        Self {
            variants: variants
                .iter()
                .map(|name| ShapeName::from_static(name))
                .collect(),
            bodies:   variants.iter().map(|_| None).collect(),
            reaches:  variants.iter().map(|_| BTreeSet::new()).collect(),
        }
    }

    fn is_settled(&self) -> bool {
        self.bodies.iter().all(Option::is_some)
    }

    fn recorded(&self) -> Vec<ShapeVariant> {
        self.variants
            .iter()
            .zip(self.bodies.iter())
            .filter_map(|(name, body)| {
                body.clone()
                    .map(|body| ShapeVariant::new(name.clone(), body))
            })
            .collect()
    }
}

/// Everything one trace has learned so far.
pub(super) struct ShapeTracer {
    defs:    BTreeMap<ShapeName, ShapeBody>,
    enums:   BTreeMap<ShapeName, EnumProgress>,
    active:  BTreeSet<ShapeName>,
    witness: Vec<BTreeSet<ShapeName>>,
    last:    Option<RonShape>,
}

impl ShapeTracer {
    pub(super) const fn new() -> Self {
        Self {
            defs:    BTreeMap::new(),
            enums:   BTreeMap::new(),
            active:  BTreeSet::new(),
            witness: Vec::new(),
            last:    None,
        }
    }

    pub(super) fn enter(&mut self, name: &ShapeName) -> Result<(), ShapeFault> {
        if !self.active.insert(name.clone()) {
            return Err(ShapeFault::new(ShapeFaultKind::SelfReferential(
                name.clone(),
            )));
        }
        Ok(())
    }

    pub(super) fn leave(&mut self, name: &ShapeName) {
        self.active.remove(name);
    }

    pub(super) fn define(&mut self, name: ShapeName, body: ShapeBody) {
        self.defs.insert(name, body);
    }

    pub(super) fn set_last(&mut self, shape: RonShape) {
        self.last = Some(shape);
    }

    pub(super) fn take_last(&mut self) -> Result<RonShape, ShapeFault> {
        self.last.take().ok_or_else(|| {
            ShapeFault::new(ShapeFaultKind::Other(FaultDetail::new(
                "a value was walked without recording a shape".to_owned(),
            )))
        })
    }

    pub(super) fn witness_enum(&mut self, name: &ShapeName) {
        for seen in &mut self.witness {
            seen.insert(name.clone());
        }
    }

    pub(super) fn open_witness(&mut self) {
        self.witness.push(BTreeSet::new());
    }

    pub(super) fn close_witness(&mut self) -> BTreeSet<ShapeName> {
        self.witness.pop().unwrap_or_default()
    }

    // Every enum reachable from `seeds`, following the enums each recorded variant entered.
    fn reachable_from(&self, seeds: &BTreeSet<ShapeName>) -> BTreeSet<ShapeName> {
        let mut seen: BTreeSet<ShapeName> = BTreeSet::new();
        let mut queue: Vec<ShapeName> = seeds.iter().cloned().collect();
        while let Some(next) = queue.pop() {
            if !seen.insert(next.clone()) {
                continue;
            }
            let Some(progress) = self.enums.get(&next) else {
                continue;
            };
            for onward in progress.reaches.iter().flatten() {
                if !seen.contains(onward) {
                    queue.push(onward.clone());
                }
            }
        }
        seen
    }

    /// Pick which variant of `name` this pass walks.
    pub(super) fn choose_variant(
        &mut self,
        name: &ShapeName,
        variants: &'static [&'static str],
    ) -> Option<usize> {
        self.enums
            .entry(name.clone())
            .or_insert_with(|| EnumProgress::new(variants));
        if variants.is_empty() {
            return None;
        }
        if let Some(index) = self
            .enums
            .get(name)?
            .bodies
            .iter()
            .position(Option::is_none)
        {
            return Some(index);
        }
        let unsettled: BTreeSet<ShapeName> = self
            .enums
            .iter()
            .filter(|(_, progress)| !progress.is_settled())
            .map(|(open, _)| open.clone())
            .collect();
        let per_variant = self.enums.get(name)?.reaches.clone();
        for (index, seeds) in per_variant.iter().enumerate() {
            if self
                .reachable_from(seeds)
                .iter()
                .any(|open| unsettled.contains(open))
            {
                return Some(index);
            }
        }
        Some(0)
    }

    pub(super) fn record_variant(
        &mut self,
        name: &ShapeName,
        index: usize,
        body: VariantShape,
        reached: BTreeSet<ShapeName>,
    ) {
        let Some(progress) = self.enums.get_mut(name) else {
            return;
        };
        if let Some(slot) = progress.bodies.get_mut(index) {
            *slot = Some(body);
        }
        if let Some(slot) = progress.reaches.get_mut(index) {
            slot.extend(reached);
        }
        let recorded = progress.recorded();
        self.defs.insert(name.clone(), ShapeBody::Choice(recorded));
    }

    pub(super) fn is_settled(&self) -> bool {
        self.enums.values().all(EnumProgress::is_settled)
    }

    pub(super) fn finish(mut self, root: RonShape) -> ShapeDoc {
        let defs = core::mem::take(&mut self.defs)
            .into_iter()
            .map(|(name, body)| ShapeDef::new(name, body))
            .collect();
        ShapeDoc::new(root, defs)
    }
}
