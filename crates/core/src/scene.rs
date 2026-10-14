//! Display lists for schematics, footprints, symbols and boards.
//!
//! Everything visual is lowered to a handful of primitives here, then the
//! WebGL renderer tessellates them and the SVG exporter prints them. Both
//! therefore always agree on what a design looks like.

use crate::font;
use crate::geom::{arc_points, Point, Rect, Transform};
use crate::id::Id;
use crate::layer::{Layer, Side};
use crate::library::{Footprint, Library, PadKind, Symbol};
use crate::netlist::Connectivity;
use crate::pcb::Pcb;
use crate::schematic::{Component, PortStyle, Sheet};
use crate::shape::{Align, Shape};
use crate::units::{mil, mm};

#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// Round-capped segment. Width 0 draws a one-pixel hairline.
    Seg {
        a: Point,
        b: Point,
        w: i64,
    },
    Path {
        pts: Vec<Point>,
        w: i64,
        closed: bool,
    },
    Disc {
        c: Point,
        r: i64,
    },
    Ring {
        c: Point,
        r: i64,
        w: i64,
    },
    /// Filled polygon with optional holes.
    Poly {
        outer: Vec<Point>,
        holes: Vec<Vec<Point>>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub prim: Prim,
    /// `0xRRGGBBAA`.
    pub color: u32,
    /// Owning document object, for highlighting.
    pub owner: Option<Id>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SceneLayer {
    Frame,
    Schematic,
    Pcb(Layer),
    Drill,
    /// Pad numbers and net names; editors show it only when zoomed in.
    PadText,
    Ratsnest,
    Overlay,
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub layers: Vec<(SceneLayer, Vec<Item>)>,
}

impl Scene {
    pub fn push(&mut self, layer: SceneLayer, item: Item) {
        if let Some((_, v)) = self.layers.iter_mut().find(|(l, _)| *l == layer) {
            v.push(item);
        } else {
            self.layers.push((layer, vec![item]));
        }
    }

    pub fn bbox(&self) -> Rect {
        let mut r = Rect::EMPTY;
        for (_, items) in &self.layers {
            for it in items {
                r = r.union(prim_bbox(&it.prim));
            }
        }
        r
    }

    pub fn item_count(&self) -> usize {
        self.layers.iter().map(|(_, v)| v.len()).sum()
    }
}

pub fn prim_bbox(p: &Prim) -> Rect {
    match p {
        Prim::Seg { a, b, w } => Rect::new(*a, *b).inflate(w / 2),
        Prim::Path { pts, w, .. } => Rect::from_points(pts.iter().copied()).inflate(w / 2),
        Prim::Disc { c, r } => Rect::around(*c, *r, *r),
        Prim::Ring { c, r, w } => Rect::around(*c, r + w / 2, r + w / 2),
        Prim::Poly { outer, .. } => Rect::from_points(outer.iter().copied()),
    }
}

pub const fn rgba(rgb: u32, a: u8) -> u32 {
    (rgb << 8) | a as u32
}

/// Colours for schematic drawings.
#[derive(Clone, Copy, Debug)]
pub struct SchTheme {
    pub frame: u32,
    pub body: u32,
    pub body_fill: u32,
    pub pin: u32,
    pub pin_text: u32,
    pub designator: u32,
    pub value: u32,
    pub wire: u32,
    pub label: u32,
    pub power: u32,
    pub no_connect: u32,
    pub note: u32,
}

impl SchTheme {
    pub const LIGHT: SchTheme = SchTheme {
        frame: rgba(0x9a6a6a, 255),
        body: rgba(0x880000, 255),
        body_fill: rgba(0xffffe6, 255),
        pin: rgba(0x880000, 255),
        pin_text: rgba(0x2b2b8a, 255),
        designator: rgba(0x000080, 255),
        value: rgba(0x005050, 255),
        wire: rgba(0x0b7a2a, 255),
        label: rgba(0x0050b0, 255),
        power: rgba(0xa00000, 255),
        no_connect: rgba(0x0000ff, 255),
        note: rgba(0x444444, 255),
    };
    pub const DARK: SchTheme = SchTheme {
        frame: rgba(0x7a5555, 255),
        body: rgba(0xe06c5f, 255),
        body_fill: rgba(0x2a2a20, 255),
        pin: rgba(0xe06c5f, 255),
        pin_text: rgba(0x9fb3ff, 255),
        designator: rgba(0x8ab4ff, 255),
        value: rgba(0x5fd0c8, 255),
        wire: rgba(0x4cc26a, 255),
        label: rgba(0x6aa8ff, 255),
        power: rgba(0xff7a6a, 255),
        no_connect: rgba(0x6a8cff, 255),
        note: rgba(0xbbbbbb, 255),
    };
}

pub const PIN_TEXT: i64 = 1_016_000; // 40 mil
pub const NAME_TEXT: i64 = 1_270_000; // 50 mil
pub const LABEL_TEXT: i64 = 1_524_000; // 60 mil
const SCH_LINE: i64 = 152_400;

#[allow(clippy::too_many_arguments)]
fn text_items(
    text: &str,
    size: i64,
    pos: Point,
    rot: f64,
    align: Align,
    color: u32,
    owner: Option<Id>,
    out: &mut Vec<Item>,
) {
    let w = font::stroke_width(size);
    for stroke in font::text_strokes(text, size, pos, rot, align) {
        if stroke.len() >= 2 {
            out.push(Item {
                prim: Prim::Path {
                    pts: stroke,
                    w,
                    closed: false,
                },
                color,
                owner,
            });
        }
    }
}

/// Like [`shape_items`], but text is mirrored (bottom-side silkscreen).
pub fn shape_items_bottom(shape: &Shape, color: u32, owner: Option<Id>, out: &mut Vec<Item>) {
    match shape {
        Shape::Text {
            pos,
            text,
            size,
            rotation,
            align,
        } => {
            let w = font::stroke_width(*size);
            let mut strokes = font::text_strokes(text, *size, *pos, *rotation, *align);
            font::mirror_x(&mut strokes, pos.x);
            for pts in strokes.into_iter().filter(|s| s.len() >= 2) {
                out.push(Item {
                    prim: Prim::Path {
                        pts,
                        w,
                        closed: false,
                    },
                    color,
                    owner,
                });
            }
        }
        other => shape_items(other, color, owner, out),
    }
}

/// Lower a shape (already in world coordinates) into primitives.
pub fn shape_items(shape: &Shape, color: u32, owner: Option<Id>, out: &mut Vec<Item>) {
    let mut push = |prim: Prim| out.push(Item { prim, color, owner });
    match shape {
        Shape::Line { a, b, width } => push(Prim::Seg {
            a: *a,
            b: *b,
            w: *width,
        }),
        Shape::Polyline { points, width } => push(Prim::Path {
            pts: points.clone(),
            w: *width,
            closed: false,
        }),
        Shape::Rect { a, b, width, fill } => {
            let pts = Rect::new(*a, *b).corners().to_vec();
            if *fill {
                push(Prim::Poly {
                    outer: pts.clone(),
                    holes: vec![],
                });
            }
            push(Prim::Path {
                pts,
                w: *width,
                closed: true,
            });
        }
        Shape::Circle {
            center,
            radius,
            width,
            fill,
        } => {
            if *fill {
                push(Prim::Disc {
                    c: *center,
                    r: radius + width / 2,
                });
            } else {
                push(Prim::Ring {
                    c: *center,
                    r: *radius,
                    w: *width,
                });
            }
        }
        Shape::Arc {
            center,
            radius,
            start,
            sweep,
            width,
        } => push(Prim::Path {
            pts: arc_points(*center, *radius, *start, *sweep, 6.0),
            w: *width,
            closed: false,
        }),
        Shape::Polygon {
            points,
            width,
            fill,
        } => {
            if *fill {
                push(Prim::Poly {
                    outer: points.clone(),
                    holes: vec![],
                });
            }
            push(Prim::Path {
                pts: points.clone(),
                w: *width,
                closed: true,
            });
        }
        Shape::Text {
            pos,
            text,
            size,
            rotation,
            align,
        } => {
            text_items(text, *size, *pos, *rotation, *align, color, owner, out);
        }
    }
}

/// Draw a symbol with `tf`. Used for placed parts and the symbol editor.
pub fn symbol_items(
    sym: &Symbol,
    tf: &Transform,
    theme: &SchTheme,
    owner: Option<Id>,
    out: &mut Vec<Item>,
) {
    // IC-style symbols (named pins) get a subtle body fill; passives stay outlined.
    let is_ic = sym.pins.iter().any(|p| !p.hide_name);
    for g in &sym.graphics {
        let s = g.transformed(tf);
        if let (
            true,
            Shape::Rect {
                a, b, fill: false, ..
            },
        ) = (is_ic, &s)
        {
            out.push(Item {
                prim: Prim::Poly {
                    outer: Rect::new(*a, *b).corners().to_vec(),
                    holes: vec![],
                },
                color: theme.body_fill,
                owner,
            });
        }
        shape_items(&s, theme.body, owner, out);
    }
    for pin in &sym.pins {
        let a = tf.apply(pin.pos);
        let b = tf.apply(pin.body_end());
        out.push(Item {
            prim: Prim::Seg { a, b, w: SCH_LINE },
            color: theme.pin,
            owner,
        });
        let dir = tf.apply_angle(pin.angle);
        let horizontal = dir == 0.0 || dir == 180.0;
        let mid = Point::new((a.x + b.x) / 2, (a.y + b.y) / 2);
        // Pin number above the pin line, centred.
        let (npos, nrot) = if horizontal {
            (mid + Point::new(0, mil(15.0)), 0.0)
        } else {
            (mid + Point::new(-mil(15.0), 0), 90.0)
        };
        if is_ic || sym.pins.len() > 2 {
            text_items(
                &pin.number,
                PIN_TEXT,
                npos,
                nrot,
                Align::Center,
                theme.pin_text,
                owner,
                out,
            );
        }
        if !pin.hide_name && !pin.name.is_empty() {
            let gap = mil(30.0);
            let (pos, rot, align) = match dir as i64 {
                0 => (b + Point::new(gap, -NAME_TEXT / 2), 0.0, Align::Left),
                180 => (b + Point::new(-gap, -NAME_TEXT / 2), 0.0, Align::Right),
                90 => (b + Point::new(NAME_TEXT / 2, gap), 90.0, Align::Left),
                _ => (b + Point::new(NAME_TEXT / 2, -gap), 90.0, Align::Right),
            };
            text_items(
                &pin.name, NAME_TEXT, pos, rot, align, theme.body, owner, out,
            );
        }
    }
}

/// Default label positions for a placed part: designator above the body,
/// value below it, both left-aligned with the body.
pub fn component_label_positions(c: &Component, lib: &Library) -> (Point, Point) {
    let b = c.bbox(lib);
    let des = c
        .designator_offset
        .map(|o| c.pos + o)
        .unwrap_or(Point::new(b.min.x, b.max.y + mil(25.0)));
    let val = c
        .value_offset
        .map(|o| c.pos + o)
        .unwrap_or(Point::new(b.min.x, b.min.y - LABEL_TEXT - mil(25.0)));
    (des, val)
}

pub fn component_items(c: &Component, lib: &Library, theme: &SchTheme, out: &mut Vec<Item>) {
    let owner = Some(c.id);
    match lib
        .devices
        .get(&c.device)
        .and_then(|d| lib.symbols.get(&d.symbol))
    {
        Some(sym) => symbol_items(sym, &c.transform(), theme, owner, out),
        None => {
            let r = Rect::around(c.pos, mil(100.0), mil(100.0));
            out.push(Item {
                prim: Prim::Path {
                    pts: r.corners().to_vec(),
                    w: SCH_LINE,
                    closed: true,
                },
                color: rgba(0xff0000, 255),
                owner,
            });
            text_items(
                "?",
                LABEL_TEXT,
                c.pos,
                0.0,
                Align::Center,
                rgba(0xff0000, 255),
                owner,
                out,
            );
        }
    }
    let (dp, vp) = component_label_positions(c, lib);
    text_items(
        &c.designator,
        LABEL_TEXT,
        dp,
        0.0,
        Align::Left,
        theme.designator,
        owner,
        out,
    );
    if !c.value.is_empty() {
        text_items(
            &c.value,
            LABEL_TEXT,
            vp,
            0.0,
            Align::Left,
            theme.value,
            owner,
            out,
        );
    }
}

fn port_items(
    name: &str,
    pos: Point,
    style: PortStyle,
    rot: f64,
    theme: &SchTheme,
    owner: Option<Id>,
    out: &mut Vec<Item>,
) {
    let tf = Transform::new(pos, rot, false);
    let ap = |x: f64, y: f64| tf.apply(Point::new(mil(x), mil(y)));
    let c = theme.power;
    match style {
        PortStyle::Power => {
            out.push(Item {
                prim: Prim::Seg {
                    a: ap(0.0, 0.0),
                    b: ap(0.0, 60.0),
                    w: SCH_LINE,
                },
                color: c,
                owner,
            });
            out.push(Item {
                prim: Prim::Seg {
                    a: ap(-50.0, 60.0),
                    b: ap(50.0, 60.0),
                    w: SCH_LINE * 2,
                },
                color: c,
                owner,
            });
            text_items(
                name,
                LABEL_TEXT,
                ap(0.0, 90.0),
                0.0,
                Align::Center,
                c,
                owner,
                out,
            );
        }
        PortStyle::Ground => {
            out.push(Item {
                prim: Prim::Seg {
                    a: ap(0.0, 0.0),
                    b: ap(0.0, -50.0),
                    w: SCH_LINE,
                },
                color: c,
                owner,
            });
            for (i, half) in [60.0, 38.0, 16.0].iter().enumerate() {
                let y = -50.0 - i as f64 * 22.0;
                out.push(Item {
                    prim: Prim::Seg {
                        a: ap(-half, y),
                        b: ap(*half, y),
                        w: SCH_LINE,
                    },
                    color: c,
                    owner,
                });
            }
            text_items(
                name,
                PIN_TEXT,
                ap(0.0, -150.0),
                0.0,
                Align::Center,
                c,
                owner,
                out,
            );
        }
        PortStyle::Port => {
            let w = 40.0 + font::text_width(name, LABEL_TEXT) as f64 / mil(1.0) as f64;
            let pts = vec![
                ap(0.0, 0.0),
                ap(30.0, 40.0),
                ap(30.0 + w, 40.0),
                ap(30.0 + w, -40.0),
                ap(30.0, -40.0),
            ];
            out.push(Item {
                prim: Prim::Path {
                    pts,
                    w: SCH_LINE,
                    closed: true,
                },
                color: theme.label,
                owner,
            });
            text_items(
                name,
                LABEL_TEXT,
                ap(50.0, -30.0),
                0.0,
                Align::Left,
                theme.label,
                owner,
                out,
            );
        }
    }
}

fn label_items(l: &crate::schematic::NetLabel, theme: &SchTheme, out: &mut Vec<Item>) {
    let o = Some(l.id);
    // Text reads left-to-right or bottom-to-top and extends away from the wire.
    let (rot, align, dx) = match crate::geom::normalize_angle(l.rotation) as i64 {
        180 => (0.0, Align::Right, -mil(10.0)),
        270 => (90.0, Align::Right, -mil(10.0)),
        r => (r as f64, Align::Left, mil(10.0)),
    };
    let pos = l.pos + crate::geom::rotate(Point::new(dx, mil(15.0)), rot);
    text_items(&l.name, LABEL_TEXT, pos, rot, align, theme.label, o, out);
    out.push(Item {
        prim: Prim::Disc {
            c: l.pos,
            r: mil(12.0),
        },
        color: theme.label,
        owner: o,
    });
}

/// Bounding box of a net label as drawn.
pub fn label_bbox(l: &crate::schematic::NetLabel) -> Rect {
    let mut v = Vec::new();
    label_items(l, &SchTheme::LIGHT, &mut v);
    v.iter()
        .fold(Rect::EMPTY, |r, it| r.union(prim_bbox(&it.prim)))
}

/// Bounding box of a power/ground flag or sheet port as drawn.
pub fn port_bbox(port: &crate::schematic::NetPort) -> Rect {
    let mut v = Vec::new();
    port_items(
        &port.name,
        port.pos,
        port.style,
        port.rotation,
        &SchTheme::LIGHT,
        None,
        &mut v,
    );
    v.iter()
        .fold(Rect::EMPTY, |r, it| r.union(prim_bbox(&it.prim)))
}

/// Wires and labels of `net` drawn bold in `color`, to lay over a sheet.
pub fn net_highlight_items(sheet: &Sheet, conn: &Connectivity, net: &str, color: u32) -> Vec<Item> {
    let mut v = Vec::new();
    for w in &sheet.wires {
        let on = conn.wire_net.get(&w.id).is_some_and(|&i| {
            conn.nets[i].name == net || conn.nets[i].aliases.iter().any(|a| a == net)
        });
        if on {
            v.push(Item {
                prim: Prim::Path {
                    pts: w.points.clone(),
                    w: SCH_LINE * 3,
                    closed: false,
                },
                color,
                owner: None,
            });
        }
    }
    for l in sheet.labels.iter().filter(|l| l.name == net) {
        v.push(Item {
            prim: Prim::Disc {
                c: l.pos,
                r: mil(30.0),
            },
            color,
            owner: None,
        });
    }
    v
}

/// Everything on a schematic sheet.
pub fn sheet_scene(
    sheet: &Sheet,
    lib: &Library,
    conn: Option<(&Connectivity, usize)>,
    theme: &SchTheme,
    title: &str,
) -> Scene {
    let mut scene = Scene::default();
    let mut frame = Vec::new();
    frame_items(sheet, theme, title, &mut frame);
    scene.layers.push((SceneLayer::Frame, frame));

    let mut v = Vec::new();
    for a in &sheet.annotations {
        shape_items(&a.shape, theme.note, Some(a.id), &mut v);
    }
    for w in &sheet.wires {
        v.push(Item {
            prim: Prim::Path {
                pts: w.points.clone(),
                w: SCH_LINE,
                closed: false,
            },
            color: theme.wire,
            owner: Some(w.id),
        });
    }
    if let Some((conn, idx)) = conn {
        if let Some(dots) = conn.junctions.get(idx) {
            for d in dots {
                v.push(Item {
                    prim: Prim::Disc {
                        c: *d,
                        r: mil(25.0),
                    },
                    color: theme.wire,
                    owner: None,
                });
            }
        }
    }
    for c in &sheet.components {
        component_items(c, lib, theme, &mut v);
    }
    for l in &sheet.labels {
        label_items(l, theme, &mut v);
    }
    for p in &sheet.ports {
        port_items(
            &p.name,
            p.pos,
            p.style,
            p.rotation,
            theme,
            Some(p.id),
            &mut v,
        );
    }
    for n in &sheet.no_connects {
        let d = mil(30.0);
        let o = Some(n.id);
        v.push(Item {
            prim: Prim::Seg {
                a: n.pos + Point::new(-d, -d),
                b: n.pos + Point::new(d, d),
                w: SCH_LINE,
            },
            color: theme.no_connect,
            owner: o,
        });
        v.push(Item {
            prim: Prim::Seg {
                a: n.pos + Point::new(-d, d),
                b: n.pos + Point::new(d, -d),
                w: SCH_LINE,
            },
            color: theme.no_connect,
            owner: o,
        });
    }
    scene.layers.push((SceneLayer::Schematic, v));
    scene
}

fn frame_items(sheet: &Sheet, theme: &SchTheme, title: &str, out: &mut Vec<Item>) {
    let f = sheet.frame();
    let c = theme.frame;
    let w = mm(0.3);
    let inner = f.inflate(-mm(5.0));
    out.push(Item {
        prim: Prim::Path {
            pts: f.corners().to_vec(),
            w,
            closed: true,
        },
        color: c,
        owner: None,
    });
    out.push(Item {
        prim: Prim::Path {
            pts: inner.corners().to_vec(),
            w,
            closed: true,
        },
        color: c,
        owner: None,
    });
    // Zone markers along the border.
    let cols = (inner.width() / mm(50.0)).max(1);
    for i in 0..cols {
        let x = inner.min.x + inner.width() * (2 * i + 1) / (2 * cols);
        text_items(
            &(i + 1).to_string(),
            mm(2.5),
            Point::new(x, inner.max.y + mm(1.2)),
            0.0,
            Align::Center,
            c,
            None,
            out,
        );
        text_items(
            &(i + 1).to_string(),
            mm(2.5),
            Point::new(x, f.min.y + mm(1.2)),
            0.0,
            Align::Center,
            c,
            None,
            out,
        );
    }
    let rows = (inner.height() / mm(50.0)).max(1);
    for i in 0..rows {
        let y = inner.max.y - inner.height() * (2 * i + 1) / (2 * rows);
        let letter = ((b'A' + i as u8) as char).to_string();
        text_items(
            &letter,
            mm(2.5),
            Point::new(f.min.x + mm(2.5), y),
            0.0,
            Align::Center,
            c,
            None,
            out,
        );
        text_items(
            &letter,
            mm(2.5),
            Point::new(inner.max.x + mm(2.5), y),
            0.0,
            Align::Center,
            c,
            None,
            out,
        );
    }
    // Title block, bottom right.
    let tb = Rect::new(
        Point::new(inner.max.x - mm(110.0), inner.min.y),
        Point::new(inner.max.x, inner.min.y + mm(24.0)),
    );
    out.push(Item {
        prim: Prim::Path {
            pts: tb.corners().to_vec(),
            w,
            closed: true,
        },
        color: c,
        owner: None,
    });
    for y in [mm(8.0), mm(16.0)] {
        out.push(Item {
            prim: Prim::Seg {
                a: Point::new(tb.min.x, tb.min.y + y),
                b: Point::new(tb.max.x, tb.min.y + y),
                w,
            },
            color: c,
            owner: None,
        });
    }
    out.push(Item {
        prim: Prim::Seg {
            a: Point::new(tb.min.x + mm(70.0), tb.min.y),
            b: Point::new(tb.min.x + mm(70.0), tb.min.y + mm(16.0)),
            w,
        },
        color: c,
        owner: None,
    });
    let t = |s: &str, x: i64, y: i64, size: f64, out: &mut Vec<Item>| {
        text_items(
            s,
            mm(size),
            Point::new(tb.min.x + x, tb.min.y + y),
            0.0,
            Align::Left,
            c,
            None,
            out,
        )
    };
    t(&format!("TITLE: {title}"), mm(2.0), mm(18.5), 3.0, out);
    t(
        &format!("SHEET: {}", sheet.name),
        mm(2.0),
        mm(10.5),
        2.5,
        out,
    );
    t(
        &format!("SIZE: {}", sheet.paper.label()),
        mm(72.0),
        mm(10.5),
        2.5,
        out,
    );
    t("Drawn with EasyPRO", mm(2.0), mm(2.5), 2.5, out);
    t("REV: 1.0", mm(72.0), mm(2.5), 2.5, out);
}

/// Options for drawing a board.
#[derive(Clone, Debug)]
pub struct PcbStyle {
    pub hidden: Vec<Layer>,
    /// Net to highlight; other copper is dimmed.
    pub highlight_net: Option<String>,
    pub show_ratsnest: bool,
}

impl Default for PcbStyle {
    fn default() -> Self {
        PcbStyle {
            hidden: vec![Layer::TopAssembly, Layer::BottomAssembly, Layer::Document],
            highlight_net: None,
            show_ratsnest: true,
        }
    }
}

fn layer_color(l: Layer, alpha: u8) -> u32 {
    rgba(l.color(), alpha)
}

/// Footprint drawn in its own coordinates (footprint editor).
pub fn footprint_scene(fp: &Footprint) -> Scene {
    let mut scene = Scene::default();
    footprint_items(
        fp,
        &Transform::default(),
        Side::Top,
        None,
        &mut scene,
        |_| 230,
        None,
    );
    scene
}

fn footprint_items(
    fp: &Footprint,
    tf: &Transform,
    side: Side,
    owner: Option<Id>,
    scene: &mut Scene,
    alpha_for_net: impl Fn(&str) -> u8,
    nets: Option<&std::collections::BTreeMap<String, String>>,
) {
    let flip = |l: Layer| if side == Side::Bottom { l.flipped() } else { l };
    for g in &fp.graphics {
        let layer = flip(g.layer);
        let mut v = Vec::new();
        let shape = g.shape.transformed(tf);
        if side == Side::Bottom {
            shape_items_bottom(&shape, layer_color(layer, 255), owner, &mut v);
        } else {
            shape_items(&shape, layer_color(layer, 255), owner, &mut v);
        }
        for it in v {
            scene.push(SceneLayer::Pcb(layer), it);
        }
    }
    for pad in &fp.pads {
        let hull = pad.hull(tf);
        let net = nets
            .and_then(|n| n.get(&pad.number))
            .map(|s| s.as_str())
            .unwrap_or("");
        let a = alpha_for_net(net);
        let layer = match pad.kind {
            PadKind::Smd => flip(Layer::TopCopper),
            PadKind::Tht { .. } => Layer::Multi,
            PadKind::Npth { .. } => Layer::Multi,
        };
        if !matches!(pad.kind, PadKind::Npth { .. }) {
            scene.push(
                SceneLayer::Pcb(layer),
                Item {
                    prim: Prim::Poly {
                        outer: hull.to_polygon(),
                        holes: vec![],
                    },
                    color: layer_color(layer, a),
                    owner,
                },
            );
        }
        if let Some(d) = pad.drill() {
            let c = tf.apply(pad.pos);
            scene.push(
                SceneLayer::Drill,
                Item {
                    prim: Prim::Disc { c, r: d / 2 },
                    color: rgba(0x202020, 255),
                    owner,
                },
            );
        }
        if nets.is_some() && !matches!(pad.kind, PadKind::Npth { .. }) {
            pad_label_items(pad, tf, net, scene);
        }
    }
}

/// Pad number (and net name below it) sized to fit inside the pad.
fn pad_label_items(pad: &crate::library::Pad, tf: &Transform, net: &str, scene: &mut Scene) {
    let b = pad.hull(tf).bbox();
    let (w, h) = (b.width() as f64, b.height() as f64);
    let lines: Vec<&str> = if net.is_empty() {
        vec![&pad.number]
    } else {
        vec![&pad.number, net]
    };
    let fit = |text: &str, max_h: f64| -> i64 {
        let chars = text.chars().count().max(1) as f64;
        // Glyph advance is about 0.93 of the cap height.
        (max_h.min(w * 0.85 / (chars * 0.93))) as i64
    };
    let c = b.center();
    let rows = lines.len() as f64;
    let mut y = c.y as f64 + (rows - 1.0) * h * 0.18;
    for (i, text) in lines.iter().enumerate() {
        let size = fit(text, h * if i == 0 { 0.42 } else { 0.28 } / rows.sqrt());
        if size < 20_000 {
            continue;
        }
        let mut v = Vec::new();
        let color = if i == 0 {
            rgba(0xffffff, 230)
        } else {
            rgba(0xfff3b0, 220)
        };
        text_items(
            text,
            size,
            Point::new(c.x, (y - size as f64 / 2.0) as i64),
            0.0,
            Align::Center,
            color,
            None,
            &mut v,
        );
        for it in v {
            scene.push(SceneLayer::PadText, it);
        }
        y -= h * 0.36;
    }
}

/// Everything on a board.
pub fn pcb_scene(
    pcb: &Pcb,
    lib: &Library,
    style: &PcbStyle,
    ratsnest: &[crate::connect::RatLine],
) -> Scene {
    let mut scene = Scene::default();
    let hl = style.highlight_net.as_deref();
    let alpha = |net: &str| -> u8 {
        match hl {
            Some(h) if h == net => 255,
            Some(_) => 70,
            None => 210,
        }
    };
    // Fixed back-to-front order; the app reorders to raise the active layer.
    let mut order: Vec<Layer> = vec![
        Layer::BottomAssembly,
        Layer::BottomSilk,
        Layer::BottomPaste,
        Layer::BottomMask,
    ];
    order.extend(Layer::copper_stack(pcb.copper_layers).into_iter().rev());
    order.extend([
        Layer::Multi,
        Layer::TopMask,
        Layer::TopPaste,
        Layer::TopSilk,
        Layer::TopAssembly,
        Layer::Document,
        Layer::Outline,
    ]);
    for l in &order {
        scene.layers.push((SceneLayer::Pcb(*l), vec![]));
    }

    for p in &pcb.pours {
        for r in &p.filled {
            scene.push(
                SceneLayer::Pcb(p.layer),
                Item {
                    prim: Prim::Poly {
                        outer: r.outer.clone(),
                        holes: r.holes.clone(),
                    },
                    color: layer_color(p.layer, alpha(&p.net) / 3 + 10),
                    owner: Some(p.id),
                },
            );
        }
        scene.push(
            SceneLayer::Pcb(p.layer),
            Item {
                prim: Prim::Path {
                    pts: p.outline.clone(),
                    w: 0,
                    closed: true,
                },
                color: layer_color(p.layer, 255),
                owner: Some(p.id),
            },
        );
    }
    for t in &pcb.tracks {
        scene.push(
            SceneLayer::Pcb(t.layer),
            Item {
                prim: Prim::Seg {
                    a: t.a,
                    b: t.b,
                    w: t.width,
                },
                color: layer_color(t.layer, alpha(&t.net)),
                owner: Some(t.id),
            },
        );
    }
    for c in &pcb.components {
        let Some(fp) = lib.footprints.get(&c.footprint) else {
            continue;
        };
        let tf = c.transform();
        footprint_items(
            fp,
            &tf,
            c.side,
            Some(c.id),
            &mut scene,
            alpha,
            Some(&c.pad_nets),
        );
        if c.show_designator {
            let silk = c.board_layer(Layer::TopSilk);
            let pos = c.designator_pos(fp);
            let mut v = Vec::new();
            let size = mm(1.0);
            let text = Shape::Text {
                pos: pos - Point::new(0, size / 2),
                text: c.designator.clone(),
                size,
                rotation: 0.0,
                align: Align::Center,
            };
            if c.side == Side::Bottom {
                shape_items_bottom(&text, layer_color(silk, 255), Some(c.id), &mut v);
            } else {
                shape_items(&text, layer_color(silk, 255), Some(c.id), &mut v);
            }
            for it in v {
                scene.push(SceneLayer::Pcb(silk), it);
            }
        }
    }
    for v in &pcb.vias {
        let a = alpha(&v.net);
        scene.push(
            SceneLayer::Pcb(Layer::Multi),
            Item {
                prim: Prim::Disc {
                    c: v.pos,
                    r: v.diameter / 2,
                },
                color: rgba(0xb0b0b0, a),
                owner: Some(v.id),
            },
        );
        scene.push(
            SceneLayer::Drill,
            Item {
                prim: Prim::Disc {
                    c: v.pos,
                    r: v.drill / 2,
                },
                color: rgba(0x202020, 255),
                owner: Some(v.id),
            },
        );
    }
    for g in &pcb.graphics {
        let mut v = Vec::new();
        let draw = if matches!(g.layer, Layer::BottomSilk | Layer::BottomCopper) {
            shape_items_bottom
        } else {
            shape_items
        };
        draw(&g.shape, layer_color(g.layer, 255), Some(g.id), &mut v);
        for it in v {
            scene.push(SceneLayer::Pcb(g.layer), it);
        }
    }
    if style.show_ratsnest {
        let items = ratsnest
            .iter()
            .map(|r| Item {
                prim: Prim::Seg {
                    a: r.a,
                    b: r.b,
                    w: 0,
                },
                color: rgba(0xf0f0f0, 200),
                owner: None,
            })
            .collect();
        scene.layers.push((SceneLayer::Ratsnest, items));
    }
    scene
        .layers
        .retain(|(l, _)| !matches!(l, SceneLayer::Pcb(x) if style.hidden.contains(x)));
    scene
}

/// A symbol on its own (symbol editor / library preview).
pub fn symbol_scene(sym: &Symbol, theme: &SchTheme) -> Scene {
    let mut v = Vec::new();
    symbol_items(sym, &Transform::default(), theme, None, &mut v);
    let mut s = Scene::default();
    s.layers.push((SceneLayer::Schematic, v));
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_scenes_are_populated() {
        let p = crate::examples::blinker();
        let b = &p.boards[0];
        let conn = crate::netlist::connectivity(&b.schematic, &p.library);
        let s = sheet_scene(
            &b.schematic.sheets[0],
            &p.library,
            Some((&conn, 0)),
            &SchTheme::LIGHT,
            &p.name,
        );
        assert!(s.item_count() > 100);
        let pcb = pcb_scene(&b.pcb, &p.library, &PcbStyle::default(), &[]);
        assert!(pcb.item_count() > 50);
        let bb = pcb.bbox();
        assert!(bb.width() >= mm(50.0));
    }
}
