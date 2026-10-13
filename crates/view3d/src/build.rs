//! Turn a board into a 3D mesh.

use crate::mesh::{Mesh3, STRIDE, V3};
use easypro_core::geom::{Point, Rect};
use easypro_core::layer::{Layer, Side};
use easypro_core::library::{Body, Footprint, Hull, Library, Model3d, PadKind};
use easypro_core::pcb::{Pcb, PcbComponent};
use easypro_core::scene::{pcb_scene, PcbStyle, Prim, SceneLayer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View3dOptions {
    pub components: bool,
    /// Surface finish: true for ENIG gold, false for HASL silver.
    pub gold: bool,
    pub silk: bool,
}

impl Default for View3dOptions {
    fn default() -> Self {
        View3dOptions {
            components: true,
            gold: true,
            silk: true,
        }
    }
}

const FR4: u32 = 0xc9b98cff;
const SILK: u32 = 0xf4f4f0ff;
const HOLE: u32 = 0x161616ff;
const METAL: u32 = 0xc8c8ccff;
const PIN_GOLD: u32 = 0xd8b048ff;

fn rgb(c: u32) -> u32 {
    (c << 8) | 0xff
}

fn lighten(c: u32, k: f32) -> u32 {
    let r = ((c >> 24) & 0xff) as f32;
    let g = ((c >> 16) & 0xff) as f32;
    let b = ((c >> 8) & 0xff) as f32;
    let l = |v: f32| (v + (255.0 - v) * k).min(255.0) as u32;
    (l(r) << 24) | (l(g) << 16) | (l(b) << 8) | 0xff
}

struct Ctx {
    center: Point,
}

impl Ctx {
    fn p(&self, p: Point) -> [f32; 2] {
        [
            ((p.x - self.center.x) as f64 / 1e6) as f32,
            ((p.y - self.center.y) as f64 / 1e6) as f32,
        ]
    }

    fn poly(&self, pts: &[Point]) -> Vec<[f32; 2]> {
        pts.iter().map(|p| self.p(*p)).collect()
    }
}

fn prim_polys(prim: &Prim) -> Vec<(Vec<Point>, Vec<Vec<Point>>)> {
    let min_w = 50_000;
    match prim {
        Prim::Seg { a, b, w } => vec![(
            Hull::capsule(*a, *b, (*w).max(min_w) / 2).to_polygon(),
            vec![],
        )],
        Prim::Path { pts, w, closed } => {
            let mut v: Vec<_> = pts
                .windows(2)
                .map(|s| {
                    (
                        Hull::capsule(s[0], s[1], (*w).max(min_w) / 2).to_polygon(),
                        vec![],
                    )
                })
                .collect();
            if *closed && pts.len() > 2 {
                v.push((
                    Hull::capsule(*pts.last().unwrap(), pts[0], (*w).max(min_w) / 2).to_polygon(),
                    vec![],
                ));
            }
            v
        }
        Prim::Disc { c, r } => vec![(Hull::circle(*c, *r).to_polygon(), vec![])],
        Prim::Ring { c, r, w } => {
            vec![(
                Hull::circle(*c, r + w / 2).to_polygon(),
                vec![Hull::circle(*c, (r - w / 2).max(0)).to_polygon()],
            )]
        }
        Prim::Poly { outer, holes } => vec![(outer.clone(), holes.clone())],
    }
}

/// Build the complete mesh. Returns the mesh and the radius of a sphere
/// containing the board (for fitting the camera).
pub fn board_mesh(pcb: &Pcb, lib: &Library, opts: &View3dOptions) -> (Mesh3, f32) {
    let outline = pcb.outline().unwrap_or_else(|| {
        let b = pcb.bbox(lib);
        let b = if b.is_empty() {
            Rect::around(Point::ZERO, 10_000_000, 10_000_000)
        } else {
            b
        };
        b.corners().to_vec()
    });
    let bb = Rect::from_points(outline.iter().copied());
    let ctx = Ctx {
        center: bb.center(),
    };
    let t = (pcb.thickness as f64 / 1e6) as f32;
    let mut m = Mesh3::default();

    let o = ctx.poly(&outline);
    m.flat(&o, &[], t, true, FR4);
    m.flat(&o, &[], 0.0, false, FR4);
    m.walls(&o, 0.0, t, FR4);

    let mask = rgb(pcb.mask_color);
    let mask_copper = lighten(mask, 0.18);
    let finish = if opts.gold { 0xd9b55aff } else { 0xc4c6caff };
    let style = PcbStyle {
        hidden: vec![],
        highlight_net: None,
        show_ratsnest: false,
    };
    let scene = pcb_scene(pcb, lib, &style, &[]);

    for side in [Side::Top, Side::Bottom] {
        let up = side == Side::Top;
        let z = |dz: f32| if up { t + dz } else { -dz };
        let copper = if up {
            Layer::TopCopper
        } else {
            Layer::BottomCopper
        };
        let silk = if up {
            Layer::TopSilk
        } else {
            Layer::BottomSilk
        };
        m.flat(&o, &[], z(0.010), up, mask);
        for (key, items) in &scene.layers {
            let (dz, color) = match key {
                SceneLayer::Pcb(l) if *l == copper => (0.020, mask_copper),
                SceneLayer::Pcb(l) if *l == silk && opts.silk => (0.040, SILK),
                _ => continue,
            };
            for it in items {
                // Pads are drawn below as exposed metal.
                if matches!(key, SceneLayer::Pcb(l) if l.is_copper())
                    && it.owner.is_some_and(|id| pcb.component(id).is_some())
                {
                    continue;
                }
                for (outer, holes) in prim_polys(&it.prim) {
                    let h: Vec<_> = holes.iter().map(|h| ctx.poly(h)).collect();
                    m.flat(&ctx.poly(&outer), &h, z(dz), up, color);
                }
            }
        }
        for c in &pcb.components {
            let Some(fp) = lib.footprints.get(&c.footprint) else {
                continue;
            };
            let tf = c.transform();
            for pad in &fp.pads {
                let exposed = match pad.kind {
                    PadKind::Smd => c.side == side,
                    PadKind::Tht { .. } => true,
                    PadKind::Npth { .. } => false,
                };
                if exposed {
                    m.flat(
                        &ctx.poly(&pad.hull(&tf).to_polygon()),
                        &[],
                        z(0.030),
                        up,
                        finish,
                    );
                }
                if let Some(d) = pad.drill() {
                    m.disc(
                        ctx.p(tf.apply(pad.pos)),
                        (d as f64 / 2e6) as f32,
                        z(0.050),
                        up,
                        HOLE,
                    );
                }
            }
        }
        for v in &pcb.vias {
            m.disc(
                ctx.p(v.pos),
                (v.diameter as f64 / 2e6) as f32,
                z(0.025),
                up,
                mask_copper,
            );
            m.disc(
                ctx.p(v.pos),
                (v.drill as f64 / 2e6) as f32,
                z(0.050),
                up,
                HOLE,
            );
        }
    }

    if opts.components {
        for c in &pcb.components {
            let Some(fp) = lib.footprints.get(&c.footprint) else {
                continue;
            };
            let Some(model) = lib.devices.get(&c.device).and_then(|d| d.model.as_ref()) else {
                continue;
            };
            let local = body_mesh(model, fp, t);
            m.append(&place(&local, c, &ctx, t));
        }
    }

    let half = [
        (m.max[0] - m.min[0]) / 2.0,
        (m.max[1] - m.min[1]) / 2.0,
        (m.max[2] - m.min[2]) / 2.0,
    ];
    let radius = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2])
        .sqrt()
        .max(5.0);
    (m, radius)
}

