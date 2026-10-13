//! Reusable parts: symbols, footprints, 3D bodies and the devices that tie
//! them together (Device = Symbol + Footprint + 3D model + parameters).

use crate::geom::{circle_polygon, rotate, Point, Rect, Transform};
use crate::layer::{Layer, Side};
use crate::shape::Shape;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Electrical type of a symbol pin, used by the electrical rule check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinKind {
    Input,
    Output,
    Bidirectional,
    #[default]
    Passive,
    PowerIn,
    PowerOut,
    OpenCollector,
    NotConnected,
}

impl PinKind {
    pub const ALL: [PinKind; 8] = [
        PinKind::Input,
        PinKind::Output,
        PinKind::Bidirectional,
        PinKind::Passive,
        PinKind::PowerIn,
        PinKind::PowerOut,
        PinKind::OpenCollector,
        PinKind::NotConnected,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PinKind::Input => "Input",
            PinKind::Output => "Output",
            PinKind::Bidirectional => "Bidirectional",
            PinKind::Passive => "Passive",
            PinKind::PowerIn => "Power In",
            PinKind::PowerOut => "Power Out",
            PinKind::OpenCollector => "Open Collector",
            PinKind::NotConnected => "Not Connected",
        }
    }
}

/// A symbol pin. `pos` is the electrical connection point; the pin line
/// extends `length` from there towards the body in direction `angle`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub number: String,
    pub name: String,
    pub pos: Point,
    pub angle: f64,
    pub length: i64,
    #[serde(default)]
    pub kind: PinKind,
    #[serde(default)]
    pub hide_name: bool,
}

impl Pin {
    pub fn body_end(&self) -> Point {
        self.pos + rotate(Point::new(self.length, 0), self.angle)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub pins: Vec<Pin>,
    pub graphics: Vec<Shape>,
}

impl Symbol {
    pub fn bbox(&self) -> Rect {
        let mut r = Rect::EMPTY;
        for g in &self.graphics {
            r = r.union(g.bbox());
        }
        for p in &self.pins {
            r.include(p.pos);
            r.include(p.body_end());
        }
        r
    }

