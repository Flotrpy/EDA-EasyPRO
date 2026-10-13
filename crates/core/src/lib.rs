//! Core of EasyPRO: everything that does not need a browser.
//!
//! The model stores all lengths as integer nanometres so that geometry is
//! exact, files diff cleanly and Gerber output never accumulates rounding.

pub mod builtin;
pub mod check;
pub mod connect;
pub mod drc;
pub mod erc;
pub mod examples;
pub mod font;
pub mod geom;
pub mod history;
pub mod id;
pub mod kicad;
pub mod layer;
pub mod library;
pub mod netlist;
pub mod pcb;
pub mod pick;
pub mod pour;
pub mod project;
pub mod route;
pub mod scene;
pub mod schematic;
pub mod sexpr;
pub mod shape;
pub mod sync;
pub mod units;
