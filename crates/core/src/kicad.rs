//! Import KiCad footprints (`.kicad_mod`) and symbols (`.kicad_sym`).
//!
//! KiCad footprints use Y-down coordinates; we flip them. Symbol libraries
//! are already Y-up.

use crate::geom::Point;
use crate::layer::Layer;
use crate::library::{Footprint, FpGraphic, Pad, PadKind, PadShape, Pin, PinKind, Symbol};
use crate::sexpr::{parse, Sexp};
use crate::shape::Shape;
use crate::units::mm;

fn pt(s: &Sexp, flip: bool) -> Option<Point> {
    let (x, y) = (s.num(0)?, s.num(1)?);
    Some(Point::new(mm(x), mm(if flip { -y } else { y })))
}

fn width(s: &Sexp) -> i64 {
    s.get("stroke")
        .and_then(|st| st.get("width"))
        .or_else(|| s.get("width"))
        .and_then(|w| w.num(0))
        .map(mm)
        .unwrap_or(mm(0.12))
}

fn fp_layer(name: &str) -> Option<Layer> {
    Some(match name {
        "F.SilkS" | "F.Silkscreen" => Layer::TopSilk,
        "B.SilkS" | "B.Silkscreen" => Layer::BottomSilk,
        "F.Fab" => Layer::TopAssembly,
        "B.Fab" => Layer::BottomAssembly,
        "F.CrtYd" | "F.Courtyard" | "Dwgs.User" | "Cmts.User" => Layer::Document,
        "F.Cu" => Layer::TopCopper,
        "B.Cu" => Layer::BottomCopper,
        _ => return None,
    })
}

/// Parse a `.kicad_mod` footprint.
pub fn import_footprint(text: &str) -> Result<Footprint, String> {
    let root = parse(text)?;
    if !matches!(root.head(), Some("footprint") | Some("module")) {
        return Err("not a KiCad footprint (expected (footprint ...))".into());
    }
    let name = root.arg(0).unwrap_or("Imported").to_string();
    let mut f = Footprint {
        id: format!("fp:{name}"),
        name,
        pads: vec![],
        graphics: vec![],
    };
    for item in root.items() {
        match item.head() {
            Some("pad") => {
                if let Some(p) = pad(item) {
                    f.pads.push(p);
                }
            }
            Some(h @ ("fp_line" | "fp_rect" | "fp_circle" | "fp_arc" | "fp_poly" | "fp_text")) => {
                let Some(layer) = item.get("layer").and_then(|l| l.arg(0)).and_then(fp_layer)
                else {
                    continue;
                };
                if let Some(shape) = graphic(h, item) {
                    f.graphics.push(FpGraphic { layer, shape });
                }
            }
            _ => {}
        }
    }
    if f.pads.is_empty() && f.graphics.is_empty() {
        return Err("footprint has no pads or graphics".into());
    }
    Ok(f)
}

fn pad(s: &Sexp) -> Option<Pad> {
    let number = s.arg(0)?.to_string();
    let kind_s = s.arg(1)?;
    let shape_s = s.arg(2)?;
    let at = s.get("at")?;
    let pos = pt(at, true)?;
    let rotation = at.num(2).unwrap_or(0.0);
    let size = s.get("size")?;
    let size = Point::new(mm(size.num(0)?), mm(size.num(1)?));
    let drill = s
        .get("drill")
        .and_then(|d| {
            d.items()
                .iter()
                .skip(1)
                .find_map(|a| a.atom()?.parse::<f64>().ok())
        })
        .map(mm);
    let shape = match shape_s {
        "rect" => PadShape::Rect,
        "circle" => PadShape::Round,
        "oval" => PadShape::Oval,
        "roundrect" => {
            let r = s
                .get("roundrect_rratio")
                .and_then(|r| r.num(0))
                .unwrap_or(0.25);
            PadShape::RoundRect {
                ratio_pct: (r * 100.0).round().clamp(0.0, 50.0) as u8,
            }
        }
        _ => PadShape::Rect,
    };
    let kind = match kind_s {
        // Pads that only list mask/paste layers are fiducials or aperture
        // helpers; still import them as SMD so nothing is silently lost.
        "smd" => PadKind::Smd,
        "thru_hole" => PadKind::Tht {
            drill: drill.unwrap_or(mm(0.8)),
        },
        "np_thru_hole" => PadKind::Npth {
            drill: drill.unwrap_or(size.x),
        },
        "connect" => PadKind::Smd,
        _ => return None,
    };
    Some(Pad {
        number,
        pos,
        size,
        shape,
        kind,
        rotation,
    })
}

