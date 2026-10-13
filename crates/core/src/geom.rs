//! Integer 2D geometry: points, boxes, transforms and distance queries.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

impl Point {
    pub const ZERO: Point = Point { x: 0, y: 0 };

    pub const fn new(x: i64, y: i64) -> Self {
        Point { x, y }
    }

    pub fn dist(self, o: Point) -> f64 {
        let dx = (self.x - o.x) as f64;
        let dy = (self.y - o.y) as f64;
        dx.hypot(dy)
    }

    /// Snap to the nearest multiple of `grid` on both axes.
    pub fn snap(self, grid: i64) -> Point {
        if grid <= 0 {
            return self;
        }
        let s = |v: i64| (v as f64 / grid as f64).round() as i64 * grid;
        Point::new(s(self.x), s(self.y))
    }

    pub fn to_f(self) -> (f64, f64) {
        (self.x as f64, self.y as f64)
    }

    pub fn from_f(x: f64, y: f64) -> Point {
        Point::new(x.round() as i64, y.round() as i64)
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

impl Neg for Point {
    type Output = Point;
    fn neg(self) -> Point {
        Point::new(-self.x, -self.y)
    }
}

/// Axis-aligned bounding box. Empty boxes have `min > max`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Default for Rect {
    fn default() -> Self {
        Rect::EMPTY
    }
}

impl Rect {
    pub const EMPTY: Rect = Rect {
        min: Point { x: i64::MAX, y: i64::MAX },
        max: Point { x: i64::MIN, y: i64::MIN },
    };

