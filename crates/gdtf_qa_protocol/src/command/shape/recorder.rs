//! A `serde` deserializer that records what a type asks for instead of decoding anything.

use serde::de::{Deserializer, Visitor};

use super::{
    access::{ChoiceAccess, EntryAccess, RecordAccess, TupleAccess, empty_choice},
    doc::{RonShape, ShapeBody},
    fault::{FaultDetail, ShapeFault, ShapeFaultKind},
    names::ShapeName,
    tracer::ShapeTracer,
};

macro_rules! record_scalar {
    ($method:ident, $visit:ident, $sample:expr, $shape:expr) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.tracer.set_last($shape);
            visitor.$visit($sample)
        }
    };
    ($method:ident, $visit:ident, $shape:expr) => {
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.tracer.set_last($shape);
            visitor.$visit()
        }
    };
}

/// Walks a type through its own `Deserialize` impl and records the shape it describes.
pub(super) struct ShapeRecorder<'t> {
    tracer: &'t mut ShapeTracer,
}

impl<'t> ShapeRecorder<'t> {
    pub(super) const fn new(tracer: &'t mut ShapeTracer) -> Self {
        Self { tracer }
    }
}

fn no_variant(name: &ShapeName) -> ShapeFault {
    ShapeFault::new(ShapeFaultKind::Other(FaultDetail::new(format!(
        "`{}` was walked without settling on a variant",
        name.as_str()
    ))))
}

impl<'de> Deserializer<'de> for ShapeRecorder<'_> {
    type Error = ShapeFault;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(ShapeFault::new(ShapeFaultKind::AsksAny))
    }

    record_scalar!(deserialize_bool, visit_bool, false, RonShape::Bool);
    record_scalar!(deserialize_i8, visit_i8, 0, RonShape::Int);
    record_scalar!(deserialize_i16, visit_i16, 0, RonShape::Int);
    record_scalar!(deserialize_i32, visit_i32, 0, RonShape::Int);
    record_scalar!(deserialize_i64, visit_i64, 0, RonShape::Int);
    record_scalar!(deserialize_i128, visit_i128, 0, RonShape::Int);
    record_scalar!(deserialize_u8, visit_u8, 0, RonShape::Int);
    record_scalar!(deserialize_u16, visit_u16, 0, RonShape::Int);
    record_scalar!(deserialize_u32, visit_u32, 0, RonShape::Int);
    record_scalar!(deserialize_u64, visit_u64, 0, RonShape::Int);
    record_scalar!(deserialize_u128, visit_u128, 0, RonShape::Int);
    record_scalar!(deserialize_f32, visit_f32, 0.0, RonShape::Float);
    record_scalar!(deserialize_f64, visit_f64, 0.0, RonShape::Float);
    record_scalar!(deserialize_char, visit_char, 'a', RonShape::Text);
    record_scalar!(deserialize_str, visit_borrowed_str, "", RonShape::Text);
    record_scalar!(deserialize_string, visit_borrowed_str, "", RonShape::Text);
    record_scalar!(
        deserialize_bytes,
        visit_borrowed_bytes,
        b"",
        RonShape::Bytes
    );
    record_scalar!(
        deserialize_byte_buf,
        visit_borrowed_bytes,
        b"",
        RonShape::Bytes
    );
    record_scalar!(deserialize_unit, visit_unit, RonShape::Unit);
    record_scalar!(
        deserialize_identifier,
        visit_borrowed_str,
        "",
        RonShape::Text
    );
    record_scalar!(deserialize_ignored_any, visit_unit, RonShape::Unit);

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value = visitor.visit_some(ShapeRecorder::new(&mut *self.tracer))?;
        let inner = self.tracer.take_last()?;
        self.tracer.set_last(RonShape::Optional(Box::new(inner)));
        Ok(value)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut shapes = Vec::with_capacity(1);
        let value = visitor.visit_seq(TupleAccess::new(&mut *self.tracer, 1, &mut shapes))?;
        let Some(inner) = shapes.pop() else {
            return Err(ShapeFault::new(ShapeFaultKind::Other(FaultDetail::new(
                "a list was walked without recording its element".to_owned(),
            ))));
        };
        self.tracer.set_last(RonShape::List(Box::new(inner)));
        Ok(value)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut shapes = Vec::with_capacity(2);
        let value = visitor.visit_map(EntryAccess::new(&mut *self.tracer, &mut shapes))?;
        let mut pair = shapes.into_iter();
        let (Some(key), Some(entry)) = (pair.next(), pair.next()) else {
            return Err(ShapeFault::new(ShapeFaultKind::Other(FaultDetail::new(
                "a map was walked without recording an entry".to_owned(),
            ))));
        };
        self.tracer
            .set_last(RonShape::Map(Box::new(key), Box::new(entry)));
        Ok(value)
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let mut shapes = Vec::with_capacity(len);
        let value = visitor.visit_seq(TupleAccess::new(&mut *self.tracer, len, &mut shapes))?;
        self.tracer.set_last(RonShape::Tuple(shapes));
        Ok(value)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let named = ShapeName::from_static(name);
        self.tracer.enter(&named)?;
        self.tracer.leave(&named);
        self.tracer
            .define(named.clone(), ShapeBody::Wraps(RonShape::Unit));
        self.tracer.set_last(RonShape::Named(named));
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let named = ShapeName::from_static(name);
        self.tracer.enter(&named)?;
        let value = visitor.visit_newtype_struct(ShapeRecorder::new(&mut *self.tracer))?;
        let inner = self.tracer.take_last()?;
        self.tracer.leave(&named);
        self.tracer.define(
            named.clone(),
            ShapeBody::Wraps(RonShape::Tuple(vec![inner])),
        );
        self.tracer.set_last(RonShape::Named(named));
        Ok(value)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let named = ShapeName::from_static(name);
        self.tracer.enter(&named)?;
        let mut shapes = Vec::with_capacity(len);
        let value = visitor.visit_seq(TupleAccess::new(&mut *self.tracer, len, &mut shapes))?;
        self.tracer.leave(&named);
        self.tracer
            .define(named.clone(), ShapeBody::Wraps(RonShape::Tuple(shapes)));
        self.tracer.set_last(RonShape::Named(named));
        Ok(value)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let named = ShapeName::from_static(name);
        self.tracer.enter(&named)?;
        let mut recorded = Vec::with_capacity(fields.len());
        let value =
            visitor.visit_map(RecordAccess::new(&mut *self.tracer, fields, &mut recorded))?;
        self.tracer.leave(&named);
        self.tracer
            .define(named.clone(), ShapeBody::Record(recorded));
        self.tracer.set_last(RonShape::Named(named));
        Ok(value)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        let named = ShapeName::from_static(name);
        self.tracer.witness_enum(&named);
        self.tracer.enter(&named)?;
        let chosen = self
            .tracer
            .choose_variant(&named, variants)
            .and_then(|index| variants.get(index).map(|variant| (index, *variant)));
        let Some((index, variant)) = chosen else {
            return Err(empty_choice(&named));
        };
        let mut body = None;
        self.tracer.open_witness();
        let value = visitor.visit_enum(ChoiceAccess::new(&mut *self.tracer, variant, &mut body))?;
        let reached = self.tracer.close_witness();
        self.tracer.leave(&named);
        let Some(body) = body else {
            return Err(no_variant(&named));
        };
        self.tracer.record_variant(&named, index, body, reached);
        self.tracer.set_last(RonShape::Named(named));
        Ok(value)
    }
}
