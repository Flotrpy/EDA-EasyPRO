//! WebGL2 rendering of EasyPRO scenes.
//!
//! Scenes are tessellated once into GPU buffers. Panning and zooming only
//! change shader uniforms, so large boards stay smooth.

pub mod camera;
pub mod gl;
pub mod tess;

pub use camera::Camera;
pub use gl::Renderer;
