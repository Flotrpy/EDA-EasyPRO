//! Design rule check for PCBs.

use crate::check::{sort, Issue, Severity};
use crate::connect::{analyze_items, index, near_pairs};
use crate::geom::{point_in_polygon, Point, Rect};
use crate::id::Id;
use crate::library::{Hull, Library, PadKind};
use crate::pcb::{CopperItem, CopperRef, Pcb};
use crate::units::Unit;

fn ids(it: &CopperItem) -> Option<Id> {
    match &it.what {
        CopperRef::Pad { comp, .. } => Some(*comp),
        CopperRef::Track(id) | CopperRef::Via(id) | CopperRef::Pour(id, _) => Some(*id),
    }
}

fn describe(it: &CopperItem, pcb: &Pcb) -> String {
    match &it.what {
        CopperRef::Pad { comp, pad } => {
            let d = pcb
                .component(*comp)
                .map(|c| c.designator.as_str())
                .unwrap_or("?");
            format!("pad {d}.{pad}")
        }
        CopperRef::Track(_) => "track".into(),
        CopperRef::Via(_) => "via".into(),
        CopperRef::Pour(..) => "copper area".into(),
    }
}

fn net_label(n: &str) -> &str {
    if n.is_empty() {
        "<no net>"
    } else {
        n
    }
}

fn midpoint(a: &Hull, b: &Hull) -> Point {
    let ca = Rect::from_points(a.points.iter().copied()).center();
    let cb = Rect::from_points(b.points.iter().copied()).center();
    Point::new((ca.x + cb.x) / 2, (ca.y + cb.y) / 2)
}

