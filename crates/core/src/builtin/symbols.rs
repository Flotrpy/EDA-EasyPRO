//! Schematic symbols for the built-in library. Pins sit on a 100 mil grid.

use crate::geom::Point;
use crate::library::{Pin, PinKind, Symbol};
use crate::shape::Shape;
use crate::units::mil;

/// Body stroke width (6 mil).
const W: i64 = 152_400;

fn p(x: f64, y: f64) -> Point {
    Point::new(mil(x), mil(y))
}

fn line(ax: f64, ay: f64, bx: f64, by: f64) -> Shape {
    Shape::Line {
        a: p(ax, ay),
        b: p(bx, by),
        width: W,
    }
}

fn poly(pts: &[(f64, f64)], fill: bool) -> Shape {
    Shape::Polygon {
        points: pts.iter().map(|&(x, y)| p(x, y)).collect(),
        width: W,
        fill,
    }
}

fn pin(number: &str, name: &str, x: f64, y: f64, angle: f64, len: f64, kind: PinKind) -> Pin {
    Pin {
        number: number.into(),
        name: name.into(),
        pos: p(x, y),
        angle,
        length: mil(len),
        kind,
        hide_name: true,
    }
}

/// Two-terminal part along the X axis, pins at x = ±200 mil.
fn two_pin(
    id: &str,
    name: &str,
    body_half: f64,
    graphics: Vec<Shape>,
    kinds: (PinKind, PinKind),
) -> Symbol {
    let len = 200.0 - body_half;
    Symbol {
        id: id.into(),
        name: name.into(),
        pins: vec![
            pin("1", "1", -200.0, 0.0, 0.0, len, kinds.0),
            pin("2", "2", 200.0, 0.0, 180.0, len, kinds.1),
        ],
        graphics,
    }
}

pub fn resistor() -> Symbol {
    two_pin(
        "sym:resistor",
        "Resistor",
        100.0,
        vec![Shape::rect(p(-100.0, -35.0), p(100.0, 35.0), W)],
        (PinKind::Passive, PinKind::Passive),
    )
}

pub fn capacitor() -> Symbol {
    two_pin(
        "sym:capacitor",
        "Capacitor",
        20.0,
        vec![
            line(-20.0, -70.0, -20.0, 70.0),
            line(20.0, -70.0, 20.0, 70.0),
        ],
        (PinKind::Passive, PinKind::Passive),
    )
}

pub fn capacitor_polarized() -> Symbol {
    two_pin(
        "sym:capacitor-pol",
        "Polarized Capacitor",
        20.0,
        vec![
            Shape::rect(p(-30.0, -70.0), p(-10.0, 70.0), W),
            Shape::Arc {
                center: p(90.0, 0.0),
                radius: mil(80.0),
                start: 120.0,
                sweep: 120.0,
                width: W,
            },
            line(-80.0, 50.0, -50.0, 50.0),
            line(-65.0, 35.0, -65.0, 65.0),
        ],
        (PinKind::Passive, PinKind::Passive),
    )
}

pub fn inductor() -> Symbol {
    let arcs = (0..4)
        .map(|i| Shape::Arc {
            center: p(-75.0 + 50.0 * i as f64, 0.0),
            radius: mil(25.0),
            start: 0.0,
            sweep: 180.0,
            width: W,
        })
        .collect();
    two_pin(
        "sym:inductor",
        "Inductor",
        100.0,
        arcs,
        (PinKind::Passive, PinKind::Passive),
    )
}

pub fn ferrite() -> Symbol {
    let mut g = vec![Shape::rect(p(-80.0, -30.0), p(80.0, 30.0), W)];
    g.push(Shape::Rect {
        a: p(-60.0, -20.0),
        b: p(60.0, 20.0),
        width: W,
        fill: true,
    });
    two_pin(
        "sym:ferrite",
        "Ferrite Bead",
        80.0,
        g,
        (PinKind::Passive, PinKind::Passive),
    )
}

pub fn fuse() -> Symbol {
    two_pin(
        "sym:fuse",
        "Fuse",
        100.0,
        vec![
            Shape::rect(p(-100.0, -30.0), p(100.0, 30.0), W),
            line(-100.0, 0.0, 100.0, 0.0),
        ],
        (PinKind::Passive, PinKind::Passive),
    )
}

