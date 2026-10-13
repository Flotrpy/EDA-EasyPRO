//! WebGL2 back end.

use crate::camera::Camera;
use crate::tess::{item, Frame, Mesh, STRIDE};
use easypro_core::scene::{Scene, SceneLayer};
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as GL, WebGlBuffer, WebGlProgram,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

const VERT: &str = r#"#version 300 es
in vec2 a_pos;
in vec2 a_dir;
in float a_hw;
in vec4 a_color;
uniform vec2 u_center;
uniform vec2 u_scale;
uniform float u_min_hw;
out vec4 v_color;
void main() {
    float hw = max(a_hw, u_min_hw);
    vec2 p = a_pos + a_dir * hw;
    gl_Position = vec4((p - u_center) * u_scale, 0.0, 1.0);
    v_color = a_color;
}
"#;

const FRAG: &str = r#"#version 300 es
precision mediump float;
in vec4 v_color;
uniform float u_alpha;
uniform vec4 u_tint;
out vec4 o;
void main() {
    vec3 c = mix(v_color.rgb, u_tint.rgb, u_tint.a);
    o = vec4(c, v_color.a * u_alpha);
}
"#;

const GRID_VERT: &str = r#"#version 300 es
in vec2 a_pos;
void main() { gl_Position = vec4(a_pos, 0.0, 1.0); }
"#;

/// Dots or lines on a world grid, with every 5th line emphasised.
const GRID_FRAG: &str = r#"#version 300 es
precision highp float;
uniform vec2 u_size;
uniform vec2 u_center;
uniform float u_zoom;
uniform float u_step;
uniform vec4 u_color;
uniform vec4 u_major;
uniform int u_lines;
uniform float u_dpr;
uniform float u_flip;
out vec4 o;
void main() {
    vec2 px = vec2(gl_FragCoord.x, gl_FragCoord.y) / u_dpr;
    vec2 world = (px - u_size * 0.5) / u_zoom * vec2(u_flip, 1.0) + u_center;
    vec2 g = world / u_step;
    vec2 d = abs(g - round(g)) * u_step * u_zoom;
    vec2 gm = world / (u_step * 5.0);
    vec2 dm = abs(gm - round(gm)) * u_step * 5.0 * u_zoom;
    float a = 0.0;
    vec4 c = u_color;
    if (u_lines == 1) {
        float m = min(dm.x, dm.y);
        float l = min(d.x, d.y);
        if (m < 0.6) { c = u_major; a = 1.0; }
        else if (l < 0.5) { a = 1.0; }
    } else {
        if (max(dm.x, dm.y) < 1.1) { c = u_major; a = 1.0; }
        else if (max(d.x, d.y) < 0.75) { a = 1.0; }
    }
    if (a == 0.0) discard;
    o = c;
}
"#;

struct Layer {
    key: SceneLayer,
    start: i32,
    count: i32,
}

struct Batch {
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
    ibo: WebGlBuffer,
    count: i32,
}

pub struct Renderer {
    gl: GL,
    canvas: HtmlCanvasElement,
    prog: WebGlProgram,
    grid_prog: WebGlProgram,
    grid_vao: WebGlVertexArrayObject,
    scene: Batch,
    overlay: Batch,
    layers: Vec<Layer>,
    pub frame: Frame,
    u: Uniforms,
}

struct Uniforms {
    center: Option<WebGlUniformLocation>,
    scale: Option<WebGlUniformLocation>,
    min_hw: Option<WebGlUniformLocation>,
    alpha: Option<WebGlUniformLocation>,
    tint: Option<WebGlUniformLocation>,
}

/// How the grid is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridStyle {
    pub step_mm: f64,
    pub lines: bool,
    pub color: [f32; 4],
    pub major: [f32; 4],
}

/// Per-layer draw options for one frame.
#[derive(Clone, Copy, Debug)]
pub struct LayerDraw {
    pub key: SceneLayer,
    pub alpha: f32,
}

fn compile(gl: &GL, kind: u32, src: &str) -> Result<web_sys::WebGlShader, String> {
    let s = gl.create_shader(kind).ok_or("cannot create shader")?;
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    if gl
        .get_shader_parameter(&s, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(s)
    } else {
        Err(gl.get_shader_info_log(&s).unwrap_or_default())
    }
}

fn link(gl: &GL, vs: &str, fs: &str) -> Result<WebGlProgram, String> {
    let p = gl.create_program().ok_or("cannot create program")?;
    gl.attach_shader(&p, &compile(gl, GL::VERTEX_SHADER, vs)?);
    gl.attach_shader(&p, &compile(gl, GL::FRAGMENT_SHADER, fs)?);
    gl.link_program(&p);
    if gl
        .get_program_parameter(&p, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(p)
    } else {
        Err(gl.get_program_info_log(&p).unwrap_or_default())
    }
}

