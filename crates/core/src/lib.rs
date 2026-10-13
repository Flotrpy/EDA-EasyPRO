//! Core of EasyPRO: everything that does not need a browser.
//!
//! The model stores all lengths as integer nanometres so that geometry is
//! exact, files diff cleanly and Gerber output never accumulates rounding.

pub mod font;
pub mod geom;
pub mod layer;
pub mod library;
pub mod pour;
pub mod shape;
pub mod units;
