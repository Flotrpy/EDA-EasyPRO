//! Gerber X2 (RS-274X) writer.
//!
//! Units are millimetres with six decimals, so a nanometre coordinate is
//! written unchanged.

use easypro_core::font;
use easypro_core::geom::{arc_points, normalize_angle, Point, Rect, Transform};
use easypro_core::layer::{Layer, Side};
use easypro_core::library::{Library, Pad, PadKind, PadShape};
use easypro_core::pcb::Pcb;
use easypro_core::shape::Shape;
use easypro_core::units::mm;
use std::collections::HashMap;
use std::fmt::Write;

fn mm6(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let v = nm.unsigned_abs();
    format!("{sign}{}.{:06}", v / 1_000_000, v % 1_000_000)
}

/// Low-level Gerber builder.
pub struct GerberWriter {
    apertures: Vec<String>,
    lookup: HashMap<String, usize>,
    current: Option<usize>,
    body: String,
    dark: bool,
}

impl Default for GerberWriter {
    fn default() -> Self {
        GerberWriter {
            apertures: vec![],
            lookup: HashMap::new(),
            current: None,
            body: String::new(),
            dark: true,
        }
    }
}

impl GerberWriter {
    fn aperture(&mut self, def: String) -> usize {
        if let Some(&i) = self.lookup.get(&def) {
            return i;
        }
        let i = self.apertures.len();
        self.apertures.push(def.clone());
        self.lookup.insert(def, i);
        i
    }

    fn select(&mut self, ap: usize) {
        if self.current != Some(ap) {
            let _ = writeln!(self.body, "D{}*", ap + 10);
            self.current = Some(ap);
        }
    }

    pub fn polarity(&mut self, dark: bool) {
        if self.dark != dark {
            self.body
                .push_str(if dark { "%LPD*%\n" } else { "%LPC*%\n" });
            self.dark = dark;
        }
    }

    fn xy(p: Point) -> String {
        format!("X{}Y{}", p.x, p.y)
    }

    pub fn circle_ap(&mut self, d: i64) -> usize {
        self.aperture(format!("C,{}", mm6(d.max(1))))
    }

    pub fn flash(&mut self, ap: usize, at: Point) {
        self.select(ap);
        let _ = writeln!(self.body, "{}D03*", Self::xy(at));
    }

    pub fn polyline(&mut self, pts: &[Point], width: i64) {
        if pts.len() < 2 {
            if let Some(p) = pts.first() {
                let ap = self.circle_ap(width);
                self.flash(ap, *p);
            }
            return;
        }
        let ap = self.circle_ap(width);
        self.select(ap);
        let _ = writeln!(self.body, "{}D02*", Self::xy(pts[0]));
        for p in &pts[1..] {
            let _ = writeln!(self.body, "{}D01*", Self::xy(*p));
        }
    }

    pub fn line(&mut self, a: Point, b: Point, width: i64) {
        self.polyline(&[a, b], width);
    }

    /// Filled contour (G36/G37).
    pub fn region(&mut self, pts: &[Point]) {
        if pts.len() < 3 {
            return;
        }
        self.body.push_str("G36*\n");
        let _ = writeln!(self.body, "{}D02*", Self::xy(pts[0]));
        for p in &pts[1..] {
            let _ = writeln!(self.body, "{}D01*", Self::xy(*p));
        }
        let _ = writeln!(self.body, "{}D01*", Self::xy(pts[0]));
        self.body.push_str("G37*\n");
    }

    pub fn shape(&mut self, s: &Shape) {
        match s {
            Shape::Line { a, b, width } => self.line(*a, *b, *width),
            Shape::Polyline { points, width } => self.polyline(points, *width),
            Shape::Rect { a, b, width, fill } => {
                let mut c = Rect::new(*a, *b).corners().to_vec();
                if *fill {
                    self.region(&c);
                }
                c.push(c[0]);
                self.polyline(&c, *width);
            }
            Shape::Circle {
                center,
                radius,
                width,
                fill,
            } => {
                if *fill {
                    let ap = self.circle_ap(radius * 2 + width);
                    self.flash(ap, *center);
                } else {
                    self.polyline(&arc_points(*center, *radius, 0.0, 360.0, 5.0), *width);
                }
            }
            Shape::Arc {
                center,
                radius,
                start,
                sweep,
                width,
            } => self.polyline(&arc_points(*center, *radius, *start, *sweep, 5.0), *width),
            Shape::Polygon {
                points,
                width,
                fill,
            } => {
                if *fill {
                    self.region(points);
                }
                let mut c = points.clone();
                if let Some(f) = c.first().copied() {
                    c.push(f);
                }
                self.polyline(&c, *width);
            }
            Shape::Text {
                pos,
                text,
                size,
                rotation,
                align,
            } => {
                let w = font::stroke_width(*size);
                for stroke in font::text_strokes(text, *size, *pos, *rotation, *align) {
                    self.polyline(&stroke, w);
                }
            }
        }
    }