fn diode_body(extra: Vec<Shape>) -> Vec<Shape> {
    let mut g = vec![
        poly(&[(-40.0, 50.0), (-40.0, -50.0), (40.0, 0.0)], true),
        line(40.0, -50.0, 40.0, 50.0),
    ];
    g.extend(extra);
    g
}

/// Pin 1 is the cathode (K), matching the band on most SMD packages.
fn diode_like(id: &str, name: &str, g: Vec<Shape>) -> Symbol {
    let mut s = two_pin(id, name, 40.0, g, (PinKind::Passive, PinKind::Passive));
    s.pins[0].pos = p(200.0, 0.0);
    s.pins[0].angle = 180.0;
    s.pins[0].name = "K".into();
    s.pins[1].pos = p(-200.0, 0.0);
    s.pins[1].angle = 0.0;
    s.pins[1].name = "A".into();
    s
}

pub fn diode() -> Symbol {
    diode_like("sym:diode", "Diode", diode_body(vec![]))
}

pub fn schottky() -> Symbol {
    diode_like(
        "sym:schottky",
        "Schottky Diode",
        diode_body(vec![
            line(40.0, 50.0, 60.0, 50.0),
            line(60.0, 50.0, 60.0, 35.0),
            line(40.0, -50.0, 20.0, -50.0),
            line(20.0, -50.0, 20.0, -35.0),
        ]),
    )
}

pub fn zener() -> Symbol {
    diode_like(
        "sym:zener",
        "Zener Diode",
        diode_body(vec![
            line(40.0, 50.0, 60.0, 65.0),
            line(40.0, -50.0, 20.0, -65.0),
        ]),
    )
}

pub fn led() -> Symbol {
    let arrow = |x: f64| {
        vec![
            line(x, 60.0, x + 40.0, 100.0),
            poly(
                &[(x + 40.0, 100.0), (x + 22.0, 94.0), (x + 34.0, 82.0)],
                true,
            ),
        ]
    };
    let mut extra = arrow(-10.0);
    extra.extend(arrow(25.0));
    diode_like("sym:led", "LED", diode_body(extra))
}

/// Bipolar transistor: pin 1 B, 2 E, 3 C (SOT-23 order).
fn bjt(id: &str, name: &str, pnp: bool) -> Symbol {
    let mut g = vec![
        line(-20.0, -60.0, -20.0, 60.0),
        line(-20.0, 25.0, 60.0, 80.0),
        line(-20.0, -25.0, 60.0, -80.0),
        Shape::Circle {
            center: p(15.0, 0.0),
            radius: mil(90.0),
            width: W,
            fill: false,
        },
    ];
    if pnp {
        g.push(poly(&[(-20.0, -25.0), (5.0, -30.0), (-6.0, -46.0)], true));
    } else {
        g.push(poly(&[(60.0, -80.0), (34.0, -75.0), (46.0, -58.0)], true));
    }
    Symbol {
        id: id.into(),
        name: name.into(),
        pins: vec![
            pin("1", "B", -100.0, 0.0, 0.0, 80.0, PinKind::Input),
            pin(
                "2",
                "E",
                60.0,
                if pnp { 100.0 } else { -100.0 },
                if pnp { 270.0 } else { 90.0 },
                20.0,
                PinKind::Passive,
            ),
            pin(
                "3",
                "C",
                60.0,
                if pnp { -100.0 } else { 100.0 },
                if pnp { 90.0 } else { 270.0 },
                20.0,
                PinKind::Passive,
            ),
        ],
        graphics: if pnp {
            // Emitter on top for PNP so the supply sits above.
            g.into_iter()
                .map(|s| match s {
                    Shape::Line { a, b, width } => Shape::Line {
                        a: Point::new(a.x, -a.y),
                        b: Point::new(b.x, -b.y),
                        width,
                    },
                    Shape::Polygon {
                        points,
                        width,
                        fill,
                    } => Shape::Polygon {
                        points: points.iter().map(|q| Point::new(q.x, -q.y)).collect(),
                        width,
                        fill,
                    },
                    other => other,
                })
                .collect()
        } else {
            g
        },
    }
}

