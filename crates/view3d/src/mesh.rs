//! Triangle meshes with per-vertex normals and colours.

/// Bytes per vertex: position (3 x f32), normal (3 x f32), RGBA8.
pub const STRIDE: usize = 28;

#[derive(Clone, Debug, Default)]
pub struct Mesh3 {
    pub bytes: Vec<u8>,
    pub indices: Vec<u32>,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

pub type V3 = [f32; 3];

impl Mesh3 {
    pub fn vertex_count(&self) -> u32 {
        (self.bytes.len() / STRIDE) as u32
    }

    pub fn vertex(&mut self, p: V3, n: V3, rgba: u32) -> u32 {
        if self.vertex_count() == 0 {
            self.min = p;
            self.max = p;
        }
        for i in 0..3 {
            self.min[i] = self.min[i].min(p[i]);
            self.max[i] = self.max[i].max(p[i]);
        }
        let i = self.vertex_count();
        for v in p.into_iter().chain(n) {
            self.bytes.extend(v.to_le_bytes());
        }
        self.bytes.extend(rgba.to_be_bytes());
        i
    }

    /// Quad a-b-c-d (counter-clockwise seen from the front) with flat normal.
    pub fn quad(&mut self, a: V3, b: V3, c: V3, d: V3, rgba: u32) {
        let n = crate::math::normalize(cross(sub(b, a), sub(c, a)));
        let i = [
            self.vertex(a, n, rgba),
            self.vertex(b, n, rgba),
            self.vertex(c, n, rgba),
            self.vertex(d, n, rgba),
        ];
        self.indices.extend([i[0], i[1], i[2], i[0], i[2], i[3]]);
    }

    pub fn cuboid(&mut self, min: V3, max: V3, rgba: u32) {
        let [x0, y0, z0] = min;
        let [x1, y1, z1] = max;
        self.quad([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1], rgba); // top
        self.quad([x0, y1, z0], [x1, y1, z0], [x1, y0, z0], [x0, y0, z0], rgba); // bottom
        self.quad([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1], rgba); // -y
        self.quad([x1, y1, z0], [x0, y1, z0], [x0, y1, z1], [x1, y1, z1], rgba); // +y
        self.quad([x0, y1, z0], [x0, y0, z0], [x0, y0, z1], [x0, y1, z1], rgba); // -x
        self.quad([x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1], rgba);
        // +x
    }

    /// Upright cylinder with caps.
    pub fn cylinder(&mut self, c: [f32; 2], r: f32, z0: f32, z1: f32, rgba: u32, cap: u32) {
        let n = 32;
        let pt = |i: usize| {
            let t = std::f32::consts::TAU * i as f32 / n as f32;
            (t.cos(), t.sin())
        };
        for i in 0..n {
            let (ca, sa) = pt(i);
            let (cb, sb) = pt(i + 1);
            let a0 = self.vertex([c[0] + r * ca, c[1] + r * sa, z0], [ca, sa, 0.0], rgba);
            let b0 = self.vertex([c[0] + r * cb, c[1] + r * sb, z0], [cb, sb, 0.0], rgba);
            let b1 = self.vertex([c[0] + r * cb, c[1] + r * sb, z1], [cb, sb, 0.0], rgba);
            let a1 = self.vertex([c[0] + r * ca, c[1] + r * sa, z1], [ca, sa, 0.0], rgba);
            self.indices.extend([a0, b0, b1, a0, b1, a1]);
        }
        self.disc(c, r, z1, true, cap);
        self.disc(c, r, z0, false, rgba);
    }

    pub fn disc(&mut self, c: [f32; 2], r: f32, z: f32, up: bool, rgba: u32) {
        let n = 32;
        let nz = if up { 1.0 } else { -1.0 };
        let center = self.vertex([c[0], c[1], z], [0.0, 0.0, nz], rgba);
        let ring: Vec<u32> = (0..n)
            .map(|i| {
                let t = std::f32::consts::TAU * i as f32 / n as f32;
                self.vertex(
                    [c[0] + r * t.cos(), c[1] + r * t.sin(), z],
                    [0.0, 0.0, nz],
                    rgba,
                )
            })
            .collect();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if up {
                self.indices.extend([center, a, b]);
            } else {
                self.indices.extend([center, b, a]);
            }
        }
    }

