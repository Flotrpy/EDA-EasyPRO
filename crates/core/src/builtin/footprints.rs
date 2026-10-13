//! Land patterns for the built-in library (IPC-7351 nominal density).

use crate::geom::Point;
use crate::layer::Layer;
use crate::library::{Footprint, FpGraphic, Pad, PadKind, PadShape};
use crate::shape::Shape;
use crate::units::mm;

const SILK: i64 = 150_000;
const FAB: i64 = 100_000;

fn p(x: f64, y: f64) -> Point {
    Point::new(mm(x), mm(y))
}

fn smd(number: &str, x: f64, y: f64, w: f64, h: f64) -> Pad {
    Pad {
        number: number.into(),
        pos: p(x, y),
        size: p(w, h),
        shape: PadShape::RoundRect { ratio_pct: 25 },
        kind: PadKind::Smd,
        rotation: 0.0,
    }
}

fn tht(number: &str, x: f64, y: f64, size: f64, drill: f64, square: bool) -> Pad {
    Pad {
        number: number.into(),
        pos: p(x, y),
        size: p(size, size),
        shape: if square {
            PadShape::Rect
        } else {
            PadShape::Round
        },
        kind: PadKind::Tht { drill: mm(drill) },
        rotation: 0.0,
    }
}

fn silk_line(ax: f64, ay: f64, bx: f64, by: f64) -> FpGraphic {
    FpGraphic {
        layer: Layer::TopSilk,
        shape: Shape::Line {
            a: p(ax, ay),
            b: p(bx, by),
            width: SILK,
        },
    }
}

fn silk_rect(w: f64, h: f64) -> FpGraphic {
    FpGraphic {
        layer: Layer::TopSilk,
        shape: Shape::rect(p(-w / 2.0, -h / 2.0), p(w / 2.0, h / 2.0), SILK),
    }
}

fn silk_dot(x: f64, y: f64) -> FpGraphic {
    FpGraphic {
        layer: Layer::TopSilk,
        shape: Shape::Circle {
            center: p(x, y),
            radius: mm(0.15),
            width: 0,
            fill: true,
        },
    }
}

fn fab_rect(w: f64, h: f64) -> FpGraphic {
    FpGraphic {
        layer: Layer::TopAssembly,
        shape: Shape::rect(p(-w / 2.0, -h / 2.0), p(w / 2.0, h / 2.0), FAB),
    }
}

fn fp(id: &str, name: &str, pads: Vec<Pad>, graphics: Vec<FpGraphic>) -> Footprint {
    Footprint {
        id: id.into(),
        name: name.into(),
        pads,
        graphics,
    }
}

/// Two-terminal chip (resistor/capacitor/inductor/fuse) package.
fn chip(code: &str, pitch: f64, pw: f64, ph: f64, body_l: f64, body_w: f64) -> Footprint {
    let x = pitch / 2.0;
    let sy = (ph / 2.0 + 0.2).max(body_w / 2.0 + 0.1);
    let sx = (x - pw / 2.0 - 0.2).max(0.1);
    let mut g = vec![fab_rect(body_l, body_w)];
    if sx > 0.15 {
        g.push(silk_line(-sx, sy, sx, sy));
        g.push(silk_line(-sx, -sy, sx, -sy));
    }
    fp(
        &format!("fp:{code}"),
        code,
        vec![smd("1", -x, 0.0, pw, ph), smd("2", x, 0.0, pw, ph)],
        g,
    )
}

pub fn c0402() -> Footprint {
    chip("0402", 1.0, 0.6, 0.6, 1.0, 0.5)
}

pub fn c0603() -> Footprint {
    chip("0603", 1.6, 0.9, 0.95, 1.6, 0.8)
}

pub fn c0805() -> Footprint {
    chip("0805", 1.9, 1.0, 1.35, 2.0, 1.25)
}

pub fn c1206() -> Footprint {
    chip("1206", 3.0, 1.15, 1.8, 3.2, 1.6)
}

/// Diode packages carry a cathode bar on the pad-1 side.
fn diode_pkg(code: &str, pitch: f64, pw: f64, ph: f64, bl: f64, bw: f64) -> Footprint {
    let mut f = chip(code, pitch, pw, ph, bl, bw);
    let x = -pitch / 2.0 - pw / 2.0 - 0.25;
    let y = ph / 2.0 + 0.1;
    f.graphics.push(silk_line(x, -y, x, y));
    f
}