pub fn npn() -> Symbol {
    bjt("sym:npn", "NPN Transistor", false)
}

pub fn pnp() -> Symbol {
    bjt("sym:pnp", "PNP Transistor", true)
}

/// MOSFET: pin 1 G, 2 S, 3 D (SOT-23 order).
fn mosfet(id: &str, name: &str, p_channel: bool) -> Symbol {
    let s = if p_channel { -1.0 } else { 1.0 };
    let mut g = vec![
        line(-40.0, -50.0, -40.0, 50.0),
        line(-20.0, -60.0, -20.0, -30.0),
        line(-20.0, -15.0, -20.0, 15.0),
        line(-20.0, 30.0, -20.0, 60.0),
        line(-20.0, 45.0 * s, 60.0, 45.0 * s),
        line(-20.0, -45.0 * s, 60.0, -45.0 * s),
        line(-20.0, 0.0, 60.0, 0.0),
        line(60.0, 0.0, 60.0, -45.0 * s),
        Shape::Circle {
            center: p(15.0, 0.0),
            radius: mil(90.0),
            width: W,
            fill: false,
        },
    ];
    if p_channel {
        g.push(poly(&[(20.0, 0.0), (-5.0, 12.0), (-5.0, -12.0)], true));
    } else {
        g.push(poly(&[(-20.0, 0.0), (5.0, 12.0), (5.0, -12.0)], true));
    }
    let (sy, dy) = if p_channel {
        (100.0, -100.0)
    } else {
        (-100.0, 100.0)
    };
    Symbol {
        id: id.into(),
        name: name.into(),
        pins: vec![
            pin(
                "1",
                "G",
                -100.0,
                if p_channel { 50.0 } else { -50.0 },
                0.0,
                60.0,
                PinKind::Input,
            ),
            pin(
                "2",
                "S",
                60.0,
                sy,
                if p_channel { 270.0 } else { 90.0 },
                55.0,
                PinKind::Passive,
            ),
            pin(
                "3",
                "D",
                60.0,
                dy,
                if p_channel { 90.0 } else { 270.0 },
                55.0,
                PinKind::Passive,
            ),
        ],
        graphics: g,
    }
}

pub fn nmos() -> Symbol {
    mosfet("sym:nmos", "N-Channel MOSFET", false)
}

pub fn pmos() -> Symbol {
    mosfet("sym:pmos", "P-Channel MOSFET", true)
}

/// Single op-amp: 1 OUT, 2 V-, 3 +IN, 4 -IN, 5 V+ (SOT-23-5 order).
pub fn opamp() -> Symbol {
    Symbol {
        id: "sym:opamp".into(),
        name: "Op-Amp".into(),
        pins: vec![
            pin("1", "OUT", 300.0, 0.0, 180.0, 100.0, PinKind::Output),
            pin("2", "V-", 0.0, -200.0, 90.0, 100.0, PinKind::PowerIn),
            pin("3", "+", -300.0, -100.0, 0.0, 100.0, PinKind::Input),
            pin("4", "-", -300.0, 100.0, 0.0, 100.0, PinKind::Input),
            pin("5", "V+", 0.0, 200.0, 270.0, 100.0, PinKind::PowerIn),
        ],
        graphics: vec![
            poly(&[(-200.0, 200.0), (-200.0, -200.0), (200.0, 0.0)], false),
            line(-180.0, 100.0, -140.0, 100.0),
            line(-180.0, -100.0, -140.0, -100.0),
            line(-160.0, -120.0, -160.0, -80.0),
        ],
    }
}

pub fn crystal() -> Symbol {
    two_pin(
        "sym:crystal",
        "Crystal",
        60.0,
        vec![
            Shape::rect(p(-25.0, -60.0), p(25.0, 60.0), W),
            line(-50.0, -50.0, -50.0, 50.0),
            line(50.0, -50.0, 50.0, 50.0),
        ],
        (PinKind::Passive, PinKind::Passive),
    )
}