fn mm(nm: i64) -> f32 {
    (nm as f64 / 1e6) as f32
}

/// Body in footprint coordinates; z = 0 is the board surface the part sits on.
pub fn body_mesh(model: &Model3d, fp: &Footprint, board_t: f32) -> Mesh3 {
    let mut m = Mesh3::default();
    let (sx, sy, h) = (mm(model.size.x), mm(model.size.y), mm(model.height));
    let so = mm(model.standoff);
    let (ox, oy) = (mm(model.offset.x), mm(model.offset.y));
    let color = rgb(model.color);
    let (x0, x1, y0, y1) = (ox - sx / 2.0, ox + sx / 2.0, oy - sy / 2.0, oy + sy / 2.0);
    let tht_leads = |m: &mut Mesh3, top: f32| {
        for pad in fp
            .pads
            .iter()
            .filter(|p| matches!(p.kind, PadKind::Tht { .. }))
        {
            let (px, py) = (mm(pad.pos.x), mm(pad.pos.y));
            m.cuboid(
                [px - 0.25, py - 0.25, -board_t - 1.5],
                [px + 0.25, py + 0.25, top],
                METAL,
            );
        }
    };
    match &model.body {
        Body::Chip => {
            let cap = (sx * 0.2).min(0.5);
            m.cuboid([x0 + cap, y0, so], [x1 - cap, y1, h], color);
            m.cuboid([x0, y0, so], [x0 + cap, y1, h], METAL);
            m.cuboid([x1 - cap, y0, so], [x1, y1, h], METAL);
        }
        Body::Ic => {
            let z0 = so.max(0.1);
            m.cuboid([x0, y0, z0], [x1, y1, h], color);
            for pad in &fp.pads {
                let (px, py) = (mm(pad.pos.x), mm(pad.pos.y));
                if px > x0 && px < x1 && py > y0 && py < y1 {
                    continue; // exposed pad / tab under the body
                }
                let (ex, ey) = (px.clamp(x0, x1), py.clamp(y0, y1));
                let lw = (mm(pad.size.x.min(pad.size.y)) * 0.6).max(0.15) / 2.0;
                let rise = (h * 0.45).min(0.7);
                if (ex - px).abs() > (ey - py).abs() {
                    m.cuboid(
                        [px.min(ex), py - lw, 0.0],
                        [px.max(ex), py + lw, 0.15],
                        METAL,
                    );
                    m.cuboid([ex - 0.08, py - lw, 0.0], [ex + 0.08, py + lw, rise], METAL);
                } else {
                    m.cuboid(
                        [px - lw, py.min(ey), 0.0],
                        [px + lw, py.max(ey), 0.15],
                        METAL,
                    );
                    m.cuboid([px - lw, ey - 0.08, 0.0], [px + lw, ey + 0.08, rise], METAL);
                }
            }
            if let Some(p1) = fp.pad("1") {
                let (px, py) = (mm(p1.pos.x), mm(p1.pos.y));
                let inset = (sx.min(sy) * 0.18).max(0.25);
                let dx = px.clamp(x0 + inset, x1 - inset);
                let dy = py.clamp(y0 + inset, y1 - inset);
                m.disc(
                    [dx, dy],
                    (sx.min(sy) * 0.07).max(0.12),
                    h + 0.002,
                    true,
                    0x606060ff,
                );
            }
        }
        Body::Dip => {
            m.cuboid([x0, y0, 0.5], [x1, y1, h], color);
            for pad in fp
                .pads
                .iter()
                .filter(|p| matches!(p.kind, PadKind::Tht { .. }))
            {
                let (px, py) = (mm(pad.pos.x), mm(pad.pos.y));
                let ex = px.clamp(x0, x1);
                m.cuboid(
                    [px - 0.25, py - 0.13, -board_t - 1.5],
                    [px + 0.25, py + 0.13, 1.0],
                    METAL,
                );
                m.cuboid(
                    [px.min(ex), py - 0.13, 0.75],
                    [px.max(ex), py + 0.13, 1.0],
                    METAL,
                );
            }
        }
        Body::Header => {
            for pad in fp
                .pads
                .iter()
                .filter(|p| matches!(p.kind, PadKind::Tht { .. }))
            {
                let (px, py) = (mm(pad.pos.x), mm(pad.pos.y));
                m.cuboid(
                    [px - 1.27, py - 1.27, 0.0],
                    [px + 1.27, py + 1.27, 2.5],
                    color,
                );
                m.cuboid(
                    [px - 0.32, py - 0.32, -board_t - 3.0],
                    [px + 0.32, py + 0.32, h],
                    PIN_GOLD,
                );
            }
        }
        Body::Cylinder => {
            let r = sx.min(sy) / 2.0;
            m.cylinder([ox, oy], r, so.max(0.3), h, color, 0xb8b8bcff);
            tht_leads(&mut m, so.max(0.3));
        }
        Body::Led => {
            let r = sx.min(sy) / 2.0;
            let lens = (color & 0xffffff00) | 0xd8;
            m.cylinder([ox, oy], r * 1.1, 1.0, 2.0, lens, lens);
            m.cylinder([ox, oy], r, 2.0, (h - r).max(2.1), lens, lens);
            m.dome([ox, oy], r, (h - r).max(2.1), lens);
            tht_leads(&mut m, 1.0);
        }
        Body::Box => {
            m.cuboid([x0, y0, so], [x1, y1, h], color);
            tht_leads(&mut m, so);
        }
        Body::Mesh {
            vertices,
            triangles,
        } => {
            for t in triangles {
                let v: Vec<V3> = t
                    .iter()
                    .filter_map(|&i| vertices.get(i as usize))
                    .map(|p| [mm(p[0]), mm(p[1]), mm(p[2])])
                    .collect();
                if v.len() == 3 {
                    let a = [v[1][0] - v[0][0], v[1][1] - v[0][1], v[1][2] - v[0][2]];
                    let b = [v[2][0] - v[0][0], v[2][1] - v[0][1], v[2][2] - v[0][2]];
                    let n = crate::math::normalize([
                        a[1] * b[2] - a[2] * b[1],
                        a[2] * b[0] - a[0] * b[2],
                        a[0] * b[1] - a[1] * b[0],
                    ]);
                    let i0 = m.vertex(v[0], n, color);
                    let i1 = m.vertex(v[1], n, color);
                    let i2 = m.vertex(v[2], n, color);
                    m.indices.extend([i0, i1, i2]);
                }
            }
        }
    }
    m
}

