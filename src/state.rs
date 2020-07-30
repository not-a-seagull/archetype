// GPL v3.0

use super::{
    colors, de_casteljau4, rasterize_line, BezierCurve, Brush, Color, DrawTarget, Project,
    Rasterizable,
};
use euclid::default::Point2D;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{self, BresenhamLineIter, Canvas};
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

impl GraphicalState {
    pub fn new() -> Self {
        Self {
            curves: Vec::new(),
            buffered_lines: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Add a buffered line.
    #[inline]
    pub fn add_buffered_line(&mut self, pt1: Point2D<f32>, pt2: Point2D<f32>) {
        self.buffered_lines.push([pt1, pt2]);
    }

    /// Drop the buffered lines.
    #[inline]
    pub fn drop_buffered_lines(&mut self) {
        self.buffered_lines.clear();
    }

    /// Convert the buffered items into lines.
    #[inline]
    pub fn convert_buffered_lines(&mut self, brush: usize) {
        self.lines.extend(
            self.buffered_lines
                .drain(..)
                .map(|f| Line { points: f, brush }),
        );
    }

    /// Convert the buffered items into a bezier curve.
    pub fn bezierify_buffered_lines(&mut self, brush: usize, error: f32) {
        let pts: Vec<Vector2F> = self
            .buffered_lines
            .drain(..)
            .flat_map(|l| l.iter().copied().collect::<Vec<Point2D<f32>>>().into_iter())
            .map(|pt| Vector2F::new(pt.x, pt.y))
            //.sorted_by(|pt1, pt2| {
            //    NotNan::new(pt1.length())
            //        .unwrap()
            //        .cmp(&NotNan::new(pt2.length()).unwrap())
            //})
            .collect();
        self.curves.extend(
            BezierCurve::fit_to(&pts, error)
                .into_iter()
                .map(|v| (v, brush)),
        );
    }

    /// Rasterize this graphical state onto an image.
    pub fn rasterize(&self, target: &DrawTarget, project: &Project) {
        #[inline]
        fn rasterize_item(
            target: &DrawTarget,
            item: &dyn Rasterizable,
            ci: usize,
            project: &Project,
        ) {
            let brush = project.brush(ci).expect("Brush ID Mismatch").clone();
            item.rasterize(target, brush);
        }

        let img = RwLock::upgradable_read(target);

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
            rasterize_item(target, curve, *ci, project);
        });

        // also rasterize the line buffer
        self.buffered_lines.par_iter().for_each(|pts| {
            const BUFFERED_BRUSH: Brush = Brush::new(colors::RED, 3);

            let line = match pts {
                [Point2D { x: x1, y: y1, .. }, Point2D { x: x2, y: y2, .. }] => LineSegment2F::new(
                    Vector2F::new(*x1 as f32, *y1 as f32),
                    Vector2F::new(*x2 as f32, *y2 as f32),
                ),
            };

            line.rasterize(target, BUFFERED_BRUSH.clone());
        });

        // rasterize the lines
        self.lines.par_iter().for_each(|ln| {
            let line = match ln.points {
                [Point2D { x: x1, y: y1, .. }, Point2D { x: x2, y: y2, .. }] => LineSegment2F::new(
                    Vector2F::new(x1 as f32, y1 as f32),
                    Vector2F::new(x2 as f32, y2 as f32),
                ),
            };

            rasterize_item(target, &line, ln.brush, project);
        });
    }
}
