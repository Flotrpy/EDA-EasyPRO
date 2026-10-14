//! Scene tessellation into triangle meshes.
//!
//! Each vertex carries a centre position, an extrusion direction and a
//! half width. The vertex shader grows strokes to a minimum on-screen
//! width, so thin lines never vanish when zoomed out.

use easypro_core::geom::Point;
use easypro_core::scene::{Item, Prim};

/// Bytes per vertex: pos (2 x f32), dir (2 x f32), half width (f32), RGBA8.
pub const STRIDE: usize = 24;

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub bytes: Vec<u8>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn vertex_count(&self) -> u32 {
        (self.bytes.len() / STRIDE) as u32
    }

    fn vertex(&mut self, pos: (f32, f32), dir: (f32, f32), hw: f32, color: u32) -> u32 {
        let i = self.vertex_count();
        for v in [pos.0, pos.1, dir.0, dir.1, hw] {
            self.bytes.extend(v.to_le_bytes());
        }
        self.bytes.extend(color.to_be_bytes());
        i
    }

    fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }

    pub fn clear(&mut self) {
        self.bytes.clear();
        self.indices.clear();
    }

    pub fn append(&mut self, other: &Mesh) {
        let base = self.vertex_count();
        self.bytes.extend(&other.bytes);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Read back a vertex position (tests and picking).
    pub fn position(&self, i: u32) -> (f32, f32) {
        let o = i as usize * STRIDE;
        let f = |k: usize| f32::from_le_bytes(self.bytes[o + k..o + k + 4].try_into().unwrap());
        (f(0), f(4))
    }
}

/// Converts nanometre model coordinates to f32 millimetres relative to an
/// origin, which keeps full precision near the design.
#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    pub origin: Point,
}

impl Frame {
    pub fn to_local(&self, p: Point) -> (f32, f32) {
        (
            ((p.x - self.origin.x) as f64 / 1e6) as f32,
            ((p.y - self.origin.y) as f64 / 1e6) as f32,
        )
    }

    pub fn len(&self, nm: i64) -> f32 {
        (nm as f64 / 1e6) as f32
    }
}

fn segments_for(r: f32) -> usize {
    // Roughly constant chord error, capped for very large circles.
    ((r.sqrt() * 24.0) as usize).clamp(12, 64)
}

pub fn seg(m: &mut Mesh, a: (f32, f32), b: (f32, f32), hw: f32, color: u32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let (nx, ny) = if len > 0.0 {
        (-dy / len, dx / len)
    } else {
        (0.0, 1.0)
    };
    let v0 = m.vertex(a, (nx, ny), hw, color);
    let v1 = m.vertex(a, (-nx, -ny), hw, color);
    let v2 = m.vertex(b, (nx, ny), hw, color);
    let v3 = m.vertex(b, (-nx, -ny), hw, color);
    m.tri(v0, v1, v2);
    m.tri(v2, v1, v3);
    // Round caps: half fans at both ends.
    let base = ny.atan2(nx);
    for (c, start) in [(a, base), (b, base + std::f32::consts::PI)] {
        let n = 8;
        let center = m.vertex(c, (0.0, 0.0), 0.0, color);
        let mut prev = m.vertex(c, (start.cos(), start.sin()), hw, color);
        for i in 1..=n {
            let t = start + std::f32::consts::PI * i as f32 / n as f32;
            let cur = m.vertex(c, (t.cos(), t.sin()), hw, color);
            m.tri(center, prev, cur);
            prev = cur;
        }
    }
}

pub fn disc(m: &mut Mesh, c: (f32, f32), r: f32, color: u32) {
    let n = segments_for(r);
    let center = m.vertex(c, (0.0, 0.0), 0.0, color);
    let first = m.vertex((c.0 + r, c.1), (0.0, 0.0), 0.0, color);
    let mut prev = first;
    for i in 1..n {
        let t = std::f32::consts::TAU * i as f32 / n as f32;
        let cur = m.vertex(
            (c.0 + r * t.cos(), c.1 + r * t.sin()),
            (0.0, 0.0),
            0.0,
            color,
        );
        m.tri(center, prev, cur);
        prev = cur;
    }
    m.tri(center, prev, first);
}

pub fn ring(m: &mut Mesh, c: (f32, f32), r: f32, hw: f32, color: u32) {
    let n = segments_for(r);
    let mut first = None;
    let mut prev: Option<(u32, u32)> = None;
    for i in 0..n {
        let t = std::f32::consts::TAU * i as f32 / n as f32;
        let (dx, dy) = (t.cos(), t.sin());
        let p = (c.0 + r * dx, c.1 + r * dy);
        let outer = m.vertex(p, (dx, dy), hw, color);
        let inner = m.vertex(p, (-dx, -dy), hw, color);
        if let Some((po, pi)) = prev {
            m.tri(po, pi, outer);
            m.tri(outer, pi, inner);
        } else {
            first = Some((outer, inner));
        }
        prev = Some((outer, inner));
    }
    if let (Some((po, pi)), Some((fo, fi))) = (prev, first) {
        m.tri(po, pi, fo);
        m.tri(fo, pi, fi);
    }
}

