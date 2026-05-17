//! `rknn3-sys` — Safe Rust bindings for Rockchip RKNN3 Runtime API.
//!
//! This crate provides safe wrappers around the RKNN3 C API, encapsulating all
//! `unsafe` FFI calls behind a safe Rust interface. Consumers of this crate
//! should never need to use `unsafe` directly.

mod error;
pub(crate) mod ffi;
pub mod prelude;

pub use error::Error;