    pub fn new(a: Point, b: Point) -> Rect {
        Rect {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn around(c: Point, half_w: i64, half_h: i64) -> Rect {
        Rect::new(
            Point::new(c.x - half_w, c.y - half_h),
            Point::new(c.x + half_w, c.y + half_h),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y
    }

    pub fn width(&self) -> i64 {
        if self.is_empty() { 0 } else { self.max.x - self.min.x }
    }

    pub fn height(&self) -> i64 {
        if self.is_empty() { 0 } else { self.max.y - self.min.y }
    }

    pub fn center(&self) -> Point {
        Point::new((self.min.x + self.max.x) / 2, (self.min.y + self.max.y) / 2)
    }

    pub fn include(&mut self, p: Point) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    pub fn union(mut self, o: Rect) -> Rect {
        if !o.is_empty() {
            self.include(o.min);
            self.include(o.max);
        }
        self
    }

    pub fn inflate(self, d: i64) -> Rect {
        if self.is_empty() {
            return self;
        }
        Rect {
            min: Point::new(self.min.x - d, self.min.y - d),
            max: Point::new(self.max.x + d, self.max.y + d),
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn contains_rect(&self, o: &Rect) -> bool {
        !o.is_empty() && self.contains(o.min) && self.contains(o.max)
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        !(o.min.x > self.max.x || o.max.x < self.min.x || o.min.y > self.max.y || o.max.y < self.min.y)
    }

    pub fn from_points(pts: impl IntoIterator<Item = Point>) -> Rect {
        let mut r = Rect::EMPTY;
        for p in pts {
            r.include(p);
        }
        r
    }

    pub fn corners(&self) -> [Point; 4] {
        [
            self.min,
            Point::new(self.max.x, self.min.y),
            self.max,
            Point::new(self.min.x, self.max.y),
        ]
    }
}

/// Placement of an object: rotate (counter-clockwise, degrees), then mirror
/// about the Y axis if `mirror`, then translate by `offset`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub offset: Point,
    pub rotation: f64,
    pub mirror: bool,
}

impl Transform {
    pub fn new(offset: Point, rotation: f64, mirror: bool) -> Self {
        Transform { offset, rotation: normalize_angle(rotation), mirror }
    }

    pub fn translate(offset: Point) -> Self {
        Transform { offset, ..Default::default() }
    }

    pub fn apply(&self, p: Point) -> Point {
        let mut q = rotate(p, self.rotation);
        if self.mirror {
            q.x = -q.x;
        }
        q + self.offset
    }

    /// Inverse of [`Transform::apply`].
    pub fn invert(&self, p: Point) -> Point {
        let mut q = p - self.offset;
        if self.mirror {
            q.x = -q.x;
        }
        rotate(q, -self.rotation)
    }

    /// Effective rotation of a direction after this transform.
    pub fn apply_angle(&self, deg: f64) -> f64 {
        let a = deg + self.rotation;
        normalize_angle(if self.mirror { 180.0 - a } else { a })
    }
}

/// Map an angle to `[0, 360)`.
pub fn normalize_angle(a: f64) -> f64 {
    let r = a.rem_euclid(360.0);
    if (r - 360.0).abs() < 1e-9 { 0.0 } else { r }
}

/// Rotate a point about the origin. Multiples of 90° are exact.
pub fn rotate(p: Point, deg: f64) -> Point {
    let a = normalize_angle(deg);
    if a == 0.0 {
        p
    } else if a == 90.0 {
        Point::new(-p.y, p.x)
    } else if a == 180.0 {
        Point::new(-p.x, -p.y)
    } else if a == 270.0 {
        Point::new(p.y, -p.x)
    } else {
        let (s, c) = a.to_radians().sin_cos();
        let (x, y) = p.to_f();
        Point::from_f(x * c - y * s, x * s + y * c)
    }
}

/// Distance from `p` to the segment `a`-`b`.
pub fn point_segment_dist(p: Point, a: Point, b: Point) -> f64 {
    let (px, py) = p.to_f();
    let (ax, ay) = a.to_f();
    let (bx, by) = b.to_f();
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return (px - ax).hypot(py - ay);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
    (px - (ax + t * dx)).hypot(py - (ay + t * dy))
}

/// Closest point on segment `a`-`b` to `p`.
pub fn project_on_segment(p: Point, a: Point, b: Point) -> Point {
    let (px, py) = p.to_f();
    let (ax, ay) = a.to_f();
    let (bx, by) = b.to_f();
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return a;
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0);
    Point::from_f(ax + t * dx, ay + t * dy)
}

fn orient(a: Point, b: Point, c: Point) -> i128 {
    (b.x - a.x) as i128 * (c.y - a.y) as i128 - (b.y - a.y) as i128 * (c.x - a.x) as i128
}

fn on_segment(a: Point, b: Point, p: Point) -> bool {
    p.x >= a.x.min(b.x) && p.x <= a.x.max(b.x) && p.y >= a.y.min(b.y) && p.y <= a.y.max(b.y)
}

/// Exact test whether segments `a`-`b` and `c`-`d` touch or cross.
pub fn segments_intersect(a: Point, b: Point, c: Point, d: Point) -> bool {
    let d1 = orient(c, d, a);
    let d2 = orient(c, d, b);
    let d3 = orient(a, b, c);
    let d4 = orient(a, b, d);
    if ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0)) {
        return true;
    }
    (d1 == 0 && on_segment(c, d, a))
        || (d2 == 0 && on_segment(c, d, b))
        || (d3 == 0 && on_segment(a, b, c))
        || (d4 == 0 && on_segment(a, b, d))
}

/// Minimum distance between two segments (0 when they intersect).
pub fn segment_segment_dist(a: Point, b: Point, c: Point, d: Point) -> f64 {
    if segments_intersect(a, b, c, d) {
        return 0.0;
    }
    point_segment_dist(a, c, d)
        .min(point_segment_dist(b, c, d))
        .min(point_segment_dist(c, a, b))
        .min(point_segment_dist(d, a, b))
}

/// Exact test whether `p` lies on segment `a`-`b`.
pub fn point_on_segment(p: Point, a: Point, b: Point) -> bool {
    orient(a, b, p) == 0 && on_segment(a, b, p)
}

/// Even-odd point in polygon test. Points on the boundary count as inside.
pub fn point_in_polygon(p: Point, poly: &[Point]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if point_on_segment(p, a, b) {
            return true;
        }
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x as f64 + (p.y - a.y) as f64 * (b.x - a.x) as f64 / (b.y - a.y) as f64;
            if (p.x as f64) < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Distance from a point to a polygon's area (0 if inside).
pub fn point_polygon_dist(p: Point, poly: &[Point]) -> f64 {
    if point_in_polygon(p, poly) {
        return 0.0;
    }
    edges(poly).map(|(a, b)| point_segment_dist(p, a, b)).fold(f64::INFINITY, f64::min)
}

/// Distance from a segment to a polygon's area (0 if touching or inside).
pub fn segment_polygon_dist(a: Point, b: Point, poly: &[Point]) -> f64 {
    if point_in_polygon(a, poly) || point_in_polygon(b, poly) {
        return 0.0;
    }
    edges(poly).map(|(c, d)| segment_segment_dist(a, b, c, d)).fold(f64::INFINITY, f64::min)
}

/// Distance between the areas of two polygons.
pub fn polygon_polygon_dist(p: &[Point], q: &[Point]) -> f64 {
    if p.first().is_some_and(|&v| point_in_polygon(v, q)) || q.first().is_some_and(|&v| point_in_polygon(v, p)) {
        return 0.0;
    }
    edges(p).map(|(a, b)| segment_polygon_dist(a, b, q)).fold(f64::INFINITY, f64::min)
}

/// Iterate closed polygon edges.
pub fn edges(poly: &[Point]) -> impl Iterator<Item = (Point, Point)> + '_ {
    let n = poly.len();
    (0..n).map(move |i| (poly[i], poly[(i + 1) % n]))
}

/// Signed area (positive when counter-clockwise), in nm².
pub fn polygon_area(poly: &[Point]) -> f64 {
    edges(poly).map(|(a, b)| a.x as f64 * b.y as f64 - b.x as f64 * a.y as f64).sum::<f64>() / 2.0
}

/// Approximate a circle as a polygon with `n` vertices.
pub fn circle_polygon(c: Point, r: i64, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64 * std::f64::consts::TAU;
            Point::from_f(c.x as f64 + r as f64 * t.cos(), c.y as f64 + r as f64 * t.sin())
        })
        .collect()
}

