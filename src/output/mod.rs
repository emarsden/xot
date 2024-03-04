//! Xot offers functionality to serialize XML data in different ways.
//!
//! This module lets you control serialization in various ways.
mod common;
pub mod html5;
#[cfg(feature = "icu")]
mod icu_normalization;
mod normalizer;
mod pretty;
mod serializer;
pub mod xml;

pub use common::Indentation;
pub use normalizer::{NoopNormalizer, Normalizer};
pub(crate) use pretty::Pretty;
pub(crate) use serializer::{gen_outputs, XmlSerializer};
pub use serializer::{Output, OutputToken};
