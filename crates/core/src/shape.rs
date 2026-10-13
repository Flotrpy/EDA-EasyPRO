//! Drawing primitives shared by symbols, footprints and documents.

use crate::geom::{arc_points, rotate, Point, Rect, Transform};
use serde::{Deserialize, Serialize};

/// Horizontal text anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Shape {
    Line { a: Point, b: Point, width: i64 },
    Polyline { points: Vec<Point>, width: i64 },
    Rect { a: Point, b: Point, width: i64, fill: bool },
    Circle { center: Point, radius: i64, width: i64, fill: bool },
    /// Arc around `center` from `start` degrees, sweeping `sweep` degrees CCW.
    Arc { center: Point, radius: i64, start: f64, sweep: f64, width: i64 },
    Polygon { points: Vec<Point>, width: i64, fill: bool },
    Text { pos: Point, text: String, size: i64, rotation: f64, align: Align },
}

impl Shape {
    pub fn line(a: Point, b: Point, width: i64) -> Shape {
        Shape::Line { a, b, width }
    }

    pub fn rect(a: Point, b: Point, width: i64) -> Shape {
        Shape::Rect { a, b, width, fill: false }
    }

    pub fn text(pos: Point, text: impl Into<String>, size: i64) -> Shape {
        Shape::Text { pos, text: text.into(), size, rotation: 0.0, align: Align::Left }
    }

    /// Return a copy moved by `tf`. Rectangles become polygons when the
    /// rotation is not a multiple of 90 degrees.
    pub fn transformed(&self, tf: &Transform) -> Shape {
        let ap = |p: &Point| tf.apply(*p);
        match self {
            Shape::Line { a, b, width } => Shape::Line { a: ap(a), b: ap(b), width: *width },
            Shape::Polyline { points, width } => {
                Shape::Polyline { points: points.iter().map(ap).collect(), width: *width }
            }
            Shape::Rect { a, b, width, fill } => {
                if (tf.rotation % 90.0).abs() < 1e-9 {
                    Shape::Rect { a: ap(a), b: ap(b), width: *width, fill: *fill }
                } else {
                    let pts = Rect::new(*a, *b).corners().iter().map(ap).collect();
                    Shape::Polygon { points: pts, width: *width, fill: *fill }
                }
            }
            Shape::Circle { center, radius, width, fill } => {
                Shape::Circle { center: ap(center), radius: *radius, width: *width, fill: *fill }
            }
            Shape::Arc { center, radius, start, sweep, width } => {
                let (start, sweep) = if tf.mirror {
                    (tf.apply_angle(*start), -*sweep)
                } else {
                    (tf.apply_angle(*start), *sweep)
                };
                Shape::Arc { center: ap(center), radius: *radius, start, sweep, width: *width }
            }
            Shape::Polygon { points, width, fill } => {
                Shape::Polygon { points: points.iter().map(ap).collect(), width: *width, fill: *fill }
            }
            Shape::Text { pos, text, size, rotation, align } => {
                // Text stays readable: mirroring flips alignment, not glyphs.
                let mut rot = crate::geom::normalize_angle(rotation + tf.rotation);
                let mut align = *align;
                if tf.mirror {
                    align = match align {
                        Align::Left => Align::Right,
                        Align::Right => Align::Left,
                        Align::Center => Align::Center,
                    };
                    if rot == 90.0 || rot == 270.0 {
                        rot = crate::geom::normalize_angle(rot + 180.0);
                        align = match align {
                            Align::Left => Align::Right,
                            Align::Right => Align::Left,
                            Align::Center => Align::Center,
                        };
                    }
                }
                Shape::Text { pos: ap(pos), text: text.clone(), size: *size, rotation: rot, align }
            }
        }
    }