pub fn sod123() -> Footprint {
    diode_pkg("SOD-123", 3.3, 0.9, 1.2, 2.7, 1.6)
}

pub fn sma() -> Footprint {
    diode_pkg("SMA", 4.0, 1.6, 1.8, 4.3, 2.6)
}

pub fn led0805() -> Footprint {
    let mut f = diode_pkg("LED-0805", 1.9, 1.0, 1.35, 2.0, 1.25);
    f.id = "fp:LED-0805".into();
    f
}

/// SOT-23 (3 pads): 1 bottom-left, 2 bottom-right, 3 top.
pub fn sot23() -> Footprint {
    fp(
        "fp:SOT-23",
        "SOT-23",
        vec![
            smd("1", -0.95, -1.1, 0.6, 1.0),
            smd("2", 0.95, -1.1, 0.6, 1.0),
            smd("3", 0.0, 1.1, 0.6, 1.0),
        ],
        vec![
            fab_rect(2.9, 1.3),
            silk_line(-1.55, 0.75, -0.5, 0.75),
            silk_line(0.5, 0.75, 1.55, 0.75),
            silk_line(-1.55, 0.75, -1.55, -0.3),
            silk_line(1.55, 0.75, 1.55, -0.3),
        ],
    )
}

/// SOT-23-5: 1,2,3 along the bottom, 4,5 on top (right to left).
pub fn sot23_5() -> Footprint {
    fp(
        "fp:SOT-23-5",
        "SOT-23-5",
        vec![
            smd("1", -0.95, -1.2, 0.6, 1.1),
            smd("2", 0.0, -1.2, 0.6, 1.1),
            smd("3", 0.95, -1.2, 0.6, 1.1),
            smd("4", 0.95, 1.2, 0.6, 1.1),
            smd("5", -0.95, 1.2, 0.6, 1.1),
        ],
        vec![
            fab_rect(2.9, 1.6),
            silk_line(-0.3, 0.9, 0.3, 0.9),
            silk_line(-0.3, -0.9, 0.3, -0.9),
            silk_dot(-1.7, -1.4),
        ],
    )
}

/// SOT-223 with the tab sharing pad number 2 (regulator output).
pub fn sot223() -> Footprint {
    fp(
        "fp:SOT-223",
        "SOT-223",
        vec![
            smd("1", -2.3, -3.15, 0.95, 2.0),
            smd("2", 0.0, -3.15, 0.95, 2.0),
            smd("3", 2.3, -3.15, 0.95, 2.0),
            smd("2", 0.0, 3.15, 3.25, 2.0),
        ],
        vec![
            fab_rect(6.5, 3.5),
            silk_line(-3.4, -1.9, 3.4, -1.9),
            silk_line(-3.4, 1.9, 3.4, 1.9),
            silk_line(-3.4, -1.9, -3.4, 1.9),
            silk_line(3.4, -1.9, 3.4, 1.9),
        ],
    )
}

/// Dual-row gull-wing package (SOIC/SSOP/TSSOP). Pin 1 top-left, counter-clockwise.
pub fn dual_row_smd(
    name: &str,
    pins: usize,
    pitch: f64,
    span: f64,
    pw: f64,
    ph: f64,
    body_w: f64,
) -> Footprint {
    let half = pins / 2;
    let top = (half as f64 - 1.0) * pitch / 2.0;
    let x = span / 2.0;
    let mut pads = Vec::new();
    for i in 0..half {
        pads.push(smd(
            &(i + 1).to_string(),
            -x,
            top - i as f64 * pitch,
            pw,
            ph,
        ));
    }
    for i in 0..half {
        pads.push(smd(
            &(half + i + 1).to_string(),
            x,
            -top + i as f64 * pitch,
            pw,
            ph,
        ));
    }
    let body_l = (half as f64) * pitch + 0.6;
    let bx = body_w / 2.0;
    let by = body_l / 2.0;
    let g = vec![
        fab_rect(body_w, body_l),
        silk_line(-bx + 0.3, by + 0.1, bx - 0.3, by + 0.1),
        silk_line(-bx + 0.3, -by - 0.1, bx - 0.3, -by - 0.1),
        silk_dot(-x - pw / 2.0 - 0.1, top + ph / 2.0 + 0.35),
    ];
    fp(&format!("fp:{name}"), name, pads, g)
}

