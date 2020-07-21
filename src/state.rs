// GPL v3.0

use super::{BezierCurve, Brush, Color};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{self, BresenhamLineIter, Canvas};
use parking_lot::RwLock;
use pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{collections::HashMap, mem};

const EXPECTED_CURVES: usize = 100;

/// The current graphical state.
#[derive(Serialize, Deserialize)]
pub struct GraphicalState {
    brushes: HashMap<usize, Brush>,
    curves: Vec<(BezierCurve, usize)>, // usize is brush index
}

// function to draw a thicker line segment onto a canvas
#[inline]
fn rasterize_line(
    c: &RwLock<RgbaImage>,
    width: i32,
    height: i32,
    line: LineSegment2F,
    color: Rgba<u8>,
    line_width: i32,
) {
    let line_iter =
        BresenhamLineIter::new((line.from_x(), line.from_y()), (line.to_x(), line.to_y()));
    line_iter
        .filter(|(x, y)| *x >= 0 && *x < width && *y >= 0 && *y < height)
        .for_each(|pt| {
            drawing::draw_filled_ellipse_mut(
                &mut *c.write(),
                (width, height),
                line_width,
                line_width,
                color,
            );
        });
}

macro_rules! f2u8 {
    ($val: expr) => {{
        ($val * (std::u8::MAX as f32)) as u8
    }};
}

impl GraphicalState {
    pub fn new() -> Self {
        Self {
            brushes: HashMap::new(),
            curves: Vec::new(),
        }
    }

    /// Rasterize this graphical state onto an image.
    pub fn rasterize(&self, target: &RwLock<RgbaImage>) {
        struct BCIntervalIter {
            prev: f32,
            t_interval: f32,
            internal: std::ops::Range<i32>,
        }

        impl Iterator for BCIntervalIter {
            type Item = (f32, f32);

            #[inline]
            fn next(&mut self) -> Option<(f32, f32)> {
                let i = match self.internal.next() {
                    Some(i) => i,
                    None => return None,
                };

                let mut t1 = (i as f32 + 1.0) * self.t_interval;
                let t2 = t1;
                mem::swap(&mut self.prev, &mut t1);

                Some((t1, t2))
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.internal.size_hint()
            }
        }

        impl ExactSizeIterator for BCIntervalIter {}

        let (width, height) = target.read().dimensions();

        // draw some curves
        self.curves.par_iter().for_each(|(curve, ci)| {
            // get the brush we are using
            let brush = self.brushes.get(ci).expect("Brush ID Mismatch");
            let color = brush.color();
            let color = Rgba([
                f2u8!(color.r()),
                f2u8!(color.g()),
                f2u8!(color.b()),
                std::u8::MAX,
            ]);

            let [start, control_a, control_b, end] = curve.clone().into_points();

            // most of this is shamelessly stolen from imageproc's code, which does
            // the curve but doesn't allow us to set the width of the line
            let cubic_bezier_curve = |t: f32| {
                let t2 = t * t;
                let t3 = t2 * t;
                let mt = 1.0 - t;
                let mt2 = mt * mt;
                let mt3 = mt2 * mt;
                let x = (start.x() * mt3)
                    + (3.0 * control_a.x() * mt2 * t)
                    + (3.0 * control_b.x() * mt * t2)
                    + (end.x() * t3);
                let y = (start.y() * mt3)
                    + (3.0 * control_a.y() * mt2 * t)
                    + (3.0 * control_b.y() * mt * t2)
                    + (end.y() * t3);
                Vector2F::new(x.round(), y.round()) // round to nearest pixel, to avoid ugly line artifacts
            };

            #[inline]
            fn distance(a: Vector2F, b: Vector2F) -> f32 {
                let r = ((b.x() - a.x()).powi(2)) + ((b.y() - a.y()).powi(2));
                r.sqrt()
            }

            let curve_length_bound = distance(start, control_a)
                + distance(control_a, control_b)
                + distance(control_b, end);
            let clb2 = curve_length_bound.powi(2);

            let num_segments = ((clb2 + 800.0).sqrt() / 8.0) as i32;
            let t_interval = 1f32 / (num_segments as f32);

            let bcii = BCIntervalIter {
                prev: 0.0,
                t_interval,
                internal: (0..num_segments),
            };

            bcii.collect::<Vec<(f32, f32)>>()
                .par_iter()
                .for_each(|(t1, t2)| {
                    let a1 = cubic_bezier_curve(*t1);
                    let a2 = cubic_bezier_curve(*t2);
                    rasterize_line(
                        target,
                        width as i32,
                        height as i32,
                        LineSegment2F::new(a1, a2),
                        color,
                        brush.width() as i32,
                    );
                });
        });
    }
}