/// Move a body from footprint space onto the board.
fn place(local: &Mesh3, c: &PcbComponent, ctx: &Ctx, t: f32) -> Mesh3 {
    let tf = c.transform();
    let top = c.side == Side::Top;
    let (s, co) = tf.rotation.to_radians().sin_cos();
    let (s, co) = (s as f32, co as f32);
    let mx = if tf.mirror { -1.0 } else { 1.0 };
    let off = ctx.p(tf.offset);
    let mut out = local.clone();
    out.min = [f32::MAX; 3];
    out.max = [f32::MIN; 3];
    for i in 0..local.vertex_count() as usize {
        let o = i * STRIDE;
        let rd = |k: usize| f32::from_le_bytes(local.bytes[o + k..o + k + 4].try_into().unwrap());
        let (px, py, pz) = (rd(0), rd(4), rd(8));
        let (nx, ny, nz) = (rd(12), rd(16), rd(20));
        let rx = px * co - py * s;
        let ry = px * s + py * co;
        let p = [
            rx * mx + off[0],
            ry + off[1],
            if top { t + pz } else { -pz },
        ];
        let n = [
            (nx * co - ny * s) * mx,
            nx * s + ny * co,
            if top { nz } else { -nz },
        ];
        for (k, v) in p.iter().chain(n.iter()).enumerate() {
            out.bytes[o + k * 4..o + k * 4 + 4].copy_from_slice(&v.to_le_bytes());
        }
        for k in 0..3 {
            out.min[k] = out.min[k].min(p[k]);
            out.max[k] = out.max[k].max(p[k]);
        }
    }
    out
}

