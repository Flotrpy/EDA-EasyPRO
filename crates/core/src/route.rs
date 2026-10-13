//! Interactive routing helpers: 45° path shaping and obstacle checks.

use crate::geom::Point;
use crate::layer::Layer;
use crate::library::Hull;
use crate::pcb::CopperItem;

/// A two-segment path from `a` to `b` made of one straight and one
/// diagonal leg (or a single leg when aligned). `diagonal_first` picks
/// which leg comes first; the router flips it with the `/` key.
pub fn path45(a: Point, b: Point, diagonal_first: bool) -> Vec<Point> {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    if dx == 0 || dy == 0 || dx.abs() == dy.abs() {
        return vec![a, b];
    }
    let d = dx.abs().min(dy.abs());
    let diag = Point::new(d * dx.signum(), d * dy.signum());
    let mid = if diagonal_first { a + diag } else { b - diag };
    vec![a, mid, b]
}

/// Schematic wires bend at right angles.
pub fn path90(a: Point, b: Point, horizontal_first: bool) -> Vec<Point> {
    if a.x == b.x || a.y == b.y {
        return vec![a, b];
    }
    let mid = if horizontal_first {
        Point::new(b.x, a.y)
    } else {
        Point::new(a.x, b.y)
    };
    vec![a, mid, b]
}

/// First obstacle of another net closer than `clearance` to the path.
pub fn first_collision<'a>(
    path: &[Point],
    width: i64,
    layer: Layer,
    net: &str,
    clearance: i64,
    items: &'a [CopperItem],
) -> Option<&'a CopperItem> {
    let segs: Vec<Hull> = path
        .windows(2)
        .map(|w| Hull::capsule(w[0], w[1], width / 2))
        .collect();
    items.iter().find(|it| {
        it.layer.shares_copper(layer)
            && (net.is_empty() || it.net != net)
            && segs.iter().any(|s| s.distance(&it.hull) < clearance as f64)
    })
}

/// Pick the clean variant of a 45° path if one exists ("walkaround light"):
/// try the preferred bend, then the other one.
pub fn best_path45(
    a: Point,
    b: Point,
    diagonal_first: bool,
    width: i64,
    layer: Layer,
    net: &str,
    clearance: i64,
    items: &[CopperItem],
) -> (Vec<Point>, bool) {
    let first = path45(a, b, diagonal_first);
    if first_collision(&first, width, layer, net, clearance, items).is_none() {
        return (first, true);
    }
    let second = path45(a, b, !diagonal_first);
    if first_collision(&second, width, layer, net, clearance, items).is_none() {
        return (second, true);
    }
    (first, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Id;
    use crate::pcb::CopperRef;

    #[test]
    fn shapes() {
        let p = path45(Point::new(0, 0), Point::new(10, 4), false);
        assert_eq!(
            p,
            vec![Point::new(0, 0), Point::new(6, 0), Point::new(10, 4)]
        );
        let p = path45(Point::new(0, 0), Point::new(10, 4), true);
        assert_eq!(
            p,
            vec![Point::new(0, 0), Point::new(4, 4), Point::new(10, 4)]
        );
        assert_eq!(path45(Point::new(0, 0), Point::new(5, 5), true).len(), 2);
        assert_eq!(
            path90(Point::new(0, 0), Point::new(3, 4), true)[1],
            Point::new(3, 0)
        );
    }

    #[test]
    fn walkaround_picks_free_bend() {
        let blocker = CopperItem {
            what: CopperRef::Via(Id(1)),
            net: "X".into(),
            layer: Layer::Multi,
            hull: Hull::circle(Point::new(60, 0), 5),
        };
        let (p, ok) = best_path45(
            Point::new(0, 0),
            Point::new(100, 40),
            false,
            2,
            Layer::TopCopper,
            "A",
            3,
            &[blocker],
        );
        assert!(ok);
        assert_eq!(p[1], Point::new(40, 40));
    }
}