pub fn soic8() -> Footprint {
    dual_row_smd("SOIC-8", 8, 1.27, 5.4, 1.55, 0.6, 3.9)
}

pub fn soic16() -> Footprint {
    dual_row_smd("SOIC-16", 16, 1.27, 5.4, 1.55, 0.6, 3.9)
}

/// Quad flat package. Pin 1 at the top of the left side, counter-clockwise.
pub fn qfp(
    name: &str,
    pins: usize,
    pitch: f64,
    span: f64,
    pl: f64,
    pwid: f64,
    body: f64,
) -> Footprint {
    let n = pins / 4;
    let first = (n as f64 - 1.0) * pitch / 2.0;
    let c = span / 2.0;
    let mut pads = Vec::new();
    let mut num = 1;
    for i in 0..n {
        pads.push(smd(
            &num.to_string(),
            -c,
            first - i as f64 * pitch,
            pl,
            pwid,
        ));
        num += 1;
    }
    for i in 0..n {
        pads.push(smd(
            &num.to_string(),
            -first + i as f64 * pitch,
            -c,
            pwid,
            pl,
        ));
        num += 1;
    }
    for i in 0..n {
        pads.push(smd(
            &num.to_string(),
            c,
            -first + i as f64 * pitch,
            pl,
            pwid,
        ));
        num += 1;
    }
    for i in 0..n {
        pads.push(smd(&num.to_string(), first - i as f64 * pitch, c, pwid, pl));
        num += 1;
    }
    let b = body / 2.0;
    let e = first + pwid / 2.0 + 0.3;
    let g = vec![
        fab_rect(body, body),
        silk_line(-b, b, -e, b),
        silk_line(-b, b, -b, e),
        silk_line(b, b, e, b),
        silk_line(b, b, b, e),
        silk_line(-b, -b, -e, -b),
        silk_line(-b, -b, -b, -e),
        silk_line(b, -b, e, -b),
        silk_line(b, -b, b, -e),
        silk_dot(-c - pl / 2.0 - 0.1, first + 0.6),
    ];
    fp(&format!("fp:{name}"), name, pads, g)
}

pub fn lqfp32() -> Footprint {
    qfp("LQFP-32", 32, 0.8, 8.4, 1.5, 0.55, 7.0)
}

pub fn lqfp48() -> Footprint {
    qfp("LQFP-48", 48, 0.5, 8.4, 1.5, 0.3, 7.0)
}

/// Dual in-line through-hole package; `row` is the row spacing in mm.
pub fn dip(pins: usize, row: f64) -> Footprint {
    let half = pins / 2;
    let top = (half as f64 - 1.0) * 2.54 / 2.0;
    let x = row / 2.0;
    let mut pads = Vec::new();
    for i in 0..half {
        let mut pd = tht(
            &(i + 1).to_string(),
            -x,
            top - i as f64 * 2.54,
            1.6,
            0.8,
            i == 0,
        );
        pd.size = p(2.0, 1.6);
        if i != 0 {
            pd.shape = PadShape::Oval;
        }
        pads.push(pd);
    }
    for i in 0..half {
        let mut pd = tht(
            &(half + i + 1).to_string(),
            x,
            -top + i as f64 * 2.54,
            1.6,
            0.8,
            false,
        );
        pd.size = p(2.0, 1.6);
        pd.shape = PadShape::Oval;
        pads.push(pd);
    }
    let bw = row - 1.27;
    let bl = half as f64 * 2.54;
    let name = format!("DIP-{pins}");
    let g = vec![
        fab_rect(bw, bl),
        silk_rect(bw - 0.6, bl),
        FpGraphic {
            layer: Layer::TopSilk,
            shape: Shape::Arc {
                center: p(0.0, bl / 2.0),
                radius: mm(0.6),
                start: 180.0,
                sweep: 180.0,
                width: SILK,
            },
        },
    ];
    fp(&format!("fp:{name}"), &name, pads, g)
}

