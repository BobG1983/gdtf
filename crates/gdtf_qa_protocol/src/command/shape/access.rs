//! Throwaway `serde` accessors that walk a type's children while recording their shapes.

use serde::{
    de::{DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
    forward_to_deserialize_any,
};

use super::{
    doc::{RonShape, ShapeField, VariantShape},
    fault::{FaultDetail, ShapeFault, ShapeFaultKind},
    names::{ShapeFieldName, ShapeName},
    recorder::ShapeRecorder,
    tracer::ShapeTracer,
};

fn ran_out(what: &str) -> ShapeFault {
    ShapeFault::new(ShapeFaultKind::Other(FaultDetail::new(format!(
        "the recorder was asked for more {what} than the type declared"
    ))))
}

// Answers every request with one fixed name, which is what an identifier position wants.
pub(super) struct IdentRecorder(&'static str);

impl IdentRecorder {
    pub(super) const fn new(name: &'static str) -> Self {
        Self(name)
    }
}

impl<'de> serde::Deserializer<'de> for IdentRecorder {
    type Error = ShapeFault;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_str(self.0)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier
        ignored_any
    }
}

// Feeds a named-field struct every field it declared, once, in order.
pub(super) struct RecordAccess<'t, 'o> {
    tracer: &'t mut ShapeTracer,
    fields: &'static [&'static str],
    at:     usize,
    out:    &'o mut Vec<ShapeField>,
}

impl<'t, 'o> RecordAccess<'t, 'o> {
    pub(super) const fn new(
        tracer: &'t mut ShapeTracer,
        fields: &'static [&'static str],
        out: &'o mut Vec<ShapeField>,
    ) -> Self {
        Self {
            tracer,
            fields,
            at: 0,
            out,
        }
    }
}

impl<'de> MapAccess<'de> for RecordAccess<'_, '_> {
    type Error = ShapeFault;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.fields.get(self.at) {
            None => Ok(None),
            Some(name) => seed.deserialize(IdentRecorder::new(name)).map(Some),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let Some(name) = self.fields.get(self.at) else {
            return Err(ran_out("fields"));
        };
        self.at += 1;
        let value = seed.deserialize(ShapeRecorder::new(&mut *self.tracer))?;
        let shape = self.tracer.take_last()?;
        self.out
            .push(ShapeField::new(ShapeFieldName::from_static(name), shape));
        Ok(value)
    }
}

// Feeds a fixed-length sequence: a tuple, a tuple struct, or one element of a list.
pub(super) struct TupleAccess<'t, 'o> {
    tracer: &'t mut ShapeTracer,
    len:    usize,
    at:     usize,
    out:    &'o mut Vec<RonShape>,
}

impl<'t, 'o> TupleAccess<'t, 'o> {
    pub(super) const fn new(
        tracer: &'t mut ShapeTracer,
        len: usize,
        out: &'o mut Vec<RonShape>,
    ) -> Self {
        Self {
            tracer,
            len,
            at: 0,
            out,
        }
    }
}

impl<'de> SeqAccess<'de> for TupleAccess<'_, '_> {
    type Error = ShapeFault;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        if self.at >= self.len {
            return Ok(None);
        }
        self.at += 1;
        let value = seed.deserialize(ShapeRecorder::new(&mut *self.tracer))?;
        self.out.push(self.tracer.take_last()?);
        Ok(Some(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len.saturating_sub(self.at))
    }
}

// Feeds a map exactly one entry, which is all it takes to learn the key and value shapes.
pub(super) struct EntryAccess<'t, 'o> {
    tracer: &'t mut ShapeTracer,
    served: bool,
    out:    &'o mut Vec<RonShape>,
}

impl<'t, 'o> EntryAccess<'t, 'o> {
    pub(super) const fn new(tracer: &'t mut ShapeTracer, out: &'o mut Vec<RonShape>) -> Self {
        Self {
            tracer,
            served: false,
            out,
        }
    }
}

impl<'de> MapAccess<'de> for EntryAccess<'_, '_> {
    type Error = ShapeFault;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        if self.served {
            return Ok(None);
        }
        self.served = true;
        let value = seed.deserialize(ShapeRecorder::new(&mut *self.tracer))?;
        self.out.push(self.tracer.take_last()?);
        Ok(Some(value))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let value = seed.deserialize(ShapeRecorder::new(&mut *self.tracer))?;
        self.out.push(self.tracer.take_last()?);
        Ok(value)
    }
}

// Feeds an enum the one variant this pass chose, and records that variant's body.
pub(super) struct ChoiceAccess<'t, 'o> {
    tracer:  &'t mut ShapeTracer,
    variant: &'static str,
    out:     &'o mut Option<VariantShape>,
}

impl<'t, 'o> ChoiceAccess<'t, 'o> {
    pub(super) const fn new(
        tracer: &'t mut ShapeTracer,
        variant: &'static str,
        out: &'o mut Option<VariantShape>,
    ) -> Self {
        Self {
            tracer,
            variant,
            out,
        }
    }
}

impl<'de> EnumAccess<'de> for ChoiceAccess<'_, '_> {
    type Error = ShapeFault;
    type Variant = Self;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let named = seed.deserialize(IdentRecorder::new(self.variant))?;
        Ok((named, self))
    }
}

impl<'de> VariantAccess<'de> for ChoiceAccess<'_, '_> {
    type Error = ShapeFault;

    fn unit_variant(self) -> Result<(), Self::Error> {
        *self.out = Some(VariantShape::Unit);
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        let value = seed.deserialize(ShapeRecorder::new(&mut *self.tracer))?;
        *self.out = Some(VariantShape::Newtype(self.tracer.take_last()?));
        Ok(value)
    }

    fn tuple_variant<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let mut shapes = Vec::with_capacity(len);
        let value = visitor.visit_seq(TupleAccess::new(&mut *self.tracer, len, &mut shapes))?;
        *self.out = Some(VariantShape::Tuple(shapes));
        Ok(value)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let mut recorded = Vec::with_capacity(fields.len());
        let value =
            visitor.visit_map(RecordAccess::new(&mut *self.tracer, fields, &mut recorded))?;
        *self.out = Some(VariantShape::Record(recorded));
        Ok(value)
    }
}

// Names a type the recorder was asked to re-enter, or that declared no variants.
pub(super) fn empty_choice(name: &ShapeName) -> ShapeFault {
    ShapeFault::new(ShapeFaultKind::Other(FaultDetail::new(format!(
        "`{}` declares no variants, so nothing can be written for it",
        name.as_str()
    ))))
}
