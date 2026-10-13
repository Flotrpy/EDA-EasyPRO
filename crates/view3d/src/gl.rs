//! WebGL2 viewer for board meshes.

use crate::math::OrbitCamera;
use crate::mesh::{Mesh3, STRIDE};
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as GL, WebGlBuffer, WebGlProgram,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

const VERT: &str = r#"#version 300 es
in vec3 a_pos;
in vec3 a_normal;
in vec4 a_color;
uniform mat4 u_vp;
out vec3 v_normal;
out vec4 v_color;
out vec3 v_pos;
void main() {
    gl_Position = u_vp * vec4(a_pos, 1.0);
    v_normal = a_normal;
    v_color = a_color;
    v_pos = a_pos;
}
"#;

const FRAG: &str = r#"#version 300 es
precision mediump float;
in vec3 v_normal;
in vec4 v_color;
in vec3 v_pos;
uniform vec3 u_eye;
out vec4 o;
void main() {
    vec3 n = normalize(v_normal);
    vec3 l1 = normalize(vec3(0.4, -0.5, 1.0));
    vec3 l2 = normalize(vec3(-0.6, 0.7, 0.4));
    vec3 v = normalize(u_eye - v_pos);
    float d = max(dot(n, l1), 0.0) * 0.65 + max(dot(n, l2), 0.0) * 0.25;
    // Light both faces so thin flat layers read correctly from below.
    float back = max(dot(-n, l1), 0.0) * 0.15;
    vec3 h = normalize(l1 + v);
    float s = pow(max(dot(n, h), 0.0), 48.0) * 0.25;
    vec3 c = v_color.rgb * (0.32 + d + back) + vec3(s);
    o = vec4(c, v_color.a);
}
"#;

pub struct Viewer {
    gl: GL,
    canvas: HtmlCanvasElement,
    prog: WebGlProgram,
    vao: WebGlVertexArrayObject,
    vbo: WebGlBuffer,
    ibo: WebGlBuffer,
    count: i32,
    u_vp: Option<WebGlUniformLocation>,
    u_eye: Option<WebGlUniformLocation>,
}

fn compile(gl: &GL, kind: u32, src: &str) -> Result<web_sys::WebGlShader, String> {
    let s = gl.create_shader(kind).ok_or("shader")?;
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

impl Viewer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Viewer, String> {
        let opts = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&opts, &"antialias".into(), &true.into());
        let _ = js_sys::Reflect::set(&opts, &"preserveDrawingBuffer".into(), &true.into());
        let gl: GL = canvas
            .get_context_with_context_options("webgl2", &opts)
            .map_err(|_| "WebGL2 is not available")?
            .ok_or("WebGL2 is not available in this browser")?
            .dyn_into()
            .map_err(|_| "unexpected context")?;
        let prog = gl.create_program().ok_or("program")?;
        gl.attach_shader(&prog, &compile(&gl, GL::VERTEX_SHADER, VERT)?);
        gl.attach_shader(&prog, &compile(&gl, GL::FRAGMENT_SHADER, FRAG)?);
        gl.link_program(&prog);
        if !gl
            .get_program_parameter(&prog, GL::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            return Err(gl.get_program_info_log(&prog).unwrap_or_default());
        }
        let vao = gl.create_vertex_array().ok_or("vao")?;
        let vbo = gl.create_buffer().ok_or("buffer")?;
        let ibo = gl.create_buffer().ok_or("buffer")?;
        gl.bind_vertex_array(Some(&vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&vbo));
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&ibo));
        for (name, size, ty, norm, off) in [
            ("a_pos", 3, GL::FLOAT, false, 0),
            ("a_normal", 3, GL::FLOAT, false, 12),
            ("a_color", 4, GL::UNSIGNED_BYTE, true, 24),
        ] {
            let loc = gl.get_attrib_location(&prog, name);
            if loc >= 0 {
                gl.enable_vertex_attrib_array(loc as u32);
                gl.vertex_attrib_pointer_with_i32(loc as u32, size, ty, norm, STRIDE as i32, off);
            }
        }
        gl.bind_vertex_array(None);
        let u_vp = gl.get_uniform_location(&prog, "u_vp");
        let u_eye = gl.get_uniform_location(&prog, "u_eye");
        Ok(Viewer {
            gl,
            canvas,
            prog,
            vao,
            vbo,
            ibo,
            count: 0,
            u_vp,
            u_eye,
        })
    }

    pub fn set_mesh(&mut self, m: &Mesh3) {
        let gl = &self.gl;
        gl.bind_vertex_array(Some(&self.vao));
        gl.bind_buffer(GL::ARRAY_BUFFER, Some(&self.vbo));
        unsafe {
            gl.buffer_data_with_array_buffer_view(
                GL::ARRAY_BUFFER,
                &js_sys::Uint8Array::view(&m.bytes),
                GL::STATIC_DRAW,
            );
        }
        gl.bind_buffer(GL::ELEMENT_ARRAY_BUFFER, Some(&self.ibo));
        unsafe {
            gl.buffer_data_with_array_buffer_view(
                GL::ELEMENT_ARRAY_BUFFER,
                &js_sys::Uint32Array::view(&m.indices),
                GL::STATIC_DRAW,
            );
        }
        gl.bind_vertex_array(None);
        self.count = m.indices.len() as i32;
    }

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

    pub fn draw(&self, cam: &OrbitCamera, background: [f32; 3]) {
        let gl = &self.gl;
        gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        gl.clear_color(background[0], background[1], background[2], 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);
        gl.enable(GL::DEPTH_TEST);
        gl.enable(GL::BLEND);
        gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
        gl.use_program(Some(&self.prog));
        gl.uniform_matrix4fv_with_f32_array(self.u_vp.as_ref(), false, &cam.view_proj());
        let e = cam.eye();
        gl.uniform3f(self.u_eye.as_ref(), e[0], e[1], e[2]);
        gl.bind_vertex_array(Some(&self.vao));
        gl.draw_elements_with_i32(GL::TRIANGLES, self.count, GL::UNSIGNED_INT, 0);
        gl.bind_vertex_array(None);
    }

    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }
}
