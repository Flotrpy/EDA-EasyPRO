//! Example projects shown on the start page.

use crate::builtin;
use crate::geom::{rotate, Point};
use crate::layer::Layer;
use crate::pcb::Pour;
use crate::project::Project;
use crate::schematic::{Component, NetLabel, NetPort, PortStyle, Wire};
use crate::sync::{annotate, update_pcb};
use crate::units::{mil, mm};

struct Builder {
    p: Project,
}

impl Builder {
    fn part(&mut self, device: &str, value: &str, x: f64, y: f64, rotation: f64) -> crate::id::Id {
        self.p.library.import_device(builtin::library(), device);
        let id = self.p.next_id();
        self.p.boards[0].schematic.sheets[0]
            .components
            .push(Component {
                id,
                device: device.into(),
                designator: format!("{}?", builtin::library().devices[device].prefix),
                value: value.into(),
                // Centred on an A4 sheet.
                pos: Point::new(mil(x + 850.0), mil(y - 1800.0)),
                rotation,
                mirror: false,
                attributes: Default::default(),
                exclude_from_bom: false,
                designator_offset: None,
                value_offset: None,
            });
        id
    }

    /// Connect a pin to a named net with a short wire and a label or port.
    fn tie(&mut self, comp: crate::id::Id, pin: &str, net: &str) {
        let lib = &self.p.library;
        let sheet = &self.p.boards[0].schematic.sheets[0];
        let c = sheet.component(comp).expect("component");
        let sym = &lib.symbols[&lib.devices[&c.device].symbol];
        let pinref = sym.pin(pin).expect("pin");
        let tf = c.transform();
        let at = tf.apply(pinref.pos);
        // Stub points away from the body.
        let dir = rotate(Point::new(-mil(200.0), 0), tf.apply_angle(pinref.angle));
        let end = at + dir;
        let wid = self.p.next_id();
        let lid = self.p.next_id();
        let sheet = &mut self.p.boards[0].schematic.sheets[0];
        sheet.wires.push(Wire {
            id: wid,
            points: vec![at, end],
        });
        let rotation = if dir.x < 0 {
            180.0
        } else if dir.y < 0 {
            270.0
        } else if dir.y > 0 {
            90.0
        } else {
            0.0
        };
        // The IC's pins are 100 mil apart: labels fit there, flags do not.
        if sheet
            .component(comp)
            .is_some_and(|c| c.device.starts_with("dev:NE555"))
        {
            sheet.labels.push(NetLabel {
                id: lid,
                name: net.into(),
                pos: end,
                rotation,
            });
            return;
        }
        match net {
            "VCC" => sheet.ports.push(NetPort {
                id: lid,
                name: net.into(),
                pos: end,
                style: PortStyle::Power,
                rotation: 0.0,
            }),
            "GND" => sheet.ports.push(NetPort {
                id: lid,
                name: net.into(),
                pos: end,
                style: PortStyle::Ground,
                rotation: 0.0,
            }),
            _ => sheet.labels.push(NetLabel {
                id: lid,
                name: net.into(),
                pos: end,
                rotation,
            }),
        }
    }
}