    /// Hemisphere-ish dome (LED lens) on top of a cylinder.
    pub fn dome(&mut self, c: [f32; 2], r: f32, z: f32, rgba: u32) {
        let rings = 6;
        let n = 24;
        let mut prev: Option<Vec<u32>> = None;
        for k in 0..=rings {
            let phi = std::f32::consts::FRAC_PI_2 * k as f32 / rings as f32;
            let (rr, zz) = (r * phi.cos(), z + r * phi.sin());
            let row: Vec<u32> = (0..n)
                .map(|i| {
                    let t = std::f32::consts::TAU * i as f32 / n as f32;
                    let nrm = [phi.cos() * t.cos(), phi.cos() * t.sin(), phi.sin()];
                    self.vertex([c[0] + rr * t.cos(), c[1] + rr * t.sin(), zz], nrm, rgba)
                })
                .collect();
            if let Some(p) = &prev {
                for i in 0..n {
                    let j = (i + 1) % n;
                    self.indices
                        .extend([p[i], p[j], row[j], p[i], row[j], row[i]]);
                }
            }
            prev = Some(row);
        }
    }

    /// Flat polygon (with holes) at height `z` facing up or down.
    pub fn flat(
        &mut self,
        outer: &[[f32; 2]],
        holes: &[Vec<[f32; 2]>],
        z: f32,
        up: bool,
        rgba: u32,
    ) {
        if outer.len() < 3 {
            return;
        }
        let mut flat = Vec::new();
        let mut idx = Vec::new();
        let mut pts = Vec::new();
        for p in outer {
            flat.extend([p[0] as f64, p[1] as f64]);
            pts.push(*p);
        }
        for h in holes.iter().filter(|h| h.len() >= 3) {
            idx.push(pts.len());
            for p in h {
                flat.extend([p[0] as f64, p[1] as f64]);
                pts.push(*p);
            }
        }
        let Ok(tris) = earcutr::earcut(&flat, &idx, 2) else {
            return;
        };
        let nz = if up { 1.0 } else { -1.0 };
        let base = self.vertex_count();
        for p in &pts {
            self.vertex([p[0], p[1], z], [0.0, 0.0, nz], rgba);
        }
        // Earcut winding is not guaranteed; orient each triangle by the normal.
        for t in tris.chunks(3) {
            let (a, b, c) = (pts[t[0]], pts[t[1]], pts[t[2]]);
            let ccw = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) > 0.0;
            let (i, j, k) = (base + t[0] as u32, base + t[1] as u32, base + t[2] as u32);
            if ccw == up {
                self.indices.extend([i, j, k]);
            } else {
                self.indices.extend([i, k, j]);
            }
        }
    }

    /// Vertical walls along a closed outline between z0 and z1.
    pub fn walls(&mut self, outline: &[[f32; 2]], z0: f32, z1: f32, rgba: u32) {
        let n = outline.len();
        let ccw = (0..n)
            .map(|i| {
                let (a, b) = (outline[i], outline[(i + 1) % n]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f32>()
            > 0.0;
        for i in 0..n {
            let (mut a, mut b) = (outline[i], outline[(i + 1) % n]);
            if !ccw {
                std::mem::swap(&mut a, &mut b);
            }
            self.quad(
                [a[0], a[1], z0],
                [b[0], b[1], z0],
                [b[0], b[1], z1],
                [a[0], a[1], z1],
                rgba,
            );
        }
    }

    pub fn append(&mut self, o: &Mesh3) {
        if o.vertex_count() == 0 {
            return;
        }
        if self.vertex_count() == 0 {
            self.min = o.min;
            self.max = o.max;
        }
        let base = self.vertex_count();
        self.bytes.extend(&o.bytes);
        self.indices.extend(o.indices.iter().map(|i| i + base));
        for i in 0..3 {
            self.min[i] = self.min[i].min(o.min[i]);
            self.max[i] = self.max[i].max(o.max[i]);
        }
    }
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normal(m: &Mesh3, i: u32) -> V3 {
        let o = i as usize * STRIDE + 12;
        let f = |k: usize| f32::from_le_bytes(m.bytes[o + k..o + k + 4].try_into().unwrap());
        [f(0), f(4), f(8)]
    }

    #[test]
    fn cuboid_normals_point_outwards() {
        let mut m = Mesh3::default();
        m.cuboid([0.0; 3], [1.0, 2.0, 3.0], 0xffffffff);
        assert_eq!(m.indices.len(), 36);
        assert_eq!(normal(&m, 0), [0.0, 0.0, 1.0]);
        assert_eq!(normal(&m, 4), [0.0, 0.0, -1.0]);
        assert_eq!(m.max, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn flat_polygon_faces_requested_side() {
        let mut m = Mesh3::default();
        let sq = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        m.flat(&sq, &[], 0.0, true, 1);
        for t in m.indices.chunks(3) {
            let p = |i: u32| {
                let o = i as usize * STRIDE;
                let f =
                    |k: usize| f32::from_le_bytes(m.bytes[o + k..o + k + 4].try_into().unwrap());
                [f(0), f(4)]
            };
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            assert!((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) > 0.0);
        }
    }
}