fn graphic(head: &str, s: &Sexp) -> Option<Shape> {
    let w = width(s);
    let fill = s
        .get("fill")
        .and_then(|f| f.arg(0))
        .is_some_and(|v| v == "solid" || v == "yes");
    Some(match head {
        "fp_line" => Shape::Line {
            a: pt(s.get("start")?, true)?,
            b: pt(s.get("end")?, true)?,
            width: w,
        },
        "fp_rect" => Shape::Rect {
            a: pt(s.get("start")?, true)?,
            b: pt(s.get("end")?, true)?,
            width: w,
            fill,
        },
        "fp_circle" => {
            let c = pt(s.get("center")?, true)?;
            let e = pt(s.get("end")?, true)?;
            Shape::Circle {
                center: c,
                radius: c.dist(e).round() as i64,
                width: w,
                fill,
            }
        }
        "fp_arc" => arc_from(s, true, w)?,
        "fp_poly" => Shape::Polygon {
            points: s
                .get("pts")?
                .all("xy")
                .filter_map(|p| pt(p, true))
                .collect(),
            width: w,
            fill: true,
        },
        "fp_text" => {
            let kind = s.arg(0)?;
            let text = match kind {
                "reference" | "value" => return None,
                _ => s.arg(1)?.to_string(),
            };
            let at = s.get("at")?;
            let size = s
                .get("effects")
                .and_then(|e| e.get("font"))
                .and_then(|f| f.get("size"))
                .and_then(|z| z.num(1))
                .unwrap_or(1.0);
            Shape::Text {
                pos: pt(at, true)?,
                text,
                size: mm(size),
                rotation: at.num(2).unwrap_or(0.0),
                align: crate::shape::Align::Center,
            }
        }
        _ => return None,
    })
}

/// KiCad 6+ arcs are start/mid/end; older files use start(center)/end/angle.
fn arc_from(s: &Sexp, flip: bool, w: i64) -> Option<Shape> {
    if let Some(mid) = s.get("mid") {
        let a = pt(s.get("start")?, flip)?;
        let m = pt(mid, flip)?;
        let b = pt(s.get("end")?, flip)?;
        let (c, r) = circumcircle(a, m, b)?;
        let ang = |p: Point| ((p.y - c.y) as f64).atan2((p.x - c.x) as f64).to_degrees();
        let (sa, ma, ea) = (ang(a), ang(m), ang(b));
        let norm = |x: f64| x.rem_euclid(360.0);
        let ccw_sweep = norm(ea - sa);
        let sweep = if norm(ma - sa) <= ccw_sweep {
            ccw_sweep
        } else {
            ccw_sweep - 360.0
        };
        Some(Shape::Arc {
            center: c,
            radius: r,
            start: sa,
            sweep,
            width: w,
        })
    } else {
        let c = pt(s.get("start")?, flip)?;
        let e = pt(s.get("end")?, flip)?;
        let angle = s.get("angle")?.num(0)?;
        let start = ((e.y - c.y) as f64).atan2((e.x - c.x) as f64).to_degrees();
        Some(Shape::Arc {
            center: c,
            radius: c.dist(e).round() as i64,
            start,
            sweep: if flip { -angle } else { angle },
            width: w,
        })
    }
}

fn circumcircle(a: Point, b: Point, c: Point) -> Option<(Point, i64)> {
    let (ax, ay) = a.to_f();
    let (bx, by) = b.to_f();
    let (cx, cy) = c.to_f();
    let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d.abs() < 1e-9 {
        return None;
    }
    let ux = ((ax * ax + ay * ay) * (by - cy)
        + (bx * bx + by * by) * (cy - ay)
        + (cx * cx + cy * cy) * (ay - by))
        / d;
    let uy = ((ax * ax + ay * ay) * (cx - bx)
        + (bx * bx + by * by) * (ax - cx)
        + (cx * cx + cy * cy) * (bx - ax))
        / d;
    let center = Point::from_f(ux, uy);
    Some((center, center.dist(a).round() as i64))
}