pub fn run(pcb: &Pcb, lib: &Library, unit: Unit) -> Vec<Issue> {
    let r = &pcb.rules;
    let items = pcb.copper_items(lib);
    let tree = index(&items);
    let max_clear = r
        .net_classes
        .iter()
        .map(|c| c.clearance)
        .chain([r.clearance])
        .max()
        .unwrap_or(0);
    let mut out = Vec::new();
    let fmt = |v: i64| format!("{} {}", unit.format(v), unit.label());

    // Clearance and shorts between different nets.
    for (i, j) in near_pairs(&items, &tree, max_clear) {
        let (a, b) = (&items[i], &items[j]);
        let same_net = !a.net.is_empty() && a.net == b.net;
        if same_net {
            continue;
        }
        if let (CopperRef::Pad { comp: ca, .. }, CopperRef::Pad { comp: cb, .. }) =
            (&a.what, &b.what)
        {
            if ca == cb {
                continue;
            }
        }
        // Fills are built around other-net copper already; only check them against each other.
        let pa = matches!(a.what, CopperRef::Pour(..));
        let pb = matches!(b.what, CopperRef::Pour(..));
        if pa != pb {
            continue;
        }
        let need = r.clearance_for(&a.net, &b.net);
        let d = a.hull.distance(&b.hull);
        let pos = midpoint(&a.hull, &b.hull);
        let mut issue = if d <= 0.0 {
            if a.net.is_empty() && b.net.is_empty() {
                continue;
            }
            Issue::new(
                Severity::Error,
                format!(
                    "Short: {} ({}) touches {} ({})",
                    describe(a, pcb),
                    net_label(&a.net),
                    describe(b, pcb),
                    net_label(&b.net)
                ),
            )
        } else if (d as i64) < need {
            Issue::new(
                Severity::Error,
                format!(
                    "Clearance {} < {}: {} ({}) and {} ({})",
                    fmt(d as i64),
                    fmt(need),
                    describe(a, pcb),
                    net_label(&a.net),
                    describe(b, pcb),
                    net_label(&b.net)
                ),
            )
        } else {
            continue;
        };
        issue = issue.at(pos);
        if let Some(id) = ids(a) {
            issue = issue.with(id);
        }
        if let Some(id) = ids(b) {
            issue = issue.with(id);
        }
        out.push(issue);
    }

    // Track widths.
    for t in &pcb.tracks {
        if t.width < r.min_track_width {
            out.push(
                Issue::new(
                    Severity::Error,
                    format!(
                        "Track width {} below minimum {}",
                        fmt(t.width),
                        fmt(r.min_track_width)
                    ),
                )
                .at(Point::new((t.a.x + t.b.x) / 2, (t.a.y + t.b.y) / 2))
                .with(t.id),
            );
        }
    }

    // Drills and annular rings.
    let mut holes: Vec<(Point, i64, Option<Id>)> = Vec::new();
    for v in &pcb.vias {
        if v.drill < r.min_drill {
            out.push(
                Issue::new(
                    Severity::Error,
                    format!(
                        "Via drill {} below minimum {}",
                        fmt(v.drill),
                        fmt(r.min_drill)
                    ),
                )
                .at(v.pos)
                .with(v.id),
            );
        }
        let ring = (v.diameter - v.drill) / 2;
        if ring < r.min_annular_ring {
            out.push(
                Issue::new(
                    Severity::Error,
                    format!(
                        "Via annular ring {} below minimum {}",
                        fmt(ring),
                        fmt(r.min_annular_ring)
                    ),
                )
                .at(v.pos)
                .with(v.id),
            );
        }
        holes.push((v.pos, v.drill, Some(v.id)));
    }
    for (c, pad, _) in pcb.pads(lib) {
        if let PadKind::Tht { drill } | PadKind::Npth { drill } = pad.kind {
            let pos = c.transform().apply(pad.pos);
            if drill < r.min_drill {
                out.push(
                    Issue::new(
                        Severity::Error,
                        format!(
                            "Pad {}.{} drill {} below minimum {}",
                            c.designator,
                            pad.number,
                            fmt(drill),
                            fmt(r.min_drill)
                        ),
                    )
                    .at(pos)
                    .with(c.id),
                );
            }
            if matches!(pad.kind, PadKind::Tht { .. }) {
                let ring = (pad.size.x.min(pad.size.y) - drill) / 2;
                if ring < r.min_annular_ring {
                    out.push(
                        Issue::new(
                            Severity::Warning,
                            format!(
                                "Pad {}.{} annular ring {} below {}",
                                c.designator,
                                pad.number,
                                fmt(ring),
                                fmt(r.min_annular_ring)
                            ),
                        )
                        .at(pos)
                        .with(c.id),
                    );
                }
            }
            holes.push((pos, drill, Some(c.id)));
        }
    }
    let min_hole_gap = crate::units::mm(0.25);
    for i in 0..holes.len() {
        for j in i + 1..holes.len() {
            let (pa, da, ia) = holes[i];
            let (pb, db, _) = holes[j];
            let gap = pa.dist(pb) - (da + db) as f64 / 2.0;
            if gap < min_hole_gap as f64 && pa != pb {
                let mut is = Issue::new(
                    Severity::Error,
                    format!("Holes too close: {} gap", fmt(gap.max(0.0) as i64)),
                )
                .at(pa);
                if let Some(id) = ia {
                    is = is.with(id);
                }
                out.push(is);
            }
        }
    }

    // Board edge.
    match pcb.outline() {
        None => out.push(Issue::new(
            Severity::Error,
            "Board outline is missing or not closed",
        )),
        Some(outline) => {
            let n = outline.len();
            let edges: Vec<Hull> = (0..n)
                .map(|i| Hull::capsule(outline[i], outline[(i + 1) % n], 0))
                .collect();
            for it in &items {
                if matches!(it.what, CopperRef::Pour(..)) {
                    continue;
                }
                let inside = it
                    .hull
                    .points
                    .iter()
                    .all(|p| point_in_polygon(*p, &outline));
                let d = edges
                    .iter()
                    .map(|e| e.distance(&it.hull))
                    .fold(f64::INFINITY, f64::min);
                let pos = Rect::from_points(it.hull.points.iter().copied()).center();
                if !inside {
                    let mut is = Issue::new(
                        Severity::Error,
                        format!("{} is outside the board", describe(it, pcb)),
                    )
                    .at(pos);
                    if let Some(id) = ids(it) {
                        is = is.with(id);
                    }
                    out.push(is);
                } else if (d as i64) < r.edge_clearance {
                    let mut is = Issue::new(
                        Severity::Error,
                        format!(
                            "{} is {} from the board edge (min {})",
                            describe(it, pcb),
                            fmt(d as i64),
                            fmt(r.edge_clearance)
                        ),
                    )
                    .at(pos);
                    if let Some(id) = ids(it) {
                        is = is.with(id);
                    }
                    out.push(is);
                }
            }
        }
    }

    placement_checks(pcb, lib, &mut out);
    silk_checks(pcb, &mut out, &fmt);
    dangling_tracks(pcb, &items, &mut out);

    // Connectivity.
    let conn = analyze_items(&items);
    for (net, missing) in &conn.unrouted {
        let pos = conn.ratsnest.iter().find(|l| &l.net == net).map(|l| l.a);
        let mut is = Issue::new(
            Severity::Warning,
            format!("Net {net} has {missing} unrouted connection(s)"),
        );
        is.pos = pos;
        out.push(is);
    }

    sort(&mut out);
    out
}