    /// Draw a pad grown by `grow` on every side (mask expansion).
    pub fn pad(&mut self, pad: &Pad, tf: &Transform, grow: i64) {
        let rot = normalize_angle(pad.rotation + tf.rotation);
        let center = tf.apply(pad.pos);
        let (w, h) = (pad.size.x + 2 * grow, pad.size.y + 2 * grow);
        let right = (rot % 90.0).abs() < 1e-9;
        let (w, h) = if right && (rot == 90.0 || rot == 270.0) {
            (h, w)
        } else {
            (w, h)
        };
        match pad.shape {
            PadShape::Round => {
                let ap = self.circle_ap(w.min(h));
                self.flash(ap, center);
            }
            PadShape::Rect if right => {
                let ap = self.aperture(format!("R,{}X{}", mm6(w), mm6(h)));
                self.flash(ap, center);
            }
            PadShape::Oval if right => {
                let ap = self.aperture(format!("O,{}X{}", mm6(w), mm6(h)));
                self.flash(ap, center);
            }
            _ => {
                let hull = pad.hull(tf).inflate(grow);
                self.region(&hull.to_polygon());
            }
        }
    }

    /// Finish the file with X2 header attributes.
    pub fn finish(&self, file_function: &str, polarity: &str) -> String {
        let mut out = String::new();
        out.push_str("%TF.GenerationSoftware,EasyPRO,EasyPRO,0.1*%\n");
        let _ = writeln!(out, "%TF.FileFunction,{file_function}*%");
        let _ = writeln!(out, "%TF.FilePolarity,{polarity}*%");
        out.push_str("%FSLAX46Y46*%\n%MOMM*%\n%LPD*%\nG01*\n");
        for (i, def) in self.apertures.iter().enumerate() {
            let _ = writeln!(out, "%ADD{}{}*%", i + 10, def);
        }
        out.push_str(&self.body);
        out.push_str("M02*\n");
        out
    }
}

/// Which artwork to produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Artwork {
    Copper(Layer),
    Silk(Side),
    Mask(Side),
    Paste(Side),
    Outline,
}

impl Artwork {
    pub fn file_function(self, copper_layers: u8) -> (String, &'static str) {
        let side = |s: Side| if s == Side::Top { "Top" } else { "Bot" };
        match self {
            Artwork::Copper(l) => {
                let idx = Layer::copper_stack(copper_layers)
                    .iter()
                    .position(|x| *x == l)
                    .unwrap_or(0)
                    + 1;
                let kind = match l {
                    Layer::TopCopper => "Top",
                    Layer::BottomCopper => "Bot",
                    _ => "Inr",
                };
                (format!("Copper,L{idx},{kind}"), "Positive")
            }
            Artwork::Silk(s) => (format!("Legend,{}", side(s)), "Positive"),
            Artwork::Mask(s) => (format!("Soldermask,{}", side(s)), "Negative"),
            Artwork::Paste(s) => (format!("Paste,{}", side(s)), "Positive"),
            Artwork::Outline => ("Profile,NP".into(), "Positive"),
        }
    }
}

