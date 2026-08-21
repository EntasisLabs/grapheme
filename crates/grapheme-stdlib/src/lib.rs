//! Grapheme standard library operation implementations.
//!
//! These modules provide default host-backed operation behavior used by
//! runtime dispatch and SDK default execution paths.
//!
//! ## Feature layers
//!
//! - **default (`host`)**: network/DB/email modules plus transforms (current product path).
//! - **`wasm` / `transforms`**: Wasm-safe pure transforms (`csv`, `yaml`, `html`) plus
//!   always-on `core` / `json`. Use this profile for `wasm32-wasip1` / Stage B containers.
//! - **capability modules**: `data`, `pdf`, `image`, `plot`, `media` remain opt-in.
//!
//! See `docs/internal/rfc/rfc-0005-wasm-compilable-stdlib-v1.md`.

pub mod capability;
pub mod core;
pub mod envelope;
pub mod json;
pub mod registry;

#[cfg(feature = "transforms")]
pub mod csv;
#[cfg(feature = "transforms")]
pub mod html;
#[cfg(feature = "transforms")]
pub mod yaml;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "web")]
pub mod research;
#[cfg(feature = "web")]
pub mod web;
#[cfg(feature = "net")]
pub mod smtp;
#[cfg(feature = "net")]
pub mod tcp;
#[cfg(feature = "email")]
pub mod email;
#[cfg(feature = "sql")]
pub mod sql;
#[cfg(feature = "surreal")]
pub mod surreal;

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
