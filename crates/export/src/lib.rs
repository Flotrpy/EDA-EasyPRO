//! Fabrication and interchange outputs.
//!
//! Everything here is pure Rust and produces plain strings/bytes, so the
//! same code runs in the browser and in tests.

pub mod bom;
pub mod drill;
pub mod fab;
pub mod gerber;
pub mod job;
pub mod netlist;
pub mod svg;
pub mod zip;
