//! Schematic documents: sheets with parts, wires, labels and ports.

use crate::geom::{Point, Rect, Transform};
use crate::id::Id;
use crate::library::{Library, Symbol};
use crate::shape::Shape;
use crate::units::mil;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Default schematic snap grid (50 mil, pins sit on 100 mil).
pub const SCH_GRID: i64 = 1_270_000;

/// A placed symbol instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub id: Id,
    pub device: String,
    pub designator: String,
    pub value: String,
    pub pos: Point,
    #[serde(default)]
    pub rotation: f64,
    #[serde(default)]
    pub mirror: bool,
    /// Per-instance overrides of device attributes.
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
    /// Leave out of BOM and PCB (e.g. logos, test notes).
    #[serde(default)]
    pub exclude_from_bom: bool,
    /// Label offsets relative to `pos`, in sheet orientation.
    #[serde(default)]
    pub designator_offset: Option<Point>,
    #[serde(default)]
    pub value_offset: Option<Point>,
}

impl Component {
    pub fn transform(&self) -> Transform {
        Transform::new(self.pos, self.rotation, self.mirror)
    }

    /// World positions of each pin's connection point.
    pub fn pin_positions<'a>(
        &'a self,
        sym: &'a Symbol,
    ) -> impl Iterator<Item = (&'a str, Point)> + 'a {
        let tf = self.transform();
        sym.pins
            .iter()
            .map(move |p| (p.number.as_str(), tf.apply(p.pos)))
    }

    pub fn bbox(&self, lib: &Library) -> Rect {
        let Some(sym) = lib
            .devices
            .get(&self.device)
            .and_then(|d| lib.symbols.get(&d.symbol))
        else {
            return Rect::around(self.pos, mil(50.0), mil(50.0));
        };
        let b = sym.bbox();
        let tf = self.transform();
        Rect::from_points(b.corners().iter().map(|c| tf.apply(*c)))
    }

    pub fn attr<'a>(&'a self, key: &str, lib: &'a Library) -> Option<&'a str> {
        self.attributes
            .get(key)
            .or_else(|| {
                lib.devices
                    .get(&self.device)
                    .and_then(|d| d.attributes.get(key))
            })
            .map(|s| s.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wire {
    pub id: Id,
    pub points: Vec<Point>,
}

impl Wire {
    pub fn segments(&self) -> impl Iterator<Item = (Point, Point)> + '_ {
        self.points.windows(2).map(|w| (w[0], w[1]))
    }
}

/// A net label: wires touching labels with the same name are connected,
/// across all sheets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetLabel {
    pub id: Id,
    pub name: String,
    pub pos: Point,
    #[serde(default)]
    pub rotation: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortStyle {
    /// Bar symbol for supply rails (VCC, +3V3).
    Power,
    /// Ground symbol.
    Ground,
    /// Off-sheet connector arrow.
    Port,
}

/// Power/ground flags and sheet ports. Like labels, they connect by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetPort {
    pub id: Id,
    pub name: String,
    pub pos: Point,
    pub style: PortStyle,
    #[serde(default)]
    pub rotation: f64,
}

/// Marks a pin as intentionally unconnected for ERC.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoConnect {
    pub id: Id,
    pub pos: Point,
}

/// Free annotation graphics (text, lines, boxes).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: Id,
    pub shape: Shape,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperSize {
    A4,
    #[default]
    A3,
    A2,
    Letter,
}

impl PaperSize {
    pub const ALL: [PaperSize; 4] = [
        PaperSize::A4,
        PaperSize::A3,
        PaperSize::A2,
        PaperSize::Letter,
    ];