/// Points along a circular arc from `start_deg` sweeping `sweep_deg`.
pub fn arc_points(c: Point, r: i64, start_deg: f64, sweep_deg: f64, max_seg_deg: f64) -> Vec<Point> {
    let n = ((sweep_deg.abs() / max_seg_deg).ceil() as usize).max(1);
    (0..=n)
        .map(|i| {
            let t = (start_deg + sweep_deg * i as f64 / n as f64).to_radians();
            Point::from_f(c.x as f64 + r as f64 * t.cos(), c.y as f64 + r as f64 * t.sin())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_is_exact_for_right_angles() {
        let p = Point::new(3, 1);
        assert_eq!(rotate(p, 90.0), Point::new(-1, 3));
        assert_eq!(rotate(p, -90.0), Point::new(1, -3));
        assert_eq!(rotate(p, 450.0), Point::new(-1, 3));
    }

    #[test]
    fn transform_inverts() {
        let t = Transform::new(Point::new(100, -50), 90.0, true);
        let p = Point::new(7, 11);
        assert_eq!(t.invert(t.apply(p)), p);
        let t = Transform::new(Point::new(5, 5), 33.0, false);
        let q = t.invert(t.apply(Point::new(1000, 2000)));
        assert!(q.dist(Point::new(1000, 2000)) <= 2.0);
    }

    #[test]
    fn segment_distances() {
        let d = segment_segment_dist(Point::new(0, 0), Point::new(10, 0), Point::new(0, 5), Point::new(10, 5));
        assert_eq!(d, 5.0);
        assert!(segments_intersect(Point::new(0, 0), Point::new(10, 10), Point::new(0, 10), Point::new(10, 0)));
        assert!(point_on_segment(Point::new(5, 0), Point::new(0, 0), Point::new(10, 0)));
        assert!(!point_on_segment(Point::new(5, 1), Point::new(0, 0), Point::new(10, 0)));
    }

    #[test]
    fn polygon_queries() {
        let sq = Rect::new(Point::new(0, 0), Point::new(10, 10)).corners().to_vec();
        assert!(point_in_polygon(Point::new(5, 5), &sq));
        assert!(point_in_polygon(Point::new(0, 5), &sq));
        assert!(!point_in_polygon(Point::new(11, 5), &sq));
        assert_eq!(point_polygon_dist(Point::new(13, 5), &sq), 3.0);
        assert_eq!(polygon_area(&sq), 100.0);
        let other = Rect::new(Point::new(20, 0), Point::new(30, 10)).corners().to_vec();
        assert_eq!(polygon_polygon_dist(&sq, &other), 10.0);
    }

    #[test]
    fn rect_ops() {
        let r = Rect::from_points([Point::new(1, 2), Point::new(-3, 7)]);
        assert_eq!(r.width(), 4);
        assert_eq!(r.height(), 5);
        assert!(Rect::EMPTY.is_empty());
        assert_eq!(Rect::EMPTY.union(r), r);
        assert_eq!(Point::new(149, -151).snap(100), Point::new(100, -200));
    }
}