pub fn button() -> Symbol {
    two_pin(
        "sym:button",
        "Push Button",
        100.0,
        vec![
            Shape::Circle {
                center: p(-80.0, 0.0),
                radius: mil(15.0),
                width: W,
                fill: false,
            },
            Shape::Circle {
                center: p(80.0, 0.0),
                radius: mil(15.0),
                width: W,
                fill: false,
            },
            line(-100.0, 50.0, 100.0, 50.0),
            line(0.0, 50.0, 0.0, 100.0),
            line(-40.0, 100.0, 40.0, 100.0),
        ],
        (PinKind::Passive, PinKind::Passive),
    )
}

pub fn mounting_hole() -> Symbol {
    Symbol {
        id: "sym:mounting-hole".into(),
        name: "Mounting Hole".into(),
        pins: vec![],
        graphics: vec![
            Shape::Circle {
                center: Point::ZERO,
                radius: mil(60.0),
                width: W,
                fill: false,
            },
            Shape::Circle {
                center: Point::ZERO,
                radius: mil(30.0),
                width: W,
                fill: false,
            },
        ],
    }
}

pub fn test_point() -> Symbol {
    Symbol {
        id: "sym:test-point".into(),
        name: "Test Point".into(),
        pins: vec![pin("1", "1", 0.0, -100.0, 90.0, 60.0, PinKind::Passive)],
        graphics: vec![Shape::Circle {
            center: p(0.0, 0.0),
            radius: mil(40.0),
            width: W,
            fill: false,
        }],
    }
}

/// Rectangular IC body with pins on the left and right.
pub fn ic_box(
    id: &str,
    name: &str,
    left: &[(&str, &str, PinKind)],
    right: &[(&str, &str, PinKind)],
) -> Symbol {
    let rows = left.len().max(right.len()) as i64;
    let longest = left
        .iter()
        .chain(right)
        .map(|(_, n, _)| n.chars().count())
        .max()
        .unwrap_or(1) as f64;
    // Half width: room for names on both sides, rounded up to 100 mil.
    let half = (((longest * 2.0 * 47.0 + 120.0) / 2.0 / 100.0).ceil() * 100.0).max(200.0);
    let top = ((rows - 1) as f64 * 100.0 / 2.0 / 100.0).ceil() * 100.0;
    let mut pins = Vec::new();
    for (i, (num, nm, kind)) in left.iter().enumerate() {
        let mut pn = pin(
            num,
            nm,
            -half - 100.0,
            top - i as f64 * 100.0,
            0.0,
            100.0,
            *kind,
        );
        pn.hide_name = false;
        pins.push(pn);
    }
    for (i, (num, nm, kind)) in right.iter().enumerate() {
        let mut pn = pin(
            num,
            nm,
            half + 100.0,
            top - i as f64 * 100.0,
            180.0,
            100.0,
            *kind,
        );
        pn.hide_name = false;
        pins.push(pn);
    }
    let bottom = top - (rows - 1) as f64 * 100.0;
    Symbol {
        id: id.into(),
        name: name.into(),
        pins,
        graphics: vec![Shape::Rect {
            a: p(-half, top + 100.0),
            b: p(half, bottom - 100.0),
            width: W,
            fill: false,
        }],
    }
}

/// Single-row connector: pins on the left, numbered top to bottom.
pub fn connector(id: &str, name: &str, pins: &[&str]) -> Symbol {
    let n = pins.len();
    let top = ((n as f64 - 1.0) * 100.0 / 2.0 / 100.0).ceil() * 100.0;
    let bottom = top - (n as f64 - 1.0) * 100.0;
    let mut out = Vec::new();
    let mut g = vec![Shape::rect(
        p(-100.0, top + 100.0),
        p(100.0, bottom - 100.0),
        W,
    )];
    for (i, nm) in pins.iter().enumerate() {
        let y = top - i as f64 * 100.0;
        let mut pn = pin(
            &(i + 1).to_string(),
            nm,
            -300.0,
            y,
            0.0,
            200.0,
            PinKind::Passive,
        );
        pn.hide_name = nm.chars().all(|c| c.is_ascii_digit());
        out.push(pn);
        g.push(Shape::Rect {
            a: p(-60.0, y - 15.0),
            b: p(-20.0, y + 15.0),
            width: W,
            fill: true,
        });
    }
    Symbol {
        id: id.into(),
        name: name.into(),
        pins: out,
        graphics: g,
    }
}
