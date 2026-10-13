//! Single-stroke vector font.
//!
//! Every glyph is a set of polylines on a 4 x 6 grid (cap height 6,
//! x-height 4, descenders to -2). Stroked text is what fabs expect on
//! silkscreen and it renders identically on screen, in SVG and in Gerber.

use crate::geom::{rotate, Point};
use crate::shape::Align;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Horizontal advance per character in grid units.
const ADVANCE: f64 = 5.6;
/// Distance between baselines of consecutive lines, as a multiple of size.
pub const LINE_SPACING: f64 = 1.6;

const GLYPHS: &[(char, &str)] = &[
    (' ', ""),
    ('!', "2,6 2,1.5; 2,0.3 2,0"),
    ('"', "1.3,6 1.3,4.5; 2.7,6 2.7,4.5"),
    ('#', "1,0 1.5,6; 2.5,0 3,6; 0,2 4,2; 0,4 4,4"),
    (
        '$',
        "4,4.5 3,5 1,5 0,4.5 0,3.5 1,3 3,3 4,2.5 4,1.5 3,1 1,1 0,1.5; 2,6 2,0",
    ),
    ('%', "0,6 0,5 1,5 1,6 0,6; 4,6 0,0; 3,1 3,0 4,0 4,1 3,1"),
    ('&', "4,0 1,4 1,5 2,6 3,5 3,4.5 0,2 0,1 1,0 2.5,0 4,2"),
    ('\'', "2,6 2,4.5"),
    ('(', "3,7 2,6 1.5,4.5 1.5,1.5 2,0 3,-1"),
    (')', "1,7 2,6 2.5,4.5 2.5,1.5 2,0 1,-1"),
    ('*', "2,5 2,1; 0.3,4 3.7,2; 0.3,2 3.7,4"),
    ('+', "2,5 2,1; 0,3 4,3"),
    (',', "2,0.5 2,0 1.5,-1"),
    ('-', "0.5,3 3.5,3"),
    ('.', "2,0.3 2,0"),
    ('/', "0,0 4,6"),
    ('0', "1,0 0,1 0,5 1,6 3,6 4,5 4,1 3,0 1,0"),
    ('1', "1,5 2,6 2,0; 1,0 3,0"),
    ('2', "0,5 1,6 3,6 4,5 4,4 0,0 4,0"),
    (
        '3',
        "0,5 1,6 3,6 4,5 4,4 3,3 4,2 4,1 3,0 1,0 0,1; 1.5,3 3,3",
    ),
    ('4', "3,0 3,6 0,2 4,2"),
    ('5', "4,6 0.3,6 0,3 1,3.6 3,3.6 4,2.6 4,1 3,0 1,0 0,1"),
    (
        '6',
        "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1 4,2.5 3,3.5 1,3.5 0,2.5",
    ),
    ('7', "0,6 4,6 1.5,0"),
    (
        '8',
        "1,3 0,4 0,5 1,6 3,6 4,5 4,4 3,3 1,3 0,2 0,1 1,0 3,0 4,1 4,2 3,3",
    ),
    (
        '9',
        "0,1 1,0 3,0 4,1 4,5 3,6 1,6 0,5 0,3.5 1,2.5 3,2.5 4,3.5",
    ),
    (':', "2,3.5 2,3.2; 2,0.3 2,0"),
    (';', "2,3.5 2,3.2; 2,0.5 2,0 1.5,-1"),
    ('<', "4,5 0,3 4,1"),
    ('=', "0,2 4,2; 0,4 4,4"),
    ('>', "0,5 4,3 0,1"),
    ('?', "0,5 1,6 3,6 4,5 4,4 2,2.5 2,1.5; 2,0.3 2,0"),
    (
        '@',
        "3,2 1.5,2 1,2.5 1,3.5 1.5,4 3,4 3,1.5 3.5,1 4,1.5 4,5 3,6 1,6 0,5 0,1 1,0 3.5,0",
    ),
    ('A', "0,0 2,6 4,0; 0.7,2 3.3,2"),
    ('B', "0,0 0,6 3,6 4,5 4,4 3,3 0,3; 3,3 4,2 4,1 3,0 0,0"),
    ('C', "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1"),
    ('D', "0,0 0,6 2.5,6 4,4.5 4,1.5 2.5,0 0,0"),
    ('E', "4,6 0,6 0,0 4,0; 0,3 3,3"),
    ('F', "4,6 0,6 0,0; 0,3 3,3"),
    ('G', "4,5 3,6 1,6 0,5 0,1 1,0 3,0 4,1 4,3 2,3"),
    ('H', "0,0 0,6; 4,0 4,6; 0,3 4,3"),
    ('I', "1,6 3,6; 2,6 2,0; 1,0 3,0"),
    ('J', "4,6 4,1 3,0 1,0 0,1"),
    ('K', "0,0 0,6; 4,6 0,2; 1.3,3.3 4,0"),
    ('L', "0,6 0,0 4,0"),
    ('M', "0,0 0,6 2,3 4,6 4,0"),
    ('N', "0,0 0,6 4,0 4,6"),
    ('O', "1,0 0,1 0,5 1,6 3,6 4,5 4,1 3,0 1,0"),
    ('P', "0,0 0,6 3,6 4,5 4,4 3,3 0,3"),
    ('Q', "1,0 0,1 0,5 1,6 3,6 4,5 4,1 3,0 1,0; 2.5,1.5 4.2,-0.2"),
    ('R', "0,0 0,6 3,6 4,5 4,4 3,3 0,3; 2,3 4,0"),
    ('S', "4,5 3,6 1,6 0,5 0,4 1,3 3,3 4,2 4,1 3,0 1,0 0,1"),
    ('T', "0,6 4,6; 2,6 2,0"),
    ('U', "0,6 0,1 1,0 3,0 4,1 4,6"),
    ('V', "0,6 2,0 4,6"),
    ('W', "0,6 1,0 2,4 3,0 4,6"),
    ('X', "0,0 4,6; 0,6 4,0"),
    ('Y', "0,6 2,3 4,6; 2,3 2,0"),
    ('Z', "0,6 4,6 0,0 4,0"),
    ('[', "3,7 1.5,7 1.5,-1 3,-1"),
    ('\\', "0,6 4,0"),
    (']', "1,7 2.5,7 2.5,-1 1,-1"),
    ('^', "1,5 2,6 3,5"),
    ('_', "0,-1 4,-1"),
    ('`', "1.5,6 2.5,5"),
    ('a', "0.5,4 3,4 4,3 4,0; 4,2.5 1,2.5 0,1.5 0,1 1,0 3,0 4,1"),
    ('b', "0,6 0,0; 0,3 1,4 3,4 4,3 4,1 3,0 1,0 0,1"),
    ('c', "4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1"),
    ('d', "4,6 4,0; 4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1"),
    ('e', "0,2 4,2 4,3 3,4 1,4 0,3 0,1 1,0 3.5,0"),
    ('f', "4,6 2.5,6 1.5,5 1.5,0; 0,4 3.5,4"),
    ('g', "4,4 4,-1 3,-2 1,-2; 4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1"),
    ('h', "0,6 0,0; 0,3 1,4 3,4 4,3 4,0"),
    ('i', "2,4 2,0; 2,5.6 2,5.9"),
    ('j', "3,4 3,-1 2,-2 1,-2; 3,5.6 3,5.9"),
    ('k', "0,6 0,0; 4,4 0,1.5; 1.5,2.3 4,0"),
    ('l', "1.5,6 1.5,1 2.5,0"),
    ('m', "0,4 0,0; 0,3 1,4 2,3 2,0; 2,3 3,4 4,3 4,0"),
    ('n', "0,4 0,0; 0,3 1,4 3,4 4,3 4,0"),
    ('o', "1,0 0,1 0,3 1,4 3,4 4,3 4,1 3,0 1,0"),
    ('p', "0,4 0,-2; 0,3 1,4 3,4 4,3 4,1 3,0 1,0 0,1"),
    ('q', "4,4 4,-2; 4,3 3,4 1,4 0,3 0,1 1,0 3,0 4,1"),
    ('r', "0,4 0,0; 0,2.5 1.5,4 4,4"),
    (
        's',
        "4,3.5 3.5,4 0.5,4 0,3.5 0,2.5 0.5,2 3.5,2 4,1.5 4,0.5 3.5,0 0.5,0 0,0.5",
    ),
    ('t', "1.5,6 1.5,1 2.5,0 4,0; 0,4 3.5,4"),
    ('u', "0,4 0,1 1,0 3,0 4,1; 4,4 4,0"),
    ('v', "0,4 2,0 4,4"),
    ('w', "0,4 1,0 2,3 3,0 4,4"),
    ('x', "0,0 4,4; 0,4 4,0"),
    ('y', "0,4 2,0; 4,4 1,-2"),
    ('z', "0,4 4,4 0,0 4,0"),
    ('{', "3,7 2,6.5 2,3.5 1,3 2,2.5 2,-0.5 3,-1"),
    ('|', "2,7 2,-1"),
    ('}', "1,7 2,6.5 2,3.5 3,3 2,2.5 2,-0.5 1,-1"),
    ('~', "0,3 1,3.7 3,2.3 4,3"),
    ('µ', "0,4 0,-2; 0,1 1,0 3,0 4,1; 4,4 4,0"),
    (
        'Ω',
        "0,0 1.5,0 1.5,1 0.3,2.5 0.3,4.5 1.3,6 2.7,6 3.7,4.5 3.7,2.5 2.5,1 2.5,0 4,0",
    ),
    ('°', "1.5,6 2.5,6 2.5,5 1.5,5 1.5,6"),
    ('±', "2,5 2,2; 0,3.5 4,3.5; 0,1 4,1"),
];