/// 1 x N pin header (2.54 mm pitch), pin 1 on top with a square pad.
pub fn header(n: usize, pitch: f64) -> Footprint {
    let top = (n as f64 - 1.0) * pitch / 2.0;
    let pads = (0..n)
        .map(|i| {
            tht(
                &(i + 1).to_string(),
                0.0,
                top - i as f64 * pitch,
                1.7,
                1.0,
                i == 0,
            )
        })
        .collect();
    let name = if (pitch - 2.54).abs() < 1e-6 {
        format!("HDR-1x{n}-2.54")
    } else {
        format!("HDR-1x{n}-{pitch}")
    };
    let len = n as f64 * pitch;
    let g = vec![fab_rect(pitch, len), silk_rect(pitch + 0.2, len + 0.2)];
    fp(&format!("fp:{name}"), &name, pads, g)
}

/// 2 x N pin header (2.54 mm), pin 1 top-left, numbered in zig-zag rows.
pub fn header2(rows: usize) -> Footprint {
    let pitch = 2.54;
    let top = (rows as f64 - 1.0) * pitch / 2.0;
    let pads = (0..2 * rows)
        .map(|i| {
            let x = if i % 2 == 0 {
                -pitch / 2.0
            } else {
                pitch / 2.0
            };
            let y = top - (i / 2) as f64 * pitch;
            tht(&(i + 1).to_string(), x, y, 1.7, 1.0, i == 0)
        })
        .collect();
    let name = format!("HDR-2x{rows}-2.54");
    let len = rows as f64 * pitch;
    let g = vec![
        fab_rect(2.0 * pitch, len),
        silk_rect(2.0 * pitch + 0.2, len + 0.2),
    ];
    fp(&format!("fp:{name}"), &name, pads, g)
}

pub fn screw_terminal_2() -> Footprint {
    let pads = vec![
        tht("1", -2.54, 0.0, 2.4, 1.2, true),
        tht("2", 2.54, 0.0, 2.4, 1.2, false),
    ];
    fp(
        "fp:TERM-2P-5.08",
        "TERM-2P-5.08",
        pads,
        vec![
            fab_rect(10.2, 7.6),
            silk_rect(10.4, 7.8),
            silk_line(-5.2, -2.5, 5.2, -2.5),
        ],
    )
}

pub fn radial_cap() -> Footprint {
    let pads = vec![
        tht("1", -1.0, 0.0, 1.6, 0.8, true),
        tht("2", 1.0, 0.0, 1.6, 0.8, false),
    ];
    let g = vec![
        FpGraphic {
            layer: Layer::TopSilk,
            shape: Shape::Circle {
                center: Point::ZERO,
                radius: mm(2.6),
                width: SILK,
                fill: false,
            },
        },
        silk_line(-2.0, 1.4, -2.0, 2.0),
        silk_line(-2.3, 1.7, -1.7, 1.7),
    ];
    fp("fp:CAP-RAD-D5-P2", "CAP-RAD-D5-P2", pads, g)
}

pub fn led_5mm() -> Footprint {
    // Pad 1 (cathode) square, on the flat side.
    let pads = vec![
        tht("1", -1.27, 0.0, 1.8, 0.9, true),
        tht("2", 1.27, 0.0, 1.8, 0.9, false),
    ];
    let g = vec![FpGraphic {
        layer: Layer::TopSilk,
        shape: Shape::Arc {
            center: Point::ZERO,
            radius: mm(2.9),
            start: 210.0,
            sweep: 300.0,
            width: SILK,
        },
    }];
    fp("fp:LED-5MM", "LED-5MM", pads, g)
}

pub fn hc49() -> Footprint {
    let pads = vec![
        tht("1", -2.44, 0.0, 1.5, 0.8, false),
        tht("2", 2.44, 0.0, 1.5, 0.8, false),
    ];
    fp(
        "fp:HC-49US",
        "HC-49US",
        pads,
        vec![fab_rect(11.0, 4.6), silk_rect(11.2, 4.8)],
    )
}