/// Parse a Wavefront OBJ (positions in millimetres) into a mesh body.
pub fn parse_obj(text: &str) -> Result<Body, String> {
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => {
                let c: Vec<f64> = it.take(3).filter_map(|s| s.parse().ok()).collect();
                if c.len() != 3 {
                    return Err(format!("bad vertex line: {line}"));
                }
                vertices.push([
                    (c[0] * 1e6) as i64,
                    (c[1] * 1e6) as i64,
                    (c[2] * 1e6) as i64,
                ]);
            }
            Some("f") => {
                let idx: Vec<u32> = it
                    .filter_map(|s| s.split('/').next()?.parse::<i64>().ok())
                    .map(|i| {
                        if i < 0 {
                            (vertices.len() as i64 + i) as u32
                        } else {
                            (i - 1) as u32
                        }
                    })
                    .collect();
                for k in 1..idx.len().saturating_sub(1) {
                    triangles.push([idx[0], idx[k], idx[k + 1]]);
                }
            }
            _ => {}
        }
    }
    if triangles.is_empty() {
        return Err("no faces found in OBJ file".into());
    }
    Ok(Body::Mesh {
        vertices,
        triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypro_core::examples::blinker;

    #[test]
    fn example_board_builds() {
        let p = blinker();
        let (m, r) = board_mesh(&p.boards[0].pcb, &p.library, &View3dOptions::default());
        assert!(m.indices.len() > 1000);
        assert!(m.indices.iter().all(|&i| i < m.vertex_count()));
        assert!(r > 25.0 && r < 60.0, "radius {r}");
        assert!(m.max[2] > 8.0, "header pins stick up");
        assert!(m.min[2] < -1.0, "pins poke through the bottom");
    }

    #[test]
    fn obj_import() {
        let b = parse_obj("v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n").unwrap();
        match b {
            Body::Mesh {
                vertices,
                triangles,
            } => {
                assert_eq!(vertices.len(), 4);
                assert_eq!(triangles, vec![[0, 1, 2], [0, 2, 3]]);
            }
            _ => panic!(),
        }
        assert!(parse_obj("hello").is_err());
    }
}