/// Render one fabrication layer of a board.
pub fn artwork(pcb: &Pcb, lib: &Library, what: Artwork) -> String {
    let mut g = GerberWriter::default();
    match what {
        Artwork::Copper(layer) => {
            for p in pcb.pours.iter().filter(|p| p.layer == layer) {
                for r in &p.filled {
                    g.polarity(true);
                    g.region(&r.outer);
                    g.polarity(false);
                    for h in &r.holes {
                        g.region(h);
                    }
                }
            }
            g.polarity(true);
            for t in pcb.tracks.iter().filter(|t| t.layer == layer) {
                g.line(t.a, t.b, t.width);
            }
            for v in &pcb.vias {
                let ap = g.circle_ap(v.diameter);
                g.flash(ap, v.pos);
            }
            for c in &pcb.components {
                let Some(fp) = lib.footprints.get(&c.footprint) else {
                    continue;
                };
                let tf = c.transform();
                for pad in fp.pads.iter().filter(|p| p.on_layer(layer, c.side)) {
                    g.pad(pad, &tf, 0);
                }
                for gr in fp
                    .graphics
                    .iter()
                    .filter(|gr| c.board_layer(gr.layer) == layer)
                {
                    g.shape(&gr.shape.transformed(&tf));
                }
            }
            for gr in pcb.graphics.iter().filter(|gr| gr.layer == layer) {
                g.shape(&gr.shape);
            }
        }
        Artwork::Silk(side) => {
            let layer = if side == Side::Top {
                Layer::TopSilk
            } else {
                Layer::BottomSilk
            };
            for c in &pcb.components {
                let Some(fp) = lib.footprints.get(&c.footprint) else {
                    continue;
                };
                let tf = c.transform();
                for gr in fp
                    .graphics
                    .iter()
                    .filter(|gr| c.board_layer(gr.layer) == layer)
                {
                    g.shape(&gr.shape.transformed(&tf));
                }
                if c.show_designator && c.side == side {
                    let size = mm(1.0);
                    let pos = c.designator_pos(fp) - Point::new(0, size / 2);
                    g.shape(&Shape::Text {
                        pos,
                        text: c.designator.clone(),
                        size,
                        rotation: 0.0,
                        align: easypro_core::shape::Align::Center,
                    });
                }
            }
            for gr in pcb.graphics.iter().filter(|gr| gr.layer == layer) {
                g.shape(&gr.shape);
            }
        }
        Artwork::Mask(side) => {
            let copper = if side == Side::Top {
                Layer::TopCopper
            } else {
                Layer::BottomCopper
            };
            for c in &pcb.components {
                let Some(fp) = lib.footprints.get(&c.footprint) else {
                    continue;
                };
                let tf = c.transform();
                for pad in &fp.pads {
                    let on =
                        pad.on_layer(copper, c.side) || matches!(pad.kind, PadKind::Npth { .. });
                    if on {
                        g.pad(pad, &tf, pcb.rules.mask_expansion);
                    }
                }
            }
        }
        Artwork::Paste(side) => {
            let copper = if side == Side::Top {
                Layer::TopCopper
            } else {
                Layer::BottomCopper
            };
            for c in &pcb.components {
                let Some(fp) = lib.footprints.get(&c.footprint) else {
                    continue;
                };
                let tf = c.transform();
                for pad in fp
                    .pads
                    .iter()
                    .filter(|p| p.kind == PadKind::Smd && p.on_layer(copper, c.side))
                {
                    g.pad(pad, &tf, 0);
                }
            }
        }
        Artwork::Outline => {
            if let Some(mut o) = pcb.outline() {
                o.push(o[0]);
                g.polyline(&o, mm(0.1));
            }
        }
    }
    let (func, pol) = what.file_function(pcb.copper_layers);
    g.finish(&func, pol)
}

#[cfg(test)]
mod tests {
    use super::*;
    use easypro_core::examples::blinker;

    #[test]
    fn coordinates_are_exact() {
        assert_eq!(mm6(1_500_000), "1.500000");
        assert_eq!(mm6(-250_000), "-0.250000");
        assert_eq!(mm6(3), "0.000003");
    }

    #[test]
    fn layers_are_well_formed() {
        let p = blinker();
        let b = &p.boards[0];
        let arts = [
            Artwork::Copper(Layer::TopCopper),
            Artwork::Copper(Layer::BottomCopper),
            Artwork::Silk(Side::Top),
            Artwork::Mask(Side::Top),
            Artwork::Paste(Side::Top),
            Artwork::Outline,
        ];
        for a in arts {
            let text = artwork(&b.pcb, &p.library, a);
            assert!(text.starts_with("%TF.GenerationSoftware"), "{a:?}");
            assert!(text.trim_end().ends_with("M02*"), "{a:?}");
            assert_eq!(text.matches("G36*").count(), text.matches("G37*").count());
            // Every aperture used is defined.
            for line in text
                .lines()
                .filter(|l| l.starts_with('D') && l.ends_with('*') && l.len() > 3)
            {
                let n: usize = line[1..line.len() - 1].parse().unwrap();
                assert!(text.contains(&format!("%ADD{n}")), "{a:?}: D{n} undefined");
            }
        }
        let bottom = artwork(&b.pcb, &p.library, Artwork::Copper(Layer::BottomCopper));
        assert!(bottom.contains("%LPC*%"), "pour holes are cleared");
        let top = artwork(&b.pcb, &p.library, Artwork::Copper(Layer::TopCopper));
        assert!(top.contains("D03*"), "pads are flashed");
    }
}