    pub fn pin(&self, number: &str) -> Option<&Pin> {
        self.pins.iter().find(|p| p.number == number)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadShape {
    Rect,
    Round,
    Oval,
    /// Rounded rectangle; corner radius is `ratio` (0..0.5) of the short side.
    RoundRect {
        ratio_pct: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PadKind {
    /// Surface mount pad on the footprint's own side.
    Smd,
    /// Plated through hole.
    Tht { drill: i64 },
    /// Non-plated hole (mounting holes).
    Npth { drill: i64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pad {
    pub number: String,
    pub pos: Point,
    pub size: Point,
    pub shape: PadShape,
    pub kind: PadKind,
    #[serde(default)]
    pub rotation: f64,
}

/// Copper outline of a pad: a polygon/segment/point hull plus a radius.
/// The real pad is every point within `radius` of the hull.
#[derive(Clone, Debug, PartialEq)]
pub struct Hull {
    pub points: Vec<Point>,
    pub radius: i64,
}

impl Hull {
    pub fn circle(c: Point, r: i64) -> Hull {
        Hull {
            points: vec![c],
            radius: r,
        }
    }

    pub fn capsule(a: Point, b: Point, r: i64) -> Hull {
        Hull {
            points: vec![a, b],
            radius: r,
        }
    }

    pub fn polygon(points: Vec<Point>) -> Hull {
        Hull { points, radius: 0 }
    }

    pub fn bbox(&self) -> Rect {
        Rect::from_points(self.points.iter().copied()).inflate(self.radius)
    }

    /// Distance between the two copper areas (0 if they overlap).
    pub fn distance(&self, o: &Hull) -> f64 {
        use crate::geom::*;
        let base = match (self.points.len(), o.points.len()) {
            (1, 1) => self.points[0].dist(o.points[0]),
            (1, 2) => point_segment_dist(self.points[0], o.points[0], o.points[1]),
            (2, 1) => point_segment_dist(o.points[0], self.points[0], self.points[1]),
            (2, 2) => {
                segment_segment_dist(self.points[0], self.points[1], o.points[0], o.points[1])
            }
            (1, _) => point_polygon_dist(self.points[0], &o.points),
            (_, 1) => point_polygon_dist(o.points[0], &self.points),
            (2, _) => segment_polygon_dist(self.points[0], self.points[1], &o.points),
            (_, 2) => segment_polygon_dist(o.points[0], o.points[1], &self.points),
            _ => polygon_polygon_dist(&self.points, &o.points),
        };
        (base - self.radius as f64 - o.radius as f64).max(0.0)
    }

    pub fn contains(&self, p: Point) -> bool {
        self.distance(&Hull::circle(p, 0)) == 0.0
    }

    /// Polygon approximation (for filling and rendering).
    pub fn to_polygon(&self) -> Vec<Point> {
        match self.points.len() {
            0 => vec![],
            1 => circle_polygon(self.points[0], self.radius, 32),
            2 if self.radius > 0 => {
                let (a, b) = (self.points[0], self.points[1]);
                let ang = ((b.y - a.y) as f64).atan2((b.x - a.x) as f64).to_degrees();
                let mut v = crate::geom::arc_points(b, self.radius, ang - 90.0, 180.0, 15.0);
                v.extend(crate::geom::arc_points(
                    a,
                    self.radius,
                    ang + 90.0,
                    180.0,
                    15.0,
                ));
                v
            }
            _ if self.radius > 0 => crate::pour::offset_polygon(&self.points, self.radius),
            _ => self.points.clone(),
        }
    }

    pub fn inflate(&self, d: i64) -> Hull {
        Hull {
            points: self.points.clone(),
            radius: self.radius + d,
        }
    }
}

impl Pad {
    pub fn drill(&self) -> Option<i64> {
        match self.kind {
            PadKind::Smd => None,
            PadKind::Tht { drill } | PadKind::Npth { drill } => Some(drill),
        }
    }

    pub fn is_plated(&self) -> bool {
        !matches!(self.kind, PadKind::Npth { .. })
    }

    /// Copper hull of this pad after placing the footprint with `tf`.
    pub fn hull(&self, tf: &Transform) -> Hull {
        let local = Transform::new(self.pos, self.rotation, false);
        let place = |p: Point| tf.apply(local.apply(p));
        let (w, h) = (self.size.x, self.size.y);
        match self.shape {
            PadShape::Round => Hull::circle(place(Point::ZERO), w.min(h) / 2),
            PadShape::Oval => {
                if w == h {
                    Hull::circle(place(Point::ZERO), w / 2)
                } else if w > h {
                    let d = (w - h) / 2;
                    Hull::capsule(place(Point::new(-d, 0)), place(Point::new(d, 0)), h / 2)
                } else {
                    let d = (h - w) / 2;
                    Hull::capsule(place(Point::new(0, -d)), place(Point::new(0, d)), w / 2)
                }
            }
            PadShape::Rect => Hull::polygon(
                Rect::around(Point::ZERO, w / 2, h / 2)
                    .corners()
                    .iter()
                    .map(|c| place(*c))
                    .collect(),
            ),
            PadShape::RoundRect { ratio_pct } => {
                let r = w.min(h) * ratio_pct.min(50) as i64 / 100;
                let pts = Rect::around(Point::ZERO, w / 2 - r, h / 2 - r)
                    .corners()
                    .iter()
                    .map(|c| place(*c))
                    .collect();
                Hull {
                    points: pts,
                    radius: r,
                }
            }
        }
    }

    /// Copper layers this pad appears on for a footprint placed on `side`.
    pub fn on_layer(&self, layer: Layer, side: Side) -> bool {
        match self.kind {
            PadKind::Smd => match side {
                Side::Top => layer == Layer::TopCopper,
                Side::Bottom => layer == Layer::BottomCopper,
            },
            PadKind::Tht { .. } => layer.is_copper(),
            PadKind::Npth { .. } => false,
        }
    }
}

/// A graphic item that belongs to a footprint layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FpGraphic {
    pub layer: Layer,
    pub shape: Shape,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Footprint {
    pub id: String,
    pub name: String,
    pub pads: Vec<Pad>,
    pub graphics: Vec<FpGraphic>,
}

impl Footprint {
    pub fn bbox(&self) -> Rect {
        let mut r = Rect::EMPTY;
        for p in &self.pads {
            r = r.union(p.hull(&Transform::default()).bbox());
        }
        for g in &self.graphics {
            r = r.union(g.shape.bbox());
        }
        r
    }

    pub fn pad(&self, number: &str) -> Option<&Pad> {
        self.pads.iter().find(|p| p.number == number)
    }
}

/// How the 3D viewer builds a component body. Leads are derived from the
/// footprint's pads so bodies always line up with the copper.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Body {
    /// Ceramic chip resistor/capacitor with metal end terminals.
    Chip,
    /// Moulded IC package with gull-wing or J leads to each SMD pad.
    Ic,
    /// Dual in-line package with leads down through each hole.
    Dip,
    /// Pin header: plastic spacer and a square pin per pad.
    Header,
    /// Upright cylinder (electrolytic caps, buzzers, crystals HC-49).
    Cylinder,
    /// Round LED with a domed lens.
    Led,
    /// Plain box (modules, connectors, switches).
    Box,
    /// Triangle mesh imported from an OBJ file. Coordinates in nm.
    Mesh {
        vertices: Vec<[i64; 3]>,
        triangles: Vec<[u32; 3]>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model3d {
    pub body: Body,
    /// Body footprint on the board (x, y) and height.
    pub size: Point,
    pub height: i64,
    /// Lift above the board surface (e.g. standoffs).
    #[serde(default)]
    pub standoff: i64,
    /// Body colour `0xRRGGBB`.
    pub color: u32,
    /// Offset of the body centre from the footprint origin.
    #[serde(default)]
    pub offset: Point,
}

/// A placeable part: what you search for in the library panel.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    /// Designator prefix, e.g. `R`, `C`, `U`.
    pub prefix: String,
    pub symbol: String,
    #[serde(default)]
    pub footprint: Option<String>,
    #[serde(default)]
    pub model: Option<Model3d>,
    #[serde(default)]
    pub value: String,
    /// Manufacturer, MPN, supplier part numbers, datasheet URL, ...
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
    /// Symbol pin number -> footprint pad number, when they differ.
    #[serde(default)]
    pub pin_map: BTreeMap<String, String>,
}

impl Device {
    pub fn pad_for_pin<'a>(&'a self, pin: &'a str) -> &'a str {
        self.pin_map.get(pin).map(|s| s.as_str()).unwrap_or(pin)
    }

    pub fn pin_for_pad<'a>(&'a self, pad: &'a str) -> &'a str {
        self.pin_map
            .iter()
            .find(|(_, v)| v.as_str() == pad)
            .map(|(k, _)| k.as_str())
            .unwrap_or(pad)
    }
}

/// A collection of parts. Projects carry their own copy so they open
/// identically anywhere, even offline.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    #[serde(default)]
    pub symbols: BTreeMap<String, Symbol>,
    #[serde(default)]
    pub footprints: BTreeMap<String, Footprint>,
    #[serde(default)]
    pub devices: BTreeMap<String, Device>,
}

impl Library {
    pub fn add_symbol(&mut self, s: Symbol) {
        self.symbols.insert(s.id.clone(), s);
    }

    pub fn add_footprint(&mut self, f: Footprint) {
        self.footprints.insert(f.id.clone(), f);
    }

    pub fn add_device(&mut self, d: Device) {
        self.devices.insert(d.id.clone(), d);
    }

    /// Copy a device and the symbol/footprint it references from `src`.
    pub fn import_device(&mut self, src: &Library, id: &str) -> bool {
        let Some(d) = src.devices.get(id) else {
            return false;
        };
        if let Some(s) = src.symbols.get(&d.symbol) {
            self.symbols
                .entry(s.id.clone())
                .or_insert_with(|| s.clone());
        }
        if let Some(f) = d.footprint.as_ref().and_then(|f| src.footprints.get(f)) {
            self.footprints
                .entry(f.id.clone())
                .or_insert_with(|| f.clone());
        }
        self.devices
            .entry(d.id.clone())
            .or_insert_with(|| d.clone());
        true
    }

    /// Case-insensitive search over name, value, category and attributes.
    pub fn search(&self, query: &str) -> Vec<&Device> {
        let terms: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();
        let mut hits: Vec<&Device> = self
            .devices
            .values()
            .filter(|d| {
                let hay = format!(
                    "{} {} {} {} {} {}",
                    d.name,
                    d.value,
                    d.category,
                    d.description,
                    d.footprint.as_deref().unwrap_or(""),
                    d.attributes.values().cloned().collect::<Vec<_>>().join(" ")
                )
                .to_lowercase();
                terms.iter().all(|t| hay.contains(t.as_str()))
            })
            .collect();
        hits.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        hits
    }

    /// Check that every reference resolves. Returns human readable problems.
    pub fn validate(&self) -> Vec<String> {
        let mut out = Vec::new();
        for d in self.devices.values() {
            let Some(sym) = self.symbols.get(&d.symbol) else {
                out.push(format!("{}: missing symbol {}", d.id, d.symbol));
                continue;
            };
            if let Some(fid) = &d.footprint {
                let Some(fp) = self.footprints.get(fid) else {
                    out.push(format!("{}: missing footprint {fid}", d.id));
                    continue;
                };
                for pin in &sym.pins {
                    let pad = d.pad_for_pin(&pin.number);
                    if fp.pad(pad).is_none() {
                        out.push(format!(
                            "{}: pin {} has no pad {pad} in {fid}",
                            d.id, pin.number
                        ));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::mm;

    fn pad(shape: PadShape, w: f64, h: f64) -> Pad {
        Pad {
            number: "1".into(),
            pos: Point::ZERO,
            size: Point::new(mm(w), mm(h)),
            shape,
            kind: PadKind::Smd,
            rotation: 0.0,
        }
    }

    #[test]
    fn pad_hulls() {
        let tf = Transform::default();
        let r = pad(PadShape::Rect, 2.0, 1.0).hull(&tf);
        assert!(r.contains(Point::new(mm(0.99), mm(0.49))));
        assert!(!r.contains(Point::new(mm(1.01), 0)));
        let o = pad(PadShape::Oval, 3.0, 1.0).hull(&tf);
        assert!(o.contains(Point::new(mm(1.49), 0)));
        assert!(!o.contains(Point::new(mm(1.49), mm(0.45))));
        let c = pad(PadShape::Round, 1.0, 1.0).hull(&tf);
        assert_eq!(
            c.distance(&Hull::circle(Point::new(mm(2.0), 0), mm(0.5))),
            mm(1.0) as f64
        );
    }

    #[test]
    fn hull_distance_with_rotation() {
        let mut p = pad(PadShape::Rect, 2.0, 1.0);
        p.rotation = 90.0;
        let h = p.hull(&Transform::default());
        assert!(h.contains(Point::new(0, mm(0.99))));
        assert!(!h.contains(Point::new(mm(0.99), 0)));
    }

    #[test]
    fn pin_mapping_defaults_to_identity() {
        let mut d = Device::default();
        assert_eq!(d.pad_for_pin("3"), "3");
        d.pin_map.insert("A".into(), "1".into());
        assert_eq!(d.pad_for_pin("A"), "1");
        assert_eq!(d.pin_for_pad("1"), "A");
    }
}
