//! Copper pour filling and polygon offsetting.

use crate::geom::{arc_points, edges, polygon_area, Point};
use crate::library::Hull;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::overlay::FloatOverlay;
use serde::{Deserialize, Serialize};

/// A filled copper area: one outer boundary with optional holes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub outer: Vec<Point>,
    #[serde(default)]
    pub holes: Vec<Vec<Point>>,
}

impl Region {
    pub fn contains(&self, p: Point) -> bool {
        crate::geom::point_in_polygon(p, &self.outer)
            && !self
                .holes
                .iter()
                .any(|h| crate::geom::point_in_polygon(p, h) && !on_boundary(p, h))
    }

    pub fn area(&self) -> f64 {
        polygon_area(&self.outer).abs()
            - self
                .holes
                .iter()
                .map(|h| polygon_area(h).abs())
                .sum::<f64>()
    }
}

fn on_boundary(p: Point, poly: &[Point]) -> bool {
    edges(poly).any(|(a, b)| crate::geom::point_on_segment(p, a, b))
}

/// Make a polygon counter-clockwise.
pub fn ccw(mut poly: Vec<Point>) -> Vec<Point> {
    if polygon_area(&poly) < 0.0 {
        poly.reverse();
    }
    poly
}

/// Grow a convex polygon outward by `r`, rounding the corners.
pub fn offset_polygon(poly: &[Point], r: i64) -> Vec<Point> {
    let poly = ccw(poly.to_vec());
    let n = poly.len();
    if n < 3 || r <= 0 {
        return poly;
    }
    let normal_angle =
        |a: Point, b: Point| ((b.y - a.y) as f64).atan2((b.x - a.x) as f64).to_degrees() - 90.0;
    let mut out = Vec::new();
    for i in 0..n {
        let prev = poly[(i + n - 1) % n];
        let cur = poly[i];
        let next = poly[(i + 1) % n];
        let a0 = normal_angle(prev, cur);
        let mut a1 = normal_angle(cur, next);
        while a1 < a0 {
            a1 += 360.0;
        }
        if a1 - a0 > 180.0 {
            // Reflex corner: not expected for convex input, keep a sharp join.
            out.push(cur);
            continue;
        }
        out.extend(arc_points(cur, r, a0, a1 - a0, 15.0));
    }
    out
}

fn to_f(poly: &[Point]) -> Vec<[f64; 2]> {
    poly.iter().map(|p| [p.x as f64, p.y as f64]).collect()
}

fn from_f(poly: &[[f64; 2]]) -> Vec<Point> {
    poly.iter().map(|p| Point::from_f(p[0], p[1])).collect()
}

/// Fill `outline` with copper, keeping `clearance` away from every hull in
/// `obstacles`. Islands that touch none of `keep` are removed when
/// `remove_islands` is set (dead copper is a common fab complaint).
pub fn fill(
    outline: &[Point],
    obstacles: &[Hull],
    clearance: i64,
    keep: &[Hull],
    remove_islands: bool,
    min_area: f64,
) -> Vec<Region> {
    if outline.len() < 3 {
        return vec![];
    }
    let subj = vec![to_f(&ccw(outline.to_vec()))];
    let clip: Vec<Vec<[f64; 2]>> = obstacles
        .iter()
        .map(|h| to_f(&ccw(h.inflate(clearance).to_polygon())))
        .filter(|p| p.len() >= 3)
        .collect();
    let shapes = if clip.is_empty() {
        vec![subj.clone()]
    } else {
        FloatOverlay::with_subj_and_clip(&subj, &clip)
            .overlay(OverlayRule::Difference, FillRule::NonZero)
    };
    shapes
        .into_iter()
        .filter_map(|shape| {
            let mut it = shape.into_iter();
            let outer = from_f(&it.next()?);
            let region = Region {
                outer,
                holes: it.map(|h| from_f(&h)).collect(),
            };
            (region.area() >= min_area).then_some(region)
        })
        .filter(|r| !remove_islands || keep.iter().any(|k| touches(r, k)))
        .collect()
}

fn touches(region: &Region, hull: &Hull) -> bool {
    hull.points
        .iter()
        .chain(hull.to_polygon().iter())
        .any(|p| region.contains(*p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Rect;

    fn square(c: Point, h: i64) -> Vec<Point> {
        Rect::around(c, h, h).corners().to_vec()
    }

    #[test]
    fn fill_without_obstacles_is_outline() {
        let r = fill(&square(Point::ZERO, 100), &[], 10, &[], false, 0.0);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].area(), 40_000.0);
    }

    #[test]
    fn obstacle_makes_hole() {
        let obstacle = Hull::circle(Point::ZERO, 10);
        let r = fill(&square(Point::ZERO, 1000), &[obstacle], 10, &[], false, 0.0);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].holes.len(), 1);
        assert!(!r[0].contains(Point::new(15, 0)));
        assert!(r[0].contains(Point::new(500, 0)));
    }

    #[test]
    fn splitting_and_island_removal() {
        // A wall splits the area in two; only the half touching `keep` stays.
        let wall = Hull::capsule(Point::new(0, -2000), Point::new(0, 2000), 10);
        let keep = Hull::circle(Point::new(500, 0), 5);
        let all = fill(
            &square(Point::ZERO, 1000),
            std::slice::from_ref(&wall),
            10,
            &[],
            false,
            0.0,
        );
        assert_eq!(all.len(), 2);
        let kept = fill(&square(Point::ZERO, 1000), &[wall], 10, &[keep], true, 0.0);
        assert_eq!(kept.len(), 1);
        assert!(kept[0].contains(Point::new(500, 0)));
    }

    #[test]
    fn offsetting_a_square_rounds_corners() {
        let p = offset_polygon(&square(Point::ZERO, 100), 10);
        assert!(p.len() > 8);
        assert!(crate::geom::point_in_polygon(Point::new(109, 0), &p));
        assert!(!crate::geom::point_in_polygon(Point::new(109, 109), &p));
    }
}
