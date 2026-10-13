//! 3D preview: the board as a solid with its copper, mask and silkscreen,
//! and parametric component bodies whose leads follow the footprint pads.

// Small fixed-size vector loops read better with indices.
#![allow(clippy::needless_range_loop)]
pub mod build;
pub mod gl;
pub mod math;
pub mod mesh;

pub use build::{board_mesh, View3dOptions};
pub use gl::Viewer;
pub use math::OrbitCamera;