/// Drawn for characters the font does not cover.
const MISSING: &str = "0,0 4,0 4,6 0,6 0,0";

type Glyph = Vec<Vec<(f64, f64)>>;

fn parse_glyph(src: &str) -> Glyph {
    src.split(';')
        .map(|stroke| {
            stroke
                .split_whitespace()
                .filter_map(|pt| {
                    let (x, y) = pt.split_once(',')?;
                    Some((x.parse().ok()?, y.parse().ok()?))
                })
                .collect::<Vec<_>>()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn table() -> &'static HashMap<char, Glyph> {
    static TABLE: OnceLock<HashMap<char, Glyph>> = OnceLock::new();
    TABLE.get_or_init(|| GLYPHS.iter().map(|(c, s)| (*c, parse_glyph(s))).collect())
}

fn glyph(c: char) -> Glyph {
    table()
        .get(&c)
        .cloned()
        .unwrap_or_else(|| parse_glyph(MISSING))
}

/// Width of the widest line of `text` at cap height `size`.
pub fn text_width(text: &str, size: i64) -> i64 {
    let unit = size as f64 / 6.0;
    let chars = text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
    if chars == 0 {
        return 0;
    }
    ((chars as f64 * ADVANCE - (ADVANCE - 4.0)) * unit).round() as i64
}

/// Default stroke width for text of the given size.
pub fn stroke_width(size: i64) -> i64 {
    (size / 8).max(1)
}

/// Polylines (in nm) that draw `text`, anchored at `pos` on the first
/// line's baseline, rotated CCW by `rotation` degrees.
pub fn text_strokes(
    text: &str,
    size: i64,
    pos: Point,
    rotation: f64,
    align: Align,
) -> Vec<Vec<Point>> {
    let unit = size as f64 / 6.0;
    let mut out = Vec::new();
    for (row, line) in text.lines().enumerate() {
        let w = text_width(line, size) as f64;
        let x0 = match align {
            Align::Left => 0.0,
            Align::Center => -w / 2.0,
            Align::Right => -w,
        };
        let y0 = -(row as f64) * LINE_SPACING * size as f64;
        for (i, c) in line.chars().enumerate() {
            let gx = x0 + i as f64 * ADVANCE * unit;
            for stroke in glyph(c) {
                out.push(
                    stroke
                        .iter()
                        .map(|&(x, y)| {
                            rotate(Point::from_f(gx + x * unit, y0 + y * unit), rotation) + pos
                        })
                        .collect(),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_glyph_parses() {
        for (c, src) in GLYPHS {
            let g = parse_glyph(src);
            if *c != ' ' {
                assert!(!g.is_empty(), "glyph {c:?} is empty");
            }
            let pts: usize = g.iter().map(|s| s.len()).sum();
            let expected = src.split_whitespace().filter(|t| t.contains(',')).count();
            assert_eq!(pts, expected, "glyph {c:?} has a malformed point");
        }
    }

    #[test]
    fn width_and_alignment() {
        assert_eq!(text_width("", 600), 0);
        assert_eq!(text_width("A", 600), 400);
        let left = text_strokes("H", 600, Point::ZERO, 0.0, Align::Left);
        let right = text_strokes("H", 600, Point::ZERO, 0.0, Align::Right);
        let max_x = |s: &Vec<Vec<Point>>| s.iter().flatten().map(|p| p.x).max().unwrap();
        assert_eq!(max_x(&left), 400);
        assert_eq!(max_x(&right), 0);
    }

    #[test]
    fn unknown_chars_draw_a_box() {
        assert_eq!(
            text_strokes("\u{2603}", 600, Point::ZERO, 0.0, Align::Left).len(),
            1
        );
    }
}
