//! Core of EasyPRO: everything that does not need a browser.
//!
//! The model stores all lengths as integer nanometres so that geometry is
//! exact, files diff cleanly and Gerber output never accumulates rounding.

pub mod font;
pub mod geom;
pub mod history;
pub mod id;
pub mod layer;
pub mod library;
pub mod netlist;
pub mod pcb;
pub mod pour;
pub mod project;
pub mod schematic;
pub mod shape;
pub mod units;
