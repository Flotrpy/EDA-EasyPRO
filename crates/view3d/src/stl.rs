//! Binary STL export of a board mesh, for checking fit in enclosure CAD.

use crate::mesh::{Mesh3, STRIDE};

fn pos(m: &Mesh3, i: u32) -> [f32; 3] {
    let o = i as usize * STRIDE;
    let f = |k: usize| f32::from_le_bytes(m.bytes[o + k..o + k + 4].try_into().unwrap());
    [f(0), f(4), f(8)]
}

/// Millimetre units, as most CAD tools assume for STL.
pub fn to_stl(m: &Mesh3, name: &str) -> Vec<u8> {
    let tris = m.indices.len() / 3;
    let mut out = Vec::with_capacity(84 + tris * 50);
    let mut header = [0u8; 80];
    let h = format!("EasyPRO board: {name}");
    header[..h.len().min(80)].copy_from_slice(&h.as_bytes()[..h.len().min(80)]);
    out.extend(header);
    out.extend((tris as u32).to_le_bytes());
    for t in m.indices.chunks(3) {
        let (a, b, c) = (pos(m, t[0]), pos(m, t[1]), pos(m, t[2]));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = crate::math::normalize([
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]);
        for p in [n, a, b, c] {
            for x in p {
                out.extend(x.to_le_bytes());
            }
        }
        out.extend(0u16.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stl_size_matches_triangle_count() {
        let mut m = Mesh3::default();
        m.cuboid([0.0; 3], [1.0, 2.0, 3.0], 0xffffffff);
        let stl = to_stl(&m, "box");
        assert_eq!(stl.len(), 84 + 12 * 50);
        assert_eq!(u32::from_le_bytes(stl[80..84].try_into().unwrap()), 12);
        assert!(stl.starts_with(b"EasyPRO board: box"));
    }
}
