//! Just enough linear algebra for an orbit camera (column-major mat4).

pub type Mat4 = [f32; 16];

pub fn mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = [0.0; 16];
    for c in 0..4 {
        for row in 0..4 {
            r[c * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[c * 4 + k]).sum();
        }
    }
    r
}

pub fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fovy / 2.0).tan();
    let nf = 1.0 / (near - far);
    [
        f / aspect,
        0.0,
        0.0,
        0.0,
        0.0,
        f,
        0.0,
        0.0,
        0.0,
        0.0,
        (far + near) * nf,
        -1.0,
        0.0,
        0.0,
        2.0 * far * near * nf,
        0.0,
    ]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn normalize(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt();
    if l == 0.0 {
        a
    } else {
        [a[0] / l, a[1] / l, a[2] / l]
    }
}

pub fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> Mat4 {
    let f = normalize(sub(target, eye));
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    [
        s[0],
        u[0],
        -f[0],
        0.0,
        s[1],
        u[1],
        -f[1],
        0.0,
        s[2],
        u[2],
        -f[2],
        0.0,
        -dot(s, eye),
        -dot(u, eye),
        dot(f, eye),
        1.0,
    ]
}

/// Orbit camera around a target. Z is up (board normal).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    pub target: [f32; 3],
    /// Rotation around Z, degrees.
    pub yaw: f32,
    /// Elevation above the board plane, degrees (-89..89).
    pub pitch: f32,
    pub distance: f32,
    pub aspect: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        OrbitCamera {
            target: [0.0; 3],
            yaw: -60.0,
            pitch: 40.0,
            distance: 120.0,
            aspect: 1.5,
        }
    }
}

impl OrbitCamera {
    pub fn eye(&self) -> [f32; 3] {
        let (y, p) = (self.yaw.to_radians(), self.pitch.to_radians());
        [
            self.target[0] + self.distance * p.cos() * y.cos(),
            self.target[1] + self.distance * p.cos() * y.sin(),
            self.target[2] + self.distance * p.sin(),
        ]
    }

    pub fn view_proj(&self) -> Mat4 {
        let up = if self.pitch.abs() > 89.0 {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let near = (self.distance * 0.01).max(0.1);
        let far = self.distance * 20.0 + 1000.0;
        mul(
            &perspective(35f32.to_radians(), self.aspect, near, far),
            &look_at(self.eye(), self.target, up),
        )
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.4;
        self.pitch = (self.pitch + dy * 0.4).clamp(-89.9, 89.9);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(2.0, 5000.0);
    }

    /// Move the target in the view plane by screen pixels.
    pub fn pan(&mut self, dx: f32, dy: f32, viewport_h: f32) {
        let scale = self.distance * 2.0 * (35f32.to_radians() / 2.0).tan() / viewport_h.max(1.0);
        let y = self.yaw.to_radians();
        let right = [-y.sin(), y.cos(), 0.0];
        let fwd = normalize(sub(self.target, self.eye()));
        let up = cross(right, fwd);
        for i in 0..3 {
            self.target[i] += (-dx * right[i] + dy * up[i]) * scale;
        }
    }

    pub fn preset(&mut self, name: &str) {
        let (yaw, pitch) = match name {
            "top" => (-90.0, 89.9),
            "bottom" => (-90.0, -89.9),
            "front" => (-90.0, 0.0),
            "left" => (180.0, 0.0),
            "right" => (0.0, 0.0),
            _ => (-60.0, 40.0),
        };
        self.yaw = yaw;
        self.pitch = pitch;
    }

    /// Distance that fits a sphere of `radius` in view.
    pub fn fit(&mut self, radius: f32) {
        self.distance = (radius / (35f32.to_radians() / 2.0).sin() * 1.1).max(5.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(m: &Mat4, p: [f32; 3]) -> [f32; 3] {
        let v = [p[0], p[1], p[2], 1.0];
        let mut o = [0.0; 4];
        for (r, out) in o.iter_mut().enumerate() {
            *out = (0..4).map(|k| m[k * 4 + r] * v[k]).sum();
        }
        [o[0] / o[3], o[1] / o[3], o[2] / o[3]]
    }

    #[test]
    fn target_projects_to_centre() {
        let c = OrbitCamera::default();
        let p = apply(&c.view_proj(), c.target);
        assert!(p[0].abs() < 1e-4 && p[1].abs() < 1e-4);
        assert!(p[2] > -1.0 && p[2] < 1.0);
    }

    #[test]
    fn top_view_sees_plus_y_up() {
        let mut c = OrbitCamera::default();
        c.preset("top");
        let p = apply(&c.view_proj(), [0.0, 10.0, 0.0]);
        assert!(p[1] > 0.0);
        let q = apply(&c.view_proj(), [10.0, 0.0, 0.0]);
        assert!(q[0] > 0.0);
    }
}