    /// Bounding box including stroke width.
    pub fn bbox(&self) -> Rect {
        match self {
            Shape::Line { a, b, width } => Rect::new(*a, *b).inflate(width / 2),
            Shape::Polyline { points, width } | Shape::Polygon { points, width, .. } => {
                Rect::from_points(points.iter().copied()).inflate(width / 2)
            }
            Shape::Rect { a, b, width, .. } => Rect::new(*a, *b).inflate(width / 2),
            Shape::Circle { center, radius, width, .. } => Rect::around(*center, radius + width / 2, radius + width / 2),
            Shape::Arc { center, radius, start, sweep, width } => {
                Rect::from_points(arc_points(*center, *radius, *start, *sweep, 10.0)).inflate(width / 2)
            }
            Shape::Text { pos, text, size, rotation, align } => {
                let w = crate::font::text_width(text, *size);
                let x0 = match align {
                    Align::Left => 0,
                    Align::Center => -w / 2,
                    Align::Right => -w,
                };
                let r = Rect::new(Point::new(x0, -size / 4), Point::new(x0 + w, *size));
                Rect::from_points(r.corners().iter().map(|c| rotate(*c, *rotation) + *pos))
            }
        }
    }

    /// Distance from `p` to the visible stroke or fill of this shape.
    pub fn distance_to(&self, p: Point) -> f64 {
        use crate::geom::*;
        match self {
            Shape::Line { a, b, width } => (point_segment_dist(p, *a, *b) - *width as f64 / 2.0).max(0.0),
            Shape::Polyline { points, width } => {
                let d = points.windows(2).map(|w| point_segment_dist(p, w[0], w[1])).fold(f64::INFINITY, f64::min);
                (d - *width as f64 / 2.0).max(0.0)
            }
            Shape::Rect { a, b, width, fill } => {
                let poly = Rect::new(*a, *b).corners().to_vec();
                if *fill {
                    point_polygon_dist(p, &poly)
                } else {
                    let d = edges(&poly).map(|(a, b)| point_segment_dist(p, a, b)).fold(f64::INFINITY, f64::min);
                    (d - *width as f64 / 2.0).max(0.0)
                }
            }
            Shape::Polygon { points, width, fill } => {
                if *fill {
                    point_polygon_dist(p, points)
                } else {
                    let d = edges(points).map(|(a, b)| point_segment_dist(p, a, b)).fold(f64::INFINITY, f64::min);
                    (d - *width as f64 / 2.0).max(0.0)
                }
            }
            Shape::Circle { center, radius, width, fill } => {
                let d = p.dist(*center);
                if *fill {
                    (d - *radius as f64).max(0.0)
                } else {
                    ((d - *radius as f64).abs() - *width as f64 / 2.0).max(0.0)
                }
            }
            Shape::Arc { center, radius, start, sweep, width } => {
                let pts = arc_points(*center, *radius, *start, *sweep, 5.0);
                let d = pts.windows(2).map(|w| point_segment_dist(p, w[0], w[1])).fold(f64::INFINITY, f64::min);
                (d - *width as f64 / 2.0).max(0.0)
            }
            Shape::Text { .. } => {
                let b = self.bbox();
                if b.contains(p) { 0.0 } else { point_polygon_dist(p, &b.corners()) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_rotated_off_axis_becomes_polygon() {
        let r = Shape::rect(Point::new(0, 0), Point::new(10, 10), 1);
        let t = Transform::new(Point::ZERO, 45.0, false);
        assert!(matches!(r.transformed(&t), Shape::Polygon { .. }));
        let t = Transform::new(Point::ZERO, 90.0, false);
        assert!(matches!(r.transformed(&t), Shape::Rect { .. }));
    }

    #[test]
    fn hit_distance() {
        let l = Shape::line(Point::new(0, 0), Point::new(100, 0), 10);
        assert_eq!(l.distance_to(Point::new(50, 3)), 0.0);
        assert_eq!(l.distance_to(Point::new(50, 25)), 20.0);
        let c = Shape::Circle { center: Point::ZERO, radius: 50, width: 0, fill: true };
        assert_eq!(c.distance_to(Point::new(10, 0)), 0.0);
    }
}
