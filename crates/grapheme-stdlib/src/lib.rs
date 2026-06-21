//! Grapheme standard library operation implementations.
//!
//! These modules provide default host-backed operation behavior used by
//! runtime dispatch and SDK default execution paths.

pub mod capability;
pub mod core;
pub mod csv;
pub mod email;
pub mod envelope;
pub mod html;
pub mod http;
pub mod json;
pub mod registry;
pub mod research;
pub mod smtp;
pub mod sql;
pub mod surreal;
pub mod tcp;
pub mod web;
pub mod yaml;

#[cfg(feature = "data")]
pub mod data;
#[cfg(feature = "pdf")]
pub mod pdf;
#[cfg(feature = "image")]
pub mod image;
#[cfg(feature = "plot")]
pub mod plot;
#[cfg(feature = "media")]
pub mod media;