    /// Landscape size in nm.
    pub fn size(self) -> Point {
        use crate::units::mm;
        match self {
            PaperSize::A4 => Point::new(mm(297.0), mm(210.0)),
            PaperSize::A3 => Point::new(mm(420.0), mm(297.0)),
            PaperSize::A2 => Point::new(mm(594.0), mm(420.0)),
            PaperSize::Letter => Point::new(mm(279.4), mm(215.9)),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PaperSize::A4 => "A4",
            PaperSize::A3 => "A3",
            PaperSize::A2 => "A2",
            PaperSize::Letter => "Letter",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub paper: PaperSize,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub wires: Vec<Wire>,
    #[serde(default)]
    pub labels: Vec<NetLabel>,
    #[serde(default)]
    pub ports: Vec<NetPort>,
    #[serde(default)]
    pub no_connects: Vec<NoConnect>,
    #[serde(default)]
    pub annotations: Vec<Annotation>,
}

impl Sheet {
    /// The drawing frame. The sheet origin is the frame's bottom-left.
    pub fn frame(&self) -> Rect {
        Rect::new(Point::ZERO, self.paper.size())
    }

    pub fn component(&self, id: Id) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }

    pub fn component_mut(&mut self, id: Id) -> Option<&mut Component> {
        self.components.iter_mut().find(|c| c.id == id)
    }

    /// Remove any object with this id. Returns whether something was removed.
    pub fn remove(&mut self, id: Id) -> bool {
        let before = self.object_count();
        self.components.retain(|o| o.id != id);
        self.wires.retain(|o| o.id != id);
        self.labels.retain(|o| o.id != id);
        self.ports.retain(|o| o.id != id);
        self.no_connects.retain(|o| o.id != id);
        self.annotations.retain(|o| o.id != id);
        self.object_count() != before
    }

    pub fn object_count(&self) -> usize {
        self.components.len()
            + self.wires.len()
            + self.labels.len()
            + self.ports.len()
            + self.no_connects.len()
            + self.annotations.len()
    }

    pub fn max_id(&self) -> Id {
        let ids = self
            .components
            .iter()
            .map(|o| o.id)
            .chain(self.wires.iter().map(|o| o.id))
            .chain(self.labels.iter().map(|o| o.id))
            .chain(self.ports.iter().map(|o| o.id))
            .chain(self.no_connects.iter().map(|o| o.id))
            .chain(self.annotations.iter().map(|o| o.id));
        ids.chain(std::iter::once(self.id))
            .max()
            .unwrap_or_default()
    }

    /// Clean up wires: drop zero-length segments and merge collinear runs.
    pub fn tidy_wires(&mut self) {
        for w in &mut self.wires {
            w.points.dedup();
            let mut i = 1;
            while i + 1 < w.points.len() {
                let (a, b, c) = (w.points[i - 1], w.points[i], w.points[i + 1]);
                let cross = (b.x - a.x) as i128 * (c.y - b.y) as i128
                    - (b.y - a.y) as i128 * (c.x - b.x) as i128;
                let dot = (b.x - a.x) as i128 * (c.x - b.x) as i128
                    + (b.y - a.y) as i128 * (c.y - b.y) as i128;
                if cross == 0 && dot >= 0 {
                    w.points.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        self.wires.retain(|w| w.points.len() >= 2);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Schematic {
    pub sheets: Vec<Sheet>,
}

impl Schematic {
    pub fn components(&self) -> impl Iterator<Item = (usize, &Component)> {
        self.sheets
            .iter()
            .enumerate()
            .flat_map(|(i, s)| s.components.iter().map(move |c| (i, c)))
    }

    pub fn find_component(&self, id: Id) -> Option<&Component> {
        self.sheets.iter().find_map(|s| s.component(id))
    }

    pub fn find_component_mut(&mut self, id: Id) -> Option<&mut Component> {
        self.sheets.iter_mut().find_map(|s| s.component_mut(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_merges_collinear_points() {
        let mut s = Sheet::default();
        s.wires.push(Wire {
            id: Id(1),
            points: vec![
                Point::new(0, 0),
                Point::new(5, 0),
                Point::new(5, 0),
                Point::new(10, 0),
                Point::new(10, 5),
            ],
        });
        s.wires.push(Wire {
            id: Id(2),
            points: vec![Point::new(1, 1), Point::new(1, 1)],
        });
        s.tidy_wires();
        assert_eq!(s.wires.len(), 1);
        assert_eq!(
            s.wires[0].points,
            vec![Point::new(0, 0), Point::new(10, 0), Point::new(10, 5)]
        );
    }

    #[test]
    fn remove_by_id() {
        let mut s = Sheet::default();
        s.labels.push(NetLabel {
            id: Id(4),
            name: "SDA".into(),
            pos: Point::ZERO,
            rotation: 0.0,
        });
        assert!(s.remove(Id(4)));
        assert!(!s.remove(Id(4)));
    }
}