/// Tracks with an end that touches no other copper, with that end:
/// leftovers from deleting parts or abandoned routes, which act as antennas
/// and confuse the fab netlist.
pub fn dangling_ends(pcb: &Pcb, items: &[CopperItem]) -> Vec<(Id, Point)> {
    let boxes: Vec<Rect> = items.iter().map(|it| it.hull.bbox()).collect();
    let mut out = Vec::new();
    for t in &pcb.tracks {
        let free = [t.a, t.b].into_iter().find(|&end| {
            let probe = Hull::circle(end, t.width / 2);
            !items.iter().zip(&boxes).any(|(it, bb)| {
                !matches!(it.what, CopperRef::Track(id) if id == t.id)
                    && it.layer.shares_copper(t.layer)
                    && bb.inflate(t.width).contains(end)
                    && it.hull.distance(&probe) <= 0.0
            })
        });
        if let Some(end) = free {
            out.push((t.id, end));
        }
    }
    out
}

fn dangling_tracks(pcb: &Pcb, items: &[CopperItem], out: &mut Vec<Issue>) {
    for (id, end) in dangling_ends(pcb, items) {
        out.push(
            Issue::new(Severity::Warning, "Track end is not connected to anything")
                .at(end)
                .with(id),
        );
    }
}

/// Delete dangling tracks until none are left (removing one stub can leave
/// the segment before it dangling). Returns the removed ids.
pub fn remove_dangling_tracks(pcb: &mut Pcb, lib: &Library) -> Vec<Id> {
    let mut removed = Vec::new();
    loop {
        let items = pcb.copper_items(lib);
        let ids: Vec<Id> = dangling_ends(pcb, &items)
            .into_iter()
            .map(|d| d.0)
            .collect();
        if ids.is_empty() {
            return removed;
        }
        pcb.tracks.retain(|t| !ids.contains(&t.id));
        removed.extend(ids);
    }
}

/// Board silkscreen that most fabs cannot print legibly.
fn silk_checks(pcb: &Pcb, out: &mut Vec<Issue>, fmt: &dyn Fn(i64) -> String) {
    use crate::layer::Layer;
    use crate::shape::Shape;
    const MIN_LINE: i64 = 100_000;
    const MIN_TEXT: i64 = 700_000;
    for g in pcb
        .graphics
        .iter()
        .filter(|g| matches!(g.layer, Layer::TopSilk | Layer::BottomSilk))
    {
        let problem = match &g.shape {
            Shape::Text { size, .. } if *size < MIN_TEXT => Some(format!(
                "Silkscreen text {} high is below {}",
                fmt(*size),
                fmt(MIN_TEXT)
            )),
            Shape::Text { .. } => None,
            Shape::Line { width, .. }
            | Shape::Polyline { width, .. }
            | Shape::Rect { width, .. }
            | Shape::Polygon { width, .. }
            | Shape::Arc { width, .. }
            | Shape::Circle { width, .. } => (*width < MIN_LINE && *width > 0).then(|| {
                format!(
                    "Silkscreen line {} wide is below {}",
                    fmt(*width),
                    fmt(MIN_LINE)
                )
            }),
        };
        if let Some(msg) = problem {
            out.push(
                Issue::new(Severity::Warning, msg)
                    .at(g.shape.bbox().center())
                    .with(g.id),
            );
        }
    }
}