impl Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Renderer, String> {
        let opts = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&opts, &"antialias".into(), &true.into());
        let _ = js_sys::Reflect::set(&opts, &"preserveDrawingBuffer".into(), &true.into());
        let gl: GL = canvas
            .get_context_with_context_options("webgl2", &opts)
            .map_err(|_| "WebGL2 is not available")?
            .ok_or("WebGL2 is not available in this browser")?
            .dyn_into()
            .map_err(|_| "unexpected context type")?;
        let prog = link(&gl, VERT, FRAG)?;
        let grid_prog = link(&gl, GRID_VERT, GRID_FRAG)?;
        let scene = Self::batch(&gl, &prog)?;
        let overlay = Self::batch(&gl, &prog)?;

        let grid_vao = gl.create_vertex_array().ok_or("vao")?;
        gl.bind_vertex_array(Some(&grid_vao));
        let quad = gl.create_buffer().ok_or("buffer")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&quad));
        let verts: [f32; 12] = [
            -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0,
        ];
        unsafe {
            let view = js_sys::Float32Array::view(&verts);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &view, GL::STATIC_DRAW);
        }
        let loc = gl.get_attrib_location(&grid_prog, "a_pos") as u32;
        gl.enable_vertex_attrib_array(loc);
        gl.vertex_attrib_pointer_with_i32(loc, 2, GL::FLOAT, false, 8, 0);
        gl.bind_vertex_array(None);

        let u = Uniforms {
            center: gl.get_uniform_location(&prog, "u_center"),
            scale: gl.get_uniform_location(&prog, "u_scale"),
            min_hw: gl.get_uniform_location(&prog, "u_min_hw"),
            alpha: gl.get_uniform_location(&prog, "u_alpha"),
            tint: gl.get_uniform_location(&prog, "u_tint"),
        };
        Ok(Renderer {
            gl,
            canvas,
            prog,
            grid_prog,
            grid_vao,
            scene,
            overlay,
            layers: vec![],
            frame: Frame::default(),
            u,
        })
    }

    fn batch(gl: &GL, prog: &WebGlProgram) -> Result<Batch, String> {
        let vao = gl.create_vertex_array().ok_or("vao")?;
        gl.bind_vertex_array(Some(&vao));
        let vbo = gl.create_buffer().ok_or("buffer")?;
        let ibo = gl.create_buffer().ok_or("buffer")?;
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&ibo));
        let attr = |name: &str, size: i32, ty: u32, norm: bool, off: i32| {
            let loc = gl.get_attrib_location(prog, name);
            if loc >= 0 {
                gl.enable_vertex_attrib_array(loc as u32);
                gl.vertex_attrib_pointer_with_i32(loc as u32, size, ty, norm, STRIDE as i32, off);
            }
        };
        attr("a_pos", 2, GL::FLOAT, false, 0);
        attr("a_dir", 2, GL::FLOAT, false, 8);
        attr("a_hw", 1, GL::FLOAT, false, 16);
        attr("a_color", 4, GL::UNSIGNED_BYTE, true, 20);
        gl.bind_vertex_array(None);
        Ok(Batch {
            vao,
            vbo,
            ibo,
            count: 0,
        })
    }

    fn upload(gl: &GL, b: &mut Batch, mesh: &Mesh, usage: u32) {
        gl.bind_vertex_array(Some(&b.vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&b.vbo));
        unsafe {
            let v = js_sys::Uint8Array::view(&mesh.bytes);
            gl.buffer_data_with_array_buffer_view(GL::ARRAY_BUFFER, &v, usage);
        }
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&b.ibo));
        unsafe {
            let i = js_sys::Uint32Array::view(&mesh.indices);
            gl.buffer_data_with_array_buffer_view(GL::ELEMENT_ARRAY_BUFFER, &i, usage);
        }
        gl.bind_vertex_array(None);
        b.count = mesh.indices.len() as i32;
    }

    /// Replace the static scene. Layers keep their order of appearance.
    pub fn set_scene(&mut self, scene: &Scene) {
        let mut mesh = Mesh::default();
        self.layers.clear();
        for (key, items) in &scene.layers {
            let start = mesh.indices.len() as i32;
            for it in items {
                item(&mut mesh, &self.frame, it);
            }
            self.layers.push(Layer {
                key: *key,
                start,
                count: mesh.indices.len() as i32 - start,
            });
        }
        Self::upload(&self.gl, &mut self.scene, &mesh, GL::STATIC_DRAW);
    }

    /// Replace the overlay (selection, hover, tool previews).
    pub fn set_overlay(&mut self, mesh: &Mesh) {
        Self::upload(&self.gl, &mut self.overlay, mesh, GL::DYNAMIC_DRAW);
    }

    pub fn has_layer(&self, key: SceneLayer) -> bool {
        self.layers.iter().any(|l| l.key == key && l.count > 0)
    }

    /// Match the drawing buffer to the canvas' CSS size.
    pub fn resize(&self, dpr: f64) -> (f64, f64) {
        let w = self.canvas.client_width().max(1) as f64;
        let h = self.canvas.client_height().max(1) as f64;
        let (pw, ph) = ((w * dpr) as u32, (h * dpr) as u32);
        if self.canvas.width() != pw || self.canvas.height() != ph {
            self.canvas.set_width(pw);
            self.canvas.set_height(ph);
        }
        (w, h)
    }

    /// Draw a frame. `order` lists scene layers back to front with alphas;
    /// layers not listed are hidden.
    pub fn draw(
        &self,
        cam: &Camera,
        background: [f32; 3],
        grid: Option<GridStyle>,
        order: &[LayerDraw],
        dpr: f64,
    ) {
        let gl = &self.gl;
        gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        gl.clear_color(background[0], background[1], background[2], 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT);
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        if let Some(g) = grid {
            // Hide grids too dense to be useful; coarsen by 5x first.
            let mut step = g.step_mm;
            while step * cam.zoom < 6.0 && step < 1e5 {
                step *= 5.0;
            }
            gl.use_program(Some(&self.grid_prog));
            let p = &self.grid_prog;
            gl.uniform2f(
                gl.get_uniform_location(p, "u_size").as_ref(),
                cam.size.0 as f32,
                cam.size.1 as f32,
            );
            gl.uniform2f(
                gl.get_uniform_location(p, "u_center").as_ref(),
                cam.center.0 as f32,
                cam.center.1 as f32,
            );
            gl.uniform1f(
                gl.get_uniform_location(p, "u_zoom").as_ref(),
                cam.zoom as f32,
            );
            gl.uniform1f(gl.get_uniform_location(p, "u_step").as_ref(), step as f32);
            gl.uniform1f(gl.get_uniform_location(p, "u_dpr").as_ref(), dpr as f32);
            gl.uniform1f(
                gl.get_uniform_location(p, "u_flip").as_ref(),
                cam.sx() as f32,
            );
            gl.uniform4fv_with_f32_array(gl.get_uniform_location(p, "u_color").as_ref(), &g.color);
            gl.uniform4fv_with_f32_array(gl.get_uniform_location(p, "u_major").as_ref(), &g.major);
            gl.uniform1i(
                gl.get_uniform_location(p, "u_lines").as_ref(),
                g.lines as i32,
            );
            gl.bind_vertex_array(Some(&self.grid_vao));
            gl.draw_arrays(GL::TRIANGLES, 0, 6);
        }

        gl.use_program(Some(&self.prog));
        let (ox, oy) = (
            self.frame.origin.x as f64 / 1e6,
            self.frame.origin.y as f64 / 1e6,
        );
        gl.uniform2f(
            self.u.center.as_ref(),
            (cam.center.0 - ox) as f32,
            (cam.center.1 - oy) as f32,
        );
        gl.uniform2f(
            self.u.scale.as_ref(),
            (cam.sx() * 2.0 * cam.zoom / cam.size.0) as f32,
            (2.0 * cam.zoom / cam.size.1) as f32,
        );
        // At least half a pixel of half-width: one-pixel minimum strokes.
        gl.uniform1f(self.u.min_hw.as_ref(), (0.5 / cam.zoom) as f32);
        gl.uniform4f(self.u.tint.as_ref(), 0.0, 0.0, 0.0, 0.0);

        gl.bind_vertex_array(Some(&self.scene.vao));
        for d in order {
            for l in self.layers.iter().filter(|l| l.key == d.key && l.count > 0) {
                gl.uniform1f(self.u.alpha.as_ref(), d.alpha);
                gl.draw_elements_with_i32(GL::TRIANGLES, l.count, GL::UNSIGNED_INT, l.start * 4);
            }
        }
        if self.overlay.count > 0 {
            gl.uniform1f(self.u.alpha.as_ref(), 1.0);
            gl.bind_vertex_array(Some(&self.overlay.vao));
            gl.draw_elements_with_i32(GL::TRIANGLES, self.overlay.count, GL::UNSIGNED_INT, 0);
        }
        gl.bind_vertex_array(None);
    }

    /// Layers in the current scene, in scene order.
    pub fn layer_keys(&self) -> Vec<SceneLayer> {
        self.layers.iter().map(|l| l.key).collect()
    }

    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }
}
