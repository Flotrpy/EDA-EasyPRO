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

/// World position and net of a placed pad (example builder).
fn pad_at(p: &Project, des: &str, pad: &str) -> (Point, String) {
    let c = p.boards[0]
        .pcb
        .components
        .iter()
        .find(|c| c.designator == des)
        .expect("part");
    let fp = &p.library.footprints[&c.footprint];
    let at = fp.pads.iter().find(|x| x.number == pad).expect("pad").pos;
    (c.transform().apply(at), c.net_of(pad).to_string())
}

/// Lay a 45° track through `points`, all on one net and layer.
fn track(p: &mut Project, net: &str, width: i64, points: &[Point]) {
    for s in points.windows(2) {
        let id = p.next_id();
        p.boards[0].pcb.tracks.push(crate::pcb::Track {
            id,
            layer: Layer::TopCopper,
            a: s[0],
            b: s[1],
            width,
            net: net.into(),
        });
    }
}

/// Drop a SMD pad to the bottom GND pour through a via at `offset`.
fn ground_via(p: &mut Project, des: &str, pad: &str, offset: Point) {
    let (at, _) = pad_at(p, des, pad);
    let id = p.next_id();
    p.boards[0].pcb.vias.push(crate::pcb::Via {
        id,
        pos: at + offset,
        diameter: mm(0.6),
        drill: mm(0.3),
        net: "GND".into(),
    });
    track(p, "GND", mm(0.4), &[at, at + offset]);
}

/// AMS1117 3.3 V regulator with a power LED, fully routed on a 45 x 22 mm board.
pub fn regulator() -> Project {
    let mut b = Builder {
        p: Project::new("3.3V Regulator"),
    };
    b.p.description =
        "AMS1117-3.3 LDO with input and output headers and a power LED. Fully routed.".into();
    b.p.boards[0].schematic.sheets[0].paper = crate::schematic::PaperSize::A4;
    let j1 = b.part("dev:HDR-1x2", "5V IN", 3000.0, 6000.0, 0.0);
    let c1 = b.part("dev:C-0805", "10uF", 4400.0, 6000.0, 0.0);
    let u1 = b.part("dev:AMS1117-3.3", "AMS1117-3.3", 5800.0, 6000.0, 0.0);
    let c2 = b.part("dev:C-0805", "22uF", 7200.0, 6000.0, 0.0);
    let r1 = b.part("dev:R-0603", "1k", 8200.0, 6600.0, 0.0);
    let d1 = b.part("dev:LED-0805", "Green", 8200.0, 5400.0, 0.0);
    let j2 = b.part("dev:HDR-1x2", "3V3 OUT", 9400.0, 6000.0, 0.0);
    for (c, pin, net) in [
        (j1, "1", "VIN"),
        (j1, "2", "GND"),
        (c1, "1", "VIN"),
        (c1, "2", "GND"),
        (u1, "3", "VIN"),
        (u1, "1", "GND"),
        (u1, "2", "+3V3"),
        (c2, "1", "+3V3"),
        (c2, "2", "GND"),
        (r1, "1", "+3V3"),
        (r1, "2", "LED_A"),
        (d1, "2", "LED_A"),
        (d1, "1", "GND"),
        (j2, "1", "+3V3"),
        (j2, "2", "GND"),
    ] {
        b.tie(c, pin, net);
    }

    let mut p = b.p;
    let lib = p.library.clone();
    annotate(&mut p.boards[0], &lib, true);
    update_pcb(&mut p.boards[0], &lib, &mut p.ids);
    let place = [
        ("J1", 5.0, 11.0, 0.0),
        ("C1", 12.0, 13.22, 90.0),
        ("U1", 22.0, 11.0, 180.0),
        ("C2", 31.0, 8.8, 90.0),
        ("R1", 33.0, 4.0, 0.0),
        ("LED1", 37.0, 4.0, 180.0),
        ("J2", 40.0, 11.0, 0.0),
    ];
    for c in &mut p.boards[0].pcb.components {
        if let Some((_, x, y, r)) = place.iter().find(|(d, ..)| *d == c.designator) {
            c.pos = Point::new(mm(*x), mm(*y));
            c.rotation = *r;
        }
    }
    let board = crate::geom::Rect::new(Point::ZERO, Point::new(mm(45.0), mm(22.0)));
    crate::board::set_rect_outline(&mut p.boards[0].pcb, board, mm(2.0), &mut p.ids);

    // Hand routing (mm): input along the top, output along the tab side.
    let pt = |x: f64, y: f64| Point::new(mm(x), mm(y));
    let power = mm(0.6);
    track(
        &mut p,
        "VIN",
        power,
        &[pt(5.0, 12.27), pt(17.82, 12.27), pt(19.7, 14.15)],
    );
    track(
        &mut p,
        "+3V3",
        power,
        &[pt(22.0, 7.85), pt(35.58, 7.85), pt(40.0, 12.27)],
    );
    // SOT-223 pin 2 and the tab are the same node; tie them under the body.
    track(&mut p, "+3V3", power, &[pt(22.0, 14.15), pt(22.0, 7.85)]);
    track(&mut p, "+3V3", mm(0.4), &[pt(32.2, 7.85), pt(32.2, 4.0)]);
    track(&mut p, "LED_A", mm(0.4), &[pt(33.8, 4.0), pt(36.05, 4.0)]);
    for (des, pad, dx, dy) in [
        ("C1", "2", 0.0, 1.6),
        ("U1", "1", 0.0, 1.8),
        ("C2", "2", 0.0, 1.6),
        ("LED1", "1", 1.6, 0.0),
    ] {
        ground_via(&mut p, des, pad, pt(dx, dy));
    }

    let pour_id = p.next_id();
    let pcb = &mut p.boards[0].pcb;
    pcb.pours.push(Pour {
        id: pour_id,
        layer: Layer::BottomCopper,
        net: "GND".into(),
        outline: board.corners().to_vec(),
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
    vec![
        ("555 LED Blinker", blinker),
        ("3.3V Regulator (routed)", regulator),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::Severity;

    #[test]
    fn regulator_is_routed_and_drc_clean() {
        let p = regulator();
        let b = &p.boards[0];
        let erc = crate::erc::run(&b.schematic, &p.library);
        assert!(
            erc.iter().all(|i| i.severity != Severity::Error),
            "{erc:#?}"
        );
        let drc = crate::drc::run(&b.pcb, &p.library, crate::units::Unit::Mm);
        assert!(drc.is_empty(), "{drc:#?}");
    }

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