fn pin_kind(s: &str) -> PinKind {
    match s {
        "input" => PinKind::Input,
        "output" => PinKind::Output,
        "bidirectional" | "tri_state" => PinKind::Bidirectional,
        "power_in" => PinKind::PowerIn,
        "power_out" => PinKind::PowerOut,
        "open_collector" | "open_emitter" => PinKind::OpenCollector,
        "no_connect" | "unconnected" => PinKind::NotConnected,
        _ => PinKind::Passive,
    }
}

/// Parse every symbol in a `.kicad_sym` library.
pub fn import_symbols(text: &str) -> Result<Vec<Symbol>, String> {
    let root = parse(text)?;
    if root.head() != Some("kicad_symbol_lib") {
        return Err("not a KiCad symbol library (expected (kicad_symbol_lib ...))".into());
    }
    let mut out = Vec::new();
    for sym in root.all("symbol") {
        let name = sym.arg(0).unwrap_or("Imported").to_string();
        let hide_names = sym
            .get("pin_names")
            .is_some_and(|p| p.has_atom("hide") || p.get("hide").is_some());
        let mut s = Symbol {
            id: format!("sym:{name}"),
            name: name.clone(),
            pins: vec![],
            graphics: vec![],
        };
        // Units are nested symbols "<name>_<unit>_<style>"; take unit 0/1, style 1.
        let mut bodies: Vec<&Sexp> = vec![sym];
        bodies.extend(sym.all("symbol").filter(|u| {
            let n = u.arg(0).unwrap_or("");
            n.ends_with("_0_1") || n.ends_with("_1_1") || n.ends_with("_0_0") || n.ends_with("_1_0")
        }));
        for body in bodies {
            for it in body.items() {
                match it.head() {
                    Some("pin") => {
                        let at = it.get("at").ok_or("pin without position")?;
                        let number = it
                            .get("number")
                            .and_then(|n| n.arg(0))
                            .unwrap_or("")
                            .to_string();
                        let mut pname = it
                            .get("name")
                            .and_then(|n| n.arg(0))
                            .unwrap_or("")
                            .to_string();
                        if pname == "~" {
                            pname = String::new();
                        }
                        s.pins.push(Pin {
                            number,
                            name: pname.clone(),
                            pos: pt(at, false).ok_or("bad pin position")?,
                            angle: at.num(2).unwrap_or(0.0),
                            length: it
                                .get("length")
                                .and_then(|l| l.num(0))
                                .map(mm)
                                .unwrap_or(mm(2.54)),
                            kind: pin_kind(it.arg(0).unwrap_or("passive")),
                            hide_name: hide_names || pname.is_empty(),
                        });
                    }
                    Some("rectangle") => {
                        let fill = it
                            .get("fill")
                            .and_then(|f| f.get("type"))
                            .and_then(|t| t.arg(0))
                            .is_some_and(|t| t == "outline");
                        if let (Some(a), Some(b)) = (
                            it.get("start").and_then(|p| pt(p, false)),
                            it.get("end").and_then(|p| pt(p, false)),
                        ) {
                            s.graphics.push(Shape::Rect {
                                a,
                                b,
                                width: width(it).max(mm(0.15)),
                                fill,
                            });
                        }
                    }
                    Some("polyline") => {
                        let pts: Vec<Point> = it
                            .get("pts")
                            .map(|p| p.all("xy").filter_map(|x| pt(x, false)).collect())
                            .unwrap_or_default();
                        if pts.len() >= 2 {
                            let fill = it
                                .get("fill")
                                .and_then(|f| f.get("type"))
                                .and_then(|t| t.arg(0))
                                .is_some_and(|t| t == "outline");
                            if fill && pts.len() >= 3 {
                                s.graphics.push(Shape::Polygon {
                                    points: pts,
                                    width: width(it).max(mm(0.15)),
                                    fill: true,
                                });
                            } else {
                                s.graphics.push(Shape::Polyline {
                                    points: pts,
                                    width: width(it).max(mm(0.15)),
                                });
                            }
                        }
                    }
                    Some("circle") => {
                        if let (Some(c), Some(r)) = (
                            it.get("center").and_then(|p| pt(p, false)),
                            it.get("radius").and_then(|r| r.num(0)),
                        ) {
                            s.graphics.push(Shape::Circle {
                                center: c,
                                radius: mm(r),
                                width: width(it).max(mm(0.15)),
                                fill: false,
                            });
                        }
                    }
                    Some("arc") => {
                        if let Some(a) = arc_from(it, false, width(it).max(mm(0.15))) {
                            s.graphics.push(a);
                        }
                    }
                    _ => {}
                }
            }
        }
        out.push(s);
    }
    if out.is_empty() {
        return Err("library contains no symbols".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FP: &str = r#"(footprint "R_0603_1608Metric" (version 20211014) (layer "F.Cu")
  (fp_line (start -0.16 -0.51) (end 0.16 -0.51) (stroke (width 0.12) (type solid)) (layer "F.SilkS"))
  (fp_rect (start -1.48 -0.73) (end 1.48 0.73) (stroke (width 0.05) (type solid)) (fill none) (layer "F.CrtYd"))
  (fp_arc (start 1 0) (mid 0 1) (end -1 0) (stroke (width 0.1) (type solid)) (layer "F.Fab"))
  (pad "1" smd roundrect (at -0.825 0) (size 0.8 0.95) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
  (pad "2" smd roundrect (at 0.825 0 90) (size 0.8 0.95) (layers "F.Cu" "F.Paste" "F.Mask") (roundrect_rratio 0.25))
  (pad "3" thru_hole circle (at 0 2.54) (size 1.7 1.7) (drill 1) (layers "*.Cu" "*.Mask"))
)"#;

    #[test]
    fn imports_footprint() {
        let f = import_footprint(FP).unwrap();
        assert_eq!(f.name, "R_0603_1608Metric");
        assert_eq!(f.pads.len(), 3);
        assert_eq!(f.pads[0].shape, PadShape::RoundRect { ratio_pct: 25 });
        assert_eq!(f.pads[1].rotation, 90.0);
        assert_eq!(f.pads[2].kind, PadKind::Tht { drill: mm(1.0) });
        assert_eq!(f.pads[2].pos, Point::new(0, mm(-2.54)), "Y is flipped");
        assert_eq!(f.graphics.len(), 3);
        if let Shape::Arc { radius, sweep, .. } = &f.graphics[2].shape {
            assert_eq!(*radius, mm(1.0));
            assert!((sweep.abs() - 180.0).abs() < 1e-6);
        } else {
            panic!("expected arc");
        }
    }

    const SYM: &str = r#"(kicad_symbol_lib (version 20211014) (generator kicad_symbol_editor)
  (symbol "LED" (pin_numbers hide) (pin_names (offset 1.016) hide) (in_bom yes) (on_board yes)
    (property "Reference" "D" (id 0) (at 0 2.54 0))
    (symbol "LED_0_1"
      (polyline (pts (xy -1.27 -1.27) (xy -1.27 1.27)) (stroke (width 0.254) (type default)) (fill (type none)))
      (rectangle (start -1 -1) (end 1 1) (stroke (width 0.254)) (fill (type none)))
    )
    (symbol "LED_1_1"
      (pin passive line (at -3.81 0 0) (length 2.54) (name "K" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
      (pin power_in line (at 3.81 0 180) (length 2.54) (name "A" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))
    )
  )
)"#;

    #[test]
    fn imports_symbols() {
        let s = import_symbols(SYM).unwrap();
        assert_eq!(s.len(), 1);
        let led = &s[0];
        assert_eq!(led.pins.len(), 2);
        assert_eq!(led.pins[1].kind, PinKind::PowerIn);
        assert_eq!(led.pins[1].angle, 180.0);
        assert!(led.pins[0].hide_name);
        assert_eq!(led.graphics.len(), 2);
    }

    #[test]
    fn rejects_other_files() {
        assert!(import_footprint("(kicad_pcb)").is_err());
        assert!(import_symbols("(footprint \"x\")").is_err());
    }
}