/// Parts sitting on top of each other, and silkscreen printed over
/// exposed copper (fabs clip it, which mangles outlines and text).
fn placement_checks(pcb: &Pcb, lib: &Library, out: &mut Vec<Issue>) {
    use crate::layer::{Layer, Side};
    use crate::shape::Shape;
    struct Placed<'a> {
        c: &'a crate::pcb::PcbComponent,
        pads: Vec<(Hull, bool)>,
        body: Rect,
        silk: Vec<Hull>,
        reach: Rect,
    }
    let mut placed = Vec::new();
    for c in &pcb.components {
        let Some(fp) = lib.footprints.get(&c.footprint) else {
            continue;
        };
        let tf = c.transform();
        let pads: Vec<(Hull, bool)> = fp
            .pads
            .iter()
            .filter(|p| !matches!(p.kind, PadKind::Npth { .. }))
            .map(|p| (p.hull(&tf), matches!(p.kind, PadKind::Tht { .. })))
            .collect();
        let body = pads.iter().fold(Rect::EMPTY, |r, (h, _)| r.union(h.bbox()));
        let mut silk = Vec::new();
        for g in fp.graphics.iter().filter(|g| g.layer == Layer::TopSilk) {
            let w = |s: &Shape| match s {
                Shape::Line { width, .. }
                | Shape::Polyline { width, .. }
                | Shape::Rect { width, .. }
                | Shape::Polygon { width, .. }
                | Shape::Arc { width, .. }
                | Shape::Circle { width, .. } => *width,
                Shape::Text { .. } => 0,
            };
            let shape = g.shape.transformed(&tf);
            let hw = w(&shape) / 2;
            let pts: Vec<Point> = match &shape {
                Shape::Line { a, b, .. } => vec![*a, *b],
                Shape::Polyline { points, .. } => points.clone(),
                Shape::Rect { a, b, .. } => {
                    let mut v = Rect::new(*a, *b).corners().to_vec();
                    v.push(v[0]);
                    v
                }
                _ => continue,
            };
            for s in pts.windows(2) {
                silk.push(Hull::capsule(s[0], s[1], hw));
            }
        }
        let reach = silk.iter().fold(body, |r, h| r.union(h.bbox()));
        placed.push(Placed {
            c,
            pads,
            body,
            silk,
            reach,
        });
    }
    for i in 0..placed.len() {
        for j in 0..placed.len() {
            if i == j {
                continue;
            }
            let (a, b) = (&placed[i], &placed[j]);
            // Cheap reject: parts whose copper and silk are far apart.
            if !a.reach.intersects(&b.reach) {
                continue;
            }
            if i < j
                && a.c.side == b.c.side
                && !a.body.is_empty()
                && !b.body.is_empty()
                && a.body.intersects(&b.body)
            {
                let overlap = Rect::new(
                    Point::new(
                        a.body.min.x.max(b.body.min.x),
                        a.body.min.y.max(b.body.min.y),
                    ),
                    Point::new(
                        a.body.max.x.min(b.body.max.x),
                        a.body.max.y.min(b.body.max.y),
                    ),
                );
                if overlap.width() > 50_000 && overlap.height() > 50_000 {
                    out.push(
                        Issue::new(
                            Severity::Warning,
                            format!("{} overlaps {}", a.c.designator, b.c.designator),
                        )
                        .at(overlap.center())
                        .with(a.c.id)
                        .with(b.c.id),
                    );
                }
            }
            // Silk of `a` (top side only for now) over exposed pads of `b`.
            if a.c.side == Side::Top
                && (b.c.side == Side::Top || b.pads.iter().any(|(_, tht)| *tht))
            {
                let hit = a.silk.iter().find_map(|s| {
                    b.pads
                        .iter()
                        .find(|(p, _)| s.distance(p) <= 0.0)
                        .map(|(p, _)| (s, p))
                });
                if let Some((_, p)) = hit {
                    out.push(
                        Issue::new(
                            Severity::Warning,
                            format!(
                                "Silkscreen of {} is printed over a pad of {}",
                                a.c.designator, b.c.designator
                            ),
                        )
                        .at(Rect::from_points(p.points.iter().copied()).center())
                        .with(a.c.id),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::Layer;
    use crate::pcb::{Track, Via};
    use crate::project::Project;
    use crate::units::mm;

    fn track(id: u64, ax: f64, ay: f64, bx: f64, by: f64, net: &str) -> Track {
        Track {
            id: Id(id),
            layer: Layer::TopCopper,
            a: Point::new(mm(ax), mm(ay)),
            b: Point::new(mm(bx), mm(by)),
            width: mm(0.25),
            net: net.into(),
        }
    }

    #[test]
    fn finds_clearance_short_and_edge_errors() {
        let p = Project::new("t");
        let mut pcb = p.boards[0].pcb.clone();
        pcb.tracks.push(track(100, 10.0, 10.0, 30.0, 10.0, "A"));
        pcb.tracks.push(track(101, 10.0, 10.3, 30.0, 10.3, "B"));
        pcb.tracks.push(track(102, 20.0, 5.0, 20.0, 15.0, "C"));
        pcb.tracks.push(track(103, 40.0, 30.0, 60.0, 30.0, "D"));
        pcb.vias.push(Via {
            id: Id(104),
            pos: Point::new(mm(5.0), mm(30.0)),
            diameter: mm(0.4),
            drill: mm(0.2),
            net: "E".into(),
        });
        let issues = run(&pcb, &p.library, Unit::Mm);
        let m: Vec<_> = issues.iter().map(|i| i.message.as_str()).collect();
        assert!(
            m.iter()
                .any(|s| s.starts_with("Clearance") && s.contains("(A)") && s.contains("(B)")),
            "{m:#?}"
        );
        assert!(
            m.iter()
                .any(|s| s.starts_with("Short") && s.contains("(C)")),
            "{m:#?}"
        );
        assert!(m.iter().any(|s| s.contains("outside the board")), "{m:#?}");
        assert!(m.iter().any(|s| s.starts_with("Via drill")), "{m:#?}");
        assert!(m.iter().any(|s| s.starts_with("Via annular")), "{m:#?}");
    }

    #[test]
    fn overlapping_parts_and_silk_are_reported() {
        let mut p = crate::examples::blinker();
        let lib = p.library.clone();
        let pcb = &mut p.boards[0].pcb;
        let u1 = pcb
            .components
            .iter()
            .find(|c| c.designator == "U1")
            .unwrap()
            .pos;
        pcb.components
            .iter_mut()
            .find(|c| c.designator == "C3")
            .unwrap()
            .pos = u1 + Point::new(mm(2.7), mm(0.6));
        let issues = run(pcb, &lib, Unit::Mm);
        let m: Vec<_> = issues.iter().map(|i| i.message.as_str()).collect();
        assert!(m.iter().any(|s| s.contains("overlaps")), "{m:#?}");
    }

    #[test]
    fn example_has_no_placement_warnings() {
        let p = crate::examples::blinker();
        let issues = run(&p.boards[0].pcb, &p.library, Unit::Mm);
        assert!(
            !issues
                .iter()
                .any(|i| i.message.contains("overlaps") || i.message.contains("Silkscreen")),
            "{issues:#?}"
        );
    }

    #[test]
    fn thin_silkscreen_is_flagged() {
        use crate::pcb::PcbGraphic;
        use crate::shape::Shape;
        let p = Project::new("t");
        let mut pcb = p.boards[0].pcb.clone();
        pcb.graphics.push(PcbGraphic {
            id: Id(500),
            layer: Layer::TopSilk,
            shape: Shape::Text {
                pos: Point::new(mm(10.0), mm(10.0)),
                text: "v1".into(),
                size: mm(0.4),
                rotation: 0.0,
                align: crate::shape::Align::Left,
            },
        });
        pcb.graphics.push(PcbGraphic {
            id: Id(501),
            layer: Layer::TopSilk,
            shape: Shape::line(
                Point::new(mm(5.0), mm(5.0)),
                Point::new(mm(9.0), mm(5.0)),
                mm(0.05),
            ),
        });
        let issues = run(&pcb, &p.library, Unit::Mm);
        assert_eq!(
            issues
                .iter()
                .filter(|i| i.message.starts_with("Silkscreen"))
                .count(),
            2,
            "{issues:#?}"
        );
    }

    #[test]
    fn clean_board_passes() {
        let p = crate::examples::regulator();
        assert!(run(&p.boards[0].pcb, &p.library, Unit::Mm).is_empty());
    }

    #[test]
    fn dangling_track_ends_are_flagged() {
        let p = crate::examples::regulator();
        let mut pcb = p.boards[0].pcb.clone();
        // Lone stub off the VIN track, plus a floating segment.
        pcb.tracks.push(track(900, 10.0, 12.27, 10.0, 15.0, "VIN"));
        pcb.tracks.push(track(901, 30.0, 20.0, 34.0, 20.0, ""));
        let issues = run(&pcb, &p.library, Unit::Mm);
        let dangling: Vec<_> = issues
            .iter()
            .filter(|i| i.message.starts_with("Track end"))
            .collect();
        assert_eq!(dangling.len(), 2, "{issues:#?}");
        assert_eq!(dangling[0].objects, vec![Id(900)]);
        assert_eq!(dangling[0].pos, Some(Point::new(mm(10.0), mm(15.0))));
        assert!(dangling.iter().all(|i| i.severity == Severity::Warning));
    }

    #[test]
    fn cleanup_removes_whole_dangling_chains() {
        let p = crate::examples::regulator();
        let mut pcb = p.boards[0].pcb.clone();
        let before = pcb.tracks.len();
        // A two-segment stub off VIN and a lone floating segment.
        pcb.tracks.push(track(900, 10.0, 12.27, 10.0, 15.0, "VIN"));
        pcb.tracks.push(track(901, 10.0, 15.0, 8.0, 17.0, "VIN"));
        pcb.tracks.push(track(902, 30.0, 20.0, 34.0, 20.0, ""));
        let mut gone = remove_dangling_tracks(&mut pcb, &p.library);
        gone.sort();
        assert_eq!(gone, vec![Id(900), Id(901), Id(902)]);
        assert_eq!(pcb.tracks.len(), before);
        assert!(run(&pcb, &p.library, Unit::Mm).is_empty());
    }
}
