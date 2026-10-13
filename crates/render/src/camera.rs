//! 2D view transform. World units are millimetres (Y up), screen units are
//! CSS pixels (Y down).

use easypro_core::geom::{Point, Rect};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// World point at the centre of the viewport, in mm.
    pub center: (f64, f64),
    /// Pixels per millimetre.
    pub zoom: f64,
    /// Viewport size in CSS pixels.
    pub size: (f64, f64),
    /// Mirror left-right: looking at a board from below.
    pub flip: bool,
}

pub const MIN_ZOOM: f64 = 0.05;
pub const MAX_ZOOM: f64 = 2000.0;

impl Default for Camera {
    fn default() -> Self {
        Camera {
            center: (0.0, 0.0),
            zoom: 8.0,
            size: (800.0, 600.0),
            flip: false,
        }
    }
}

impl Camera {
    /// -1 when the view is mirrored.
    pub fn sx(&self) -> f64 {
        if self.flip {
            -1.0
        } else {
            1.0
        }
    }

    pub fn to_screen(&self, wx: f64, wy: f64) -> (f64, f64) {
        (
            self.sx() * (wx - self.center.0) * self.zoom + self.size.0 / 2.0,
            self.size.1 / 2.0 - (wy - self.center.1) * self.zoom,
        )
    }

    pub fn to_world(&self, sx: f64, sy: f64) -> (f64, f64) {
        (
            self.sx() * (sx - self.size.0 / 2.0) / self.zoom + self.center.0,
            (self.size.1 / 2.0 - sy) / self.zoom + self.center.1,
        )
    }

    pub fn to_world_nm(&self, sx: f64, sy: f64) -> Point {
        let (x, y) = self.to_world(sx, sy);
        Point::from_f(x * 1e6, y * 1e6)
    }

    pub fn pan_pixels(&mut self, dx: f64, dy: f64) {
        self.center.0 -= self.sx() * dx / self.zoom;
        self.center.1 += dy / self.zoom;
    }

    /// Zoom by `factor`, keeping the world point under the cursor fixed.
    pub fn zoom_at(&mut self, sx: f64, sy: f64, factor: f64) {
        let before = self.to_world(sx, sy);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.to_world(sx, sy);
        self.center.0 += before.0 - after.0;
        self.center.1 += before.1 - after.1;
    }

    /// Fit a world rectangle (nm) with a margin in pixels.
    pub fn fit(&mut self, r: Rect, margin_px: f64) {
        if r.is_empty() {
            return;
        }
        let w = (r.width() as f64 / 1e6).max(1.0);
        let h = (r.height() as f64 / 1e6).max(1.0);
        let zx = (self.size.0 - 2.0 * margin_px).max(10.0) / w;
        let zy = (self.size.1 - 2.0 * margin_px).max(10.0) / h;
        self.zoom = zx.min(zy).clamp(MIN_ZOOM, MAX_ZOOM);
        let c = r.center();
        self.center = (c.x as f64 / 1e6, c.y as f64 / 1e6);
    }

    /// Visible world rectangle in nm.
    pub fn visible(&self) -> Rect {
        let a = self.to_world_nm(0.0, self.size.1);
        let b = self.to_world_nm(self.size.0, 0.0);
        Rect::new(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_world_round_trip() {
        let c = Camera {
            center: (10.0, -5.0),
            zoom: 4.0,
            size: (400.0, 300.0),
            flip: false,
        };
        let (sx, sy) = c.to_screen(12.5, 3.0);
        let (wx, wy) = c.to_world(sx, sy);
        assert!((wx - 12.5).abs() < 1e-9 && (wy - 3.0).abs() < 1e-9);
        assert_eq!(c.to_screen(10.0, -5.0), (200.0, 150.0));
    }

    #[test]
    fn flipped_view_round_trips_and_mirrors() {
        let mut c = Camera {
            center: (10.0, 0.0),
            zoom: 2.0,
            size: (200.0, 100.0),
            flip: true,
        };
        let (sx, _) = c.to_screen(15.0, 0.0);
        assert!(sx < 100.0, "points right of centre appear on the left");
        let (wx, _) = c.to_world(sx, 50.0);
        assert!((wx - 15.0).abs() < 1e-9);
        c.pan_pixels(10.0, 0.0);
        assert!(
            c.center.0 > 10.0,
            "dragging right moves the view the mirrored way"
        );
    }

    #[test]
    fn zoom_keeps_cursor_point() {
        let mut c = Camera::default();
        let before = c.to_world(100.0, 50.0);
        c.zoom_at(100.0, 50.0, 2.5);
        let after = c.to_world(100.0, 50.0);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
    }

    #[test]
    fn fit_contains_rect() {
        let mut c = Camera {
            size: (1000.0, 500.0),
            ..Default::default()
        };
        let r = Rect::new(Point::new(0, 0), Point::new(100_000_000, 40_000_000));
        c.fit(r, 20.0);
        assert!(c.visible().contains_rect(&r));
    }
}
