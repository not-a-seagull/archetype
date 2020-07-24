// GPL v3.0

use super::{colors, BezierCurve, Brush, Color, Project};
use euclid::default::Point2D;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{self, BresenhamLineIter, Canvas};
use itertools::Itertools;
use ordered_float::NotNan;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{collections::HashMap, mem};

// repr of a line
#[derive(Serialize, Deserialize)]
struct Line {
    points: [Point2D<f32>; 2],
    brush: usize,
}

/// The current graphical state.
#[derive(Serialize, Deserialize)]
pub struct GraphicalState {
    curves: Vec<(BezierCurve, usize)>, // usize is brush index
    buffered_lines: Vec<[Point2D<f32>; 2]>,
    lines: Vec<Line>,
}

// function to draw a thicker line segment onto a canvas
#[inline]
fn rasterize_line(
    c: &RwLock<(RgbaImage, bool)>,
    width: i32,
    height: i32,
    line: LineSegment2F,
    color: Rgba<u8>,
    line_width: i32,
) {
    let line_iter =
        BresenhamLineIter::new((line.from_x(), line.from_y()), (line.to_x(), line.to_y()));

    let mut writer = c.write();
    line_iter
        .filter(|(x, y)| *x >= 0 && *x < width && *y >= 0 && *y < height)
        .for_each(|pt| {
            drawing::draw_filled_ellipse_mut(
                &mut writer.0,
                (pt.0, pt.1),
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
            curves: Vec::new(),
            //            buffered_lines: Vec::new(),
            buffered_lines: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Add a buffered line.
    pub fn add_buffered_line(&mut self, pt1: Point2D<f32>, pt2: Point2D<f32>) {
        self.buffered_lines.push([pt1, pt2]);
    }

    /// Drop the buffered lines.
    pub fn drop_buffered_lines(&mut self) {
        self.buffered_lines.clear();
    }

    /// Convert the buffered items into a bezier curve.
    pub fn bezierify_buffered_lines(&mut self, brush: usize) {
        let pts: Vec<Vector2F> = self
            .buffered_lines
            .drain(..)
            .flat_map(|l| l.iter().copied().collect::<Vec<Point2D<f32>>>().into_iter())
            .map(|pt| Vector2F::new(pt.x, pt.y))
            .sorted_by(|pt1, pt2| {
                NotNan::new(pt1.length())
                    .unwrap()
                    .cmp(&NotNan::new(pt2.length()).unwrap())
            })
            .collect();
        self.curves.extend(
            BezierCurve::fit_to(&pts, 2.0)
                .into_iter()
                .map(|v| (v, brush)),
        );
        println!("{:?}", &self.curves);
    }

    /// Rasterize this graphical state onto an image.
    pub fn rasterize(&self, target: &RwLock<(RgbaImage, bool)>, project: &RwLock<Project>) {
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

        let img = RwLock::upgradable_read(target);
        let (width, height) = img.0.dimensions();

        // update the bool if necessary
        if !img.1 {
            let mut img = RwLockUpgradableReadGuard::upgrade(img);
            img.1 = true;
            mem::drop(img);
        } else {
            mem::drop(img); // don't hog the lock
        }

        // draw some curves
        self.curves.par_iter().for_each(|(curve, ci)| {
            // get the brush we are using
            let brush = project
                .read()
                .brush(*ci)
                .expect("Brush ID Mismatch")
                .clone();
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

        // also rasterize the line buffer
        self.buffered_lines.par_iter().for_each(|pts| {
            let line = match pts {
                [Point2D { x: x1, y: y1, .. }, Point2D { x: x2, y: y2, .. }] => LineSegment2F::new(
                    Vector2F::new(*x1 as f32, *y1 as f32),
                    Vector2F::new(*x2 as f32, *y2 as f32),
                ),
            };

            rasterize_line(
                target,
                width as i32,
                height as i32,
                line,
                Rgba([std::u8::MAX, 0, 0, std::u8::MAX]),
                3,
            );
        });
    }
}