pub fn to220() -> Footprint {
    let pads = vec![
        tht("1", -2.54, 0.0, 1.8, 1.1, true),
        tht("2", 0.0, 0.0, 1.8, 1.1, false),
        tht("3", 2.54, 0.0, 1.8, 1.1, false),
    ];
    fp(
        "fp:TO-220",
        "TO-220-3",
        pads,
        vec![
            fab_rect(10.0, 4.5),
            silk_rect(10.4, 4.7),
            silk_line(-5.2, 1.0, 5.2, 1.0),
        ],
    )
}

/// 6 x 6 mm tactile switch: pads 1/1 and 2/2 are internally joined.
pub fn tact_6x6() -> Footprint {
    let pads = vec![
        tht("1", -3.25, 2.25, 1.8, 1.0, false),
        tht("2", 3.25, 2.25, 1.8, 1.0, false),
        tht("1", -3.25, -2.25, 1.8, 1.0, false),
        tht("2", 3.25, -2.25, 1.8, 1.0, false),
    ];
    let g = vec![
        fab_rect(6.0, 6.0),
        silk_line(-2.0, 3.1, 2.0, 3.1),
        silk_line(-2.0, -3.1, 2.0, -3.1),
        FpGraphic {
            layer: Layer::TopSilk,
            shape: Shape::Circle {
                center: Point::ZERO,
                radius: mm(1.75),
                width: SILK,
                fill: false,
            },
        },
    ];
    fp("fp:SW-TACT-6x6", "SW-TACT-6x6", pads, g)
}

pub fn mounting_hole_m3() -> Footprint {
    let pad = Pad {
        number: "".into(),
        pos: Point::ZERO,
        size: p(3.2, 3.2),
        shape: PadShape::Round,
        kind: PadKind::Npth { drill: mm(3.2) },
        rotation: 0.0,
    };
    let g = vec![FpGraphic {
        layer: Layer::TopSilk,
        shape: Shape::Circle {
            center: Point::ZERO,
            radius: mm(3.0),
            width: SILK,
            fill: false,
        },
    }];
    fp("fp:MH-M3", "MountingHole-M3", vec![pad], g)
}

pub fn test_point_pad() -> Footprint {
    let pad = Pad {
        number: "1".into(),
        pos: Point::ZERO,
        size: p(1.5, 1.5),
        shape: PadShape::Round,
        kind: PadKind::Smd,
        rotation: 0.0,
    };
    fp("fp:TP-1.5", "TestPoint-1.5", vec![pad], vec![])
}

/// USB Type-C receptacle (USB 2.0 subset). Signal pads are numbered by
/// function so duplicated contacts share a net.
pub fn usb_c() -> Footprint {
    let row = [
        ("GND", -3.2, 0.6),
        ("VBUS", -2.4, 0.6),
        ("SBU2", -1.75, 0.3),
        ("CC1", -1.25, 0.3),
        ("D-", -0.75, 0.3),
        ("D+", -0.25, 0.3),
        ("D-", 0.25, 0.3),
        ("D+", 0.75, 0.3),
        ("SBU1", 1.25, 0.3),
        ("CC2", 1.75, 0.3),
        ("VBUS", 2.4, 0.6),
        ("GND", 3.2, 0.6),
    ];
    let mut pads: Vec<Pad> = row
        .iter()
        .map(|(n, x, w)| smd(n, *x, 3.4, *w, 1.15))
        .collect();
    for (x, y, h) in [
        (-4.32, 2.85, 2.0),
        (4.32, 2.85, 2.0),
        (-4.32, -1.25, 1.8),
        (4.32, -1.25, 1.8),
    ] {
        pads.push(Pad {
            number: "SHELL".into(),
            pos: p(x, y),
            size: p(1.0, h),
            shape: PadShape::Oval,
            kind: PadKind::Tht { drill: mm(0.6) },
            rotation: 0.0,
        });
    }
    let g = vec![
        fab_rect(8.94, 7.3),
        silk_line(-4.6, -3.0, 4.6, -3.0),
        silk_line(-4.6, 0.3, -4.6, 1.6),
        silk_line(4.6, 0.3, 4.6, 1.6),
    ];
    let mut f = fp("fp:USB-C-16P", "USB-C-16P", pads, g);
    for pd in &mut f.pads {
        if pd.number == "SHELL" {
            continue;
        }
        pd.shape = PadShape::Rect;
    }
    f
}

