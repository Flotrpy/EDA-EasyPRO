//! SVG export of any scene (schematic sheets, boards, footprints).

use easypro_core::geom::{Point, Rect};
use easypro_core::scene::{Prim, Scene};
use std::fmt::Write;

fn color(rgba: u32) -> (String, f64) {
    (format!("#{:06x}", rgba >> 8), (rgba & 0xff) as f64 / 255.0)
}

fn n(v: i64) -> String {
    let mm = v as f64 / 1e6;
    let s = format!("{mm:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// Model is Y-up; SVG is Y-down, so every Y is negated.
fn pt(p: Point) -> String {
    format!("{},{}", n(p.x), n(-p.y))
}

fn path_d(pts: &[Point], closed: bool) -> String {
    let mut d = String::new();
    for (i, p) in pts.iter().enumerate() {
        let _ = write!(d, "{}{} ", if i == 0 { "M" } else { "L" }, pt(*p));
    }
    if closed {
        d.push('Z');
    }
    d.trim_end().to_string()
}

/// Render `scene` to SVG with millimetre units. `background` is `0xRRGGBB`.
pub fn scene_to_svg(scene: &Scene, background: Option<u32>, margin: i64) -> String {
    let b: Rect = scene.bbox().inflate(margin);
    let (w, h) = (b.width().max(1), b.height().max(1));
    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}mm" height="{}mm" viewBox="{} {} {} {}">"#,
        n(w),
        n(h),
        n(b.min.x),
        n(-b.max.y),
        n(w),
        n(h)
    );
    if let Some(bg) = background {
        let _ = writeln!(
            out,
            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="#{bg:06x}"/>"##,
            n(b.min.x),
            n(-b.max.y),
            n(w),
            n(h)
        );
    }
    for (layer, items) in &scene.layers {
        let _ = writeln!(out, r#"<g data-layer="{layer:?}">"#);
        for it in items {
            let (c, a) = color(it.color);
            let op = if a < 1.0 {
                format!(r#" opacity="{a:.2}""#)
            } else {
                String::new()
            };
            let stroke = |w: i64| {
                if w == 0 {
                    format!(r#"stroke="{c}" stroke-width="0.1" vector-effect="non-scaling-stroke""#)
                } else {
                    format!(r#"stroke="{c}" stroke-width="{}""#, n(w))
                }
            };
            match &it.prim {
                Prim::Seg { a: p, b: q, w } => {
                    let _ = writeln!(
                        out,
                        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" {} stroke-linecap="round"{op}/>"#,
                        n(p.x),
                        n(-p.y),
                        n(q.x),
                        n(-q.y),
                        stroke(*w)
                    );
                }
                Prim::Path { pts, w, closed } => {
                    let _ = writeln!(
                        out,
                        r#"<path d="{}" fill="none" {} stroke-linecap="round" stroke-linejoin="round"{op}/>"#,
                        path_d(pts, *closed),
                        stroke(*w)
                    );
                }
                Prim::Disc { c: p, r } => {
                    let _ = writeln!(
                        out,
                        r#"<circle cx="{}" cy="{}" r="{}" fill="{c}"{op}/>"#,
                        n(p.x),
                        n(-p.y),
                        n(*r)
                    );
                }
                Prim::Ring { c: p, r, w } => {
                    let _ = writeln!(
                        out,
                        r#"<circle cx="{}" cy="{}" r="{}" fill="none" {}{op}/>"#,
                        n(p.x),
                        n(-p.y),
                        n(*r),
                        stroke(*w)
                    );
                }
                Prim::Poly { outer, holes } => {
                    let mut d = path_d(outer, true);
                    for h in holes {
                        d.push(' ');
                        d.push_str(&path_d(h, true));
                    }
                    let _ = writeln!(out, r#"<path d="{d}" fill="{c}" fill-rule="evenodd"{op}/>"#);
                }
            }
        }
        out.push_str("</g>\n");
    }
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypro_core::examples::blinker;
    use easypro_core::scene::{pcb_scene, sheet_scene, PcbStyle, SchTheme};

    #[test]
    fn exports_sheet_and_board() {
        let p = blinker();
        let b = &p.boards[0];
        let s = sheet_scene(
            &b.schematic.sheets[0],
            &p.library,
            None,
            &SchTheme::LIGHT,
            &p.name,
        );
        let svg = scene_to_svg(&s, Some(0xffffff), 0);
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("<path"));
        let pcb = scene_to_svg(
            &pcb_scene(&b.pcb, &p.library, &PcbStyle::default(), &[]),
            Some(0),
            1_000_000,
        );
        assert!(pcb.contains("fill-rule=\"evenodd\""));
    }

    #[test]
    fn number_formatting() {
        assert_eq!(n(1_500_000), "1.5");
        assert_eq!(n(-2_000_000), "-2");
        assert_eq!(n(0), "0");
    }
}
