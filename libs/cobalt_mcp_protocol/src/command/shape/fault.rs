//! Why a shape trace could not finish.

use core::fmt::{Display, Formatter, Result as FmtResult};

use bevy_derive::Deref;

use super::names::ShapeName;

/// Rust path of the type a trace started from.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TracedType(&'static str);

impl TracedType {
    /// Wrap a type path.
    #[must_use]
    pub const fn new(path: &'static str) -> Self {
        Self(path)
    }
}

/// How many walks over a type the tracer may take.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TracePassBudget(u32);

impl TracePassBudget {
    /// Wrap a pass count.
    #[must_use]
    pub const fn new(passes: u32) -> Self {
        Self(passes)
    }
}

/// Free-text detail from a deserializer error.
#[derive(Deref, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FaultDetail(String);

impl FaultDetail {
    /// Wrap a detail string.
    #[must_use]
    pub const fn new(detail: String) -> Self {
        Self(detail)
    }
}

/// What stopped the trace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShapeFaultKind {
    /// The type contains itself, so the walk would never end.
    SelfReferential(ShapeName),
    /// The type described nothing, which `untagged`, `flatten` and self-describing values do.
    AsksAny,
    /// The walk did not settle inside its pass budget.
    PassBudget(TracePassBudget),
    /// A deserializer error raised while feeding the visitor.
    Other(FaultDetail),
}

impl Display for ShapeFaultKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::SelfReferential(name) => {
                write!(formatter, "`{}` is self-referential", name.as_str())
            }
            Self::AsksAny => write!(formatter, "a type asked for `deserialize_any`"),
            Self::PassBudget(budget) => write!(
                formatter,
                "the shape did not settle within {} passes",
                **budget
            ),
            Self::Other(detail) => write!(formatter, "{}", detail.as_str()),
        }
    }
}

/// A shape trace that could not finish, naming the type it started from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapeFault {
    traced: Option<TracedType>,
    kind:   ShapeFaultKind,
}

impl ShapeFault {
    /// Build a fault with no traced type yet.
    #[must_use]
    pub const fn new(kind: ShapeFaultKind) -> Self {
        Self { traced: None, kind }
    }

    /// Name the type the trace started from.
    #[must_use]
    pub const fn traced_from(mut self, traced: TracedType) -> Self {
        self.traced = Some(traced);
        self
    }

    /// Which type the trace started from, when it is known.
    #[must_use]
    pub const fn traced(&self) -> Option<TracedType> {
        self.traced
    }

    /// What stopped the trace.
    #[must_use]
    pub const fn kind(&self) -> &ShapeFaultKind {
        &self.kind
    }
}

impl Display for ShapeFault {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self.traced {
            Some(traced) => write!(formatter, "tracing `{}`: {}", *traced, self.kind),
            None => write!(formatter, "{}", self.kind),
        }
    }
}

impl core::error::Error for ShapeFault {}

impl serde::de::Error for ShapeFault {
    fn custom<T: Display>(message: T) -> Self {
        Self::new(ShapeFaultKind::Other(FaultDetail::new(message.to_string())))
    }
}