/// Classic NE555 astable LED blinker on a 50 x 40 mm two-layer board.
pub fn blinker() -> Project {
    let mut b = Builder {
        p: Project::new("555 LED Blinker"),
    };
    b.p.description = "NE555 astable oscillator driving an LED at about 1.4 Hz.".into();
    b.p.boards[0].schematic.sheets[0].paper = crate::schematic::PaperSize::A4;
    let u1 = b.part("dev:NE555-SOIC", "NE555", 5000.0, 6000.0, 0.0);
    let r1 = b.part("dev:R-0603", "10k", 7000.0, 6600.0, 0.0);
    let r2 = b.part("dev:R-0603", "47k", 7000.0, 6000.0, 0.0);
    let r3 = b.part("dev:R-0603", "1k", 7000.0, 5400.0, 0.0);
    let c1 = b.part("dev:C-0805", "10uF", 3000.0, 6600.0, 0.0);
    let c2 = b.part("dev:C-0603", "10nF", 3000.0, 6000.0, 0.0);
    let d1 = b.part("dev:LED-0805", "Red", 3000.0, 5400.0, 0.0);
    let j1 = b.part("dev:HDR-1x2", "5V IN", 3000.0, 4600.0, 0.0);
    let c3 = b.part("dev:C-0603", "100nF", 7000.0, 4600.0, 0.0);

    for (pin, net) in [
        ("1", "GND"),
        ("2", "THR"),
        ("3", "OUT"),
        ("4", "VCC"),
        ("5", "CV"),
        ("6", "THR"),
        ("7", "DIS"),
        ("8", "VCC"),
    ] {
        b.tie(u1, pin, net);
    }
    b.tie(r1, "1", "VCC");
    b.tie(r1, "2", "DIS");
    b.tie(r2, "1", "DIS");
    b.tie(r2, "2", "THR");
    b.tie(r3, "1", "OUT");
    b.tie(r3, "2", "LED_A");
    b.tie(c1, "1", "THR");
    b.tie(c1, "2", "GND");
    b.tie(c2, "1", "CV");
    b.tie(c2, "2", "GND");
    b.tie(d1, "2", "LED_A");
    b.tie(d1, "1", "GND");
    b.tie(j1, "1", "VCC");
    b.tie(j1, "2", "GND");
    b.tie(c3, "1", "VCC");
    b.tie(c3, "2", "GND");

    let mut p = b.p;
    let lib = p.library.clone();
    annotate(&mut p.boards[0], &lib, true);
    update_pcb(&mut p.boards[0], &lib, &mut p.ids);

    // Hand placement inside the 50 x 40 mm outline.
    let place = [
        ("U1", 25.0, 22.0, 0.0),
        ("R1", 36.0, 28.0, 90.0),
        ("R2", 39.0, 22.0, 90.0),
        ("R3", 36.0, 14.0, 0.0),
        ("C1", 14.0, 22.0, 90.0),
        ("C2", 18.0, 14.0, 0.0),
        ("LED1", 42.0, 10.0, 0.0),
        ("J1", 6.0, 20.0, 0.0),
        ("C3", 25.0, 30.0, 0.0),
    ];
    for c in &mut p.boards[0].pcb.components {
        if let Some((_, x, y, r)) = place.iter().find(|(d, ..)| *d == c.designator) {
            c.pos = Point::new(mm(*x), mm(*y));
            c.rotation = *r;
        }
    }
    let pour_id = p.next_id();
    let pcb = &mut p.boards[0].pcb;
    pcb.pours.push(Pour {
        id: pour_id,
        layer: Layer::BottomCopper,
        net: "GND".into(),
        outline: crate::geom::Rect::new(Point::ZERO, Point::new(mm(50.0), mm(40.0)))
            .corners()
            .to_vec(),
        clearance: None,
        remove_islands: true,
        filled: vec![],
    });
    pcb.rebuild_pours(&lib);
    p
}

/// An example: display name and constructor.
pub type Example = (&'static str, fn() -> Project);

pub fn all() -> Vec<Example> {
    vec![("555 LED Blinker", blinker)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Severity;

    #[test]
    fn blinker_is_clean_and_synced() {
        let p = blinker();
        let b = &p.boards[0];
        let erc = crate::erc::run(&b.schematic, &p.library);
        assert!(
            erc.iter().all(|i| i.severity != Severity::Error),
            "{erc:#?}"
        );
        assert_eq!(b.pcb.components.len(), 9);
        let nets = b.pcb.nets();
        for n in ["VCC", "GND", "THR", "DIS", "OUT", "CV", "LED_A"] {
            assert!(nets.iter().any(|x| x == n), "missing {n} in {nets:?}");
        }
        assert!(!b.pcb.pours[0].filled.is_empty());
        let drc = crate::drc::run(&b.pcb, &p.library, crate::units::Unit::Mm);
        assert!(
            !drc.iter()
                .any(|i| i.message.starts_with("Short") || i.message.contains("outside")),
            "{drc:#?}"
        );
        // Round trip through the file format.
        assert_eq!(crate::project::Project::from_json(&p.to_json()).unwrap(), p);
    }
}