/// ESP32-WROOM-32 module castellations (38 pads + exposed ground).
pub fn esp32_wroom() -> Footprint {
    let mut pads = Vec::new();
    for i in 0..14 {
        pads.push(smd(
            &(i + 1).to_string(),
            -8.75,
            5.0 - i as f64 * 1.27,
            1.5,
            0.9,
        ));
    }
    for i in 0..10 {
        pads.push(smd(
            &(i + 15).to_string(),
            -5.715 + i as f64 * 1.27,
            -12.5,
            0.9,
            1.5,
        ));
    }
    for i in 0..14 {
        pads.push(smd(
            &(i + 25).to_string(),
            8.75,
            -11.51 + i as f64 * 1.27,
            1.5,
            0.9,
        ));
    }
    let mut ep = smd("39", -1.0, -2.0, 5.0, 5.0);
    ep.shape = PadShape::Rect;
    pads.push(ep);
    let g = vec![
        FpGraphic {
            layer: Layer::TopAssembly,
            shape: Shape::rect(p(-9.0, -12.75), p(9.0, 12.75), FAB),
        },
        silk_line(-9.1, 12.85, 9.1, 12.85),
        silk_line(-9.1, 12.85, -9.1, 6.0),
        silk_line(9.1, 12.85, 9.1, 6.0),
        silk_line(-9.1, 6.75, 9.1, 6.75),
        silk_dot(-10.0, 5.0),
    ];
    fp("fp:ESP32-WROOM-32", "ESP32-WROOM-32", pads, g)
}

pub fn all() -> Vec<Footprint> {
    let mut v = vec![
        c0402(),
        c0603(),
        c0805(),
        c1206(),
        sod123(),
        sma(),
        led0805(),
        sot23(),
        sot23_5(),
        sot223(),
        soic8(),
        soic16(),
        lqfp32(),
        lqfp48(),
        dip(8, 7.62),
        dip(28, 7.62),
        screw_terminal_2(),
        radial_cap(),
        led_5mm(),
        hc49(),
        to220(),
        tact_6x6(),
        mounting_hole_m3(),
        test_point_pad(),
        usb_c(),
        esp32_wroom(),
    ];
    for n in [2, 3, 4, 6, 8] {
        v.push(header(n, 2.54));
    }
    for rows in [3, 5, 10] {
        v.push(header2(rows));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_row_header_numbering() {
        let f = header2(5);
        assert_eq!(f.pads.len(), 10);
        let at = |n: &str| f.pad(n).unwrap().pos;
        // 1 top-left, 2 beside it, then down in rows.
        assert!(at("1").x < at("2").x && at("1").y == at("2").y);
        assert!(at("3").y < at("1").y && at("3").x == at("1").x);
        assert_eq!(at("1").y, -at("9").y);
        assert!(matches!(f.pad("1").unwrap().shape, PadShape::Rect));
    }

    #[test]
    fn qfp_numbering_is_counter_clockwise() {
        let f = lqfp32();
        assert_eq!(f.pads.len(), 32);
        let p1 = f.pad("1").unwrap().pos;
        let p9 = f.pad("9").unwrap().pos;
        let p17 = f.pad("17").unwrap().pos;
        assert!(p1.x < 0 && p1.y > 0);
        assert!(p9.y < 0 && p9.x < 0);
        assert!(p17.x > 0 && p17.y < 0);
    }

    #[test]
    fn pads_do_not_overlap() {
        for f in all() {
            let hulls: Vec<_> = f
                .pads
                .iter()
                .map(|p| (p.number.clone(), p.hull(&Default::default())))
                .collect();
            for i in 0..hulls.len() {
                for j in i + 1..hulls.len() {
                    if hulls[i].0 == hulls[j].0 {
                        continue;
                    }
                    let d = hulls[i].1.distance(&hulls[j].1);
                    assert!(
                        d >= mm(0.1) as f64,
                        "{}: pads {} and {} are {d} nm apart",
                        f.name,
                        hulls[i].0,
                        hulls[j].0
                    );
                }
            }
        }
    }

    #[test]
    fn bounding_boxes_are_sane() {
        for f in all() {
            let b = f.bbox();
            assert!(!b.is_empty() && b.width() < mm(40.0), "{}", f.name);
        }
    }
}