pub fn polygon(m: &mut Mesh, outer: &[(f32, f32)], holes: &[Vec<(f32, f32)>], color: u32) {
    if outer.len() < 3 {
        return;
    }
    let mut flat: Vec<f64> =
        Vec::with_capacity((outer.len() + holes.iter().map(|h| h.len()).sum::<usize>()) * 2);
    let mut hole_idx = Vec::new();
    let mut pts: Vec<(f32, f32)> = Vec::new();
    for p in outer {
        flat.extend([p.0 as f64, p.1 as f64]);
        pts.push(*p);
    }
    for h in holes.iter().filter(|h| h.len() >= 3) {
        hole_idx.push(pts.len());
        for p in h {
            flat.extend([p.0 as f64, p.1 as f64]);
            pts.push(*p);
        }
    }
    let Ok(tris) = earcutr::earcut(&flat, &hole_idx, 2) else {
        return;
    };
    let base = m.vertex_count();
    for p in &pts {
        m.vertex(*p, (0.0, 0.0), 0.0, color);
    }
    for t in tris.chunks(3) {
        m.tri(base + t[0] as u32, base + t[1] as u32, base + t[2] as u32);
    }
}

/// Append one scene item.
pub fn item(m: &mut Mesh, f: &Frame, it: &Item) {
    item_colored(m, f, it, it.color);
}

pub fn item_colored(m: &mut Mesh, f: &Frame, it: &Item, color: u32) {
    match &it.prim {
        Prim::Seg { a, b, w } => seg(m, f.to_local(*a), f.to_local(*b), f.len(*w) / 2.0, color),
        Prim::Path { pts, w, closed } => {
            let hw = f.len(*w) / 2.0;
            for p in pts.windows(2) {
                seg(m, f.to_local(p[0]), f.to_local(p[1]), hw, color);
            }
            if *closed && pts.len() > 2 {
                seg(
                    m,
                    f.to_local(*pts.last().unwrap()),
                    f.to_local(pts[0]),
                    hw,
                    color,
                );
            }
        }
        Prim::Disc { c, r } => disc(m, f.to_local(*c), f.len(*r), color),
        Prim::Ring { c, r, w } => ring(m, f.to_local(*c), f.len(*r), f.len(*w) / 2.0, color),
        Prim::Poly { outer, holes } => {
            let o: Vec<_> = outer.iter().map(|p| f.to_local(*p)).collect();
            let h: Vec<Vec<_>> = holes
                .iter()
                .map(|h| h.iter().map(|p| f.to_local(*p)).collect())
                .collect();
            polygon(m, &o, &h, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_has_body_and_caps() {
        let mut m = Mesh::default();
        seg(&mut m, (0.0, 0.0), (1.0, 0.0), 0.1, 0xff0000ff);
        assert_eq!(m.indices.len() % 3, 0);
        assert_eq!(m.indices.len() / 3, 2 + 2 * 8);
        assert!(m.indices.iter().all(|&i| i < m.vertex_count()));
    }

    #[test]
    fn polygon_with_hole_triangulates() {
        let mut m = Mesh::default();
        let outer = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let hole = vec![(4.0, 4.0), (6.0, 4.0), (6.0, 6.0), (4.0, 6.0)];
        polygon(&mut m, &outer, &[hole], 1);
        assert_eq!(m.vertex_count(), 8);
        assert_eq!(m.indices.len() / 3, 8);
    }

    #[test]
    fn color_bytes_are_rgba_order() {
        let mut m = Mesh::default();
        disc(&mut m, (0.0, 0.0), 1.0, 0x11223344);
        assert_eq!(&m.bytes[20..24], &[0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn frame_keeps_precision_far_from_origin() {
        let f = Frame {
            origin: Point::new(1_000_000_000_000, 0),
        };
        assert_eq!(f.to_local(Point::new(1_000_000_000_001, 0)).0, 1e-6);
    }

    #[test]
    fn append_offsets_indices() {
        let mut a = Mesh::default();
        disc(&mut a, (0.0, 0.0), 1.0, 1);
        let mut b = Mesh::default();
        disc(&mut b, (0.0, 0.0), 1.0, 1);
        let n = a.vertex_count();
        a.append(&b);
        assert!(a.indices.iter().any(|&i| i >= n));
        assert_eq!(a.position(n), (0.0, 0.0));
    }
}
