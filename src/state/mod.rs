// GPL v3.0

mod data;

use super::{
    colors, de_casteljau4, BezierCurve, Brush, Color, DrawTarget, DynamicColor, Point, Project,
    Rasterizable,
};
use data::*;
use euclid::default::Point2D;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{self, BresenhamLineIter, Canvas};
use ordered_float::NotNan;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    mem,
};

/// The current graphical state.
#[derive(Serialize, Deserialize)]
pub struct GraphicalState {
    curves: Vec<Curve>, // usize is brush index
    buffered_lines: Vec<BufferedLine>,
    lines: Vec<StateLine>,
    polygons: Vec<Polyshape>,
    history: SmallVec<[StateOperation; 25]>,

    selected: Vec<StateDataLoc>,
    last_history_selected: Option<usize>,
}

impl GraphicalState {
    #[inline]
    pub fn new() -> Self {
        Self {
            curves: Vec::new(),
            buffered_lines: Vec::new(),
            lines: Vec::new(),
            polygons: Vec::new(),
            history: SmallVec::new(),
            selected: Vec::new(),
            last_history_selected: None,
        }
    }

    #[inline]
    pub fn curves(&self) -> &Vec<Curve> {
        &self.curves
    }

    #[inline]
    pub fn curves_mut(&mut self) -> &mut Vec<Curve> {
        &mut self.curves
    }

    #[inline]
    pub fn lines(&self) -> &Vec<StateLine> {
        &self.lines
    }

    #[inline]
    pub fn lines_mut(&mut self) -> &mut Vec<StateLine> {
        &mut self.lines
    }

    #[inline]
    pub fn polygons(&self) -> &Vec<Polyshape> {
        &self.polygons
    }

    #[inline]
    pub fn polygons_mut(&mut self) -> &mut Vec<Polyshape> {
        &mut self.polygons
    }

    #[inline]
    pub fn buffered_lines(&self) -> &Vec<BufferedLine> {
        &self.buffered_lines
    }

    #[inline]
    pub fn buffered_lines_mut(&mut self) -> &mut Vec<BufferedLine> {
        &mut self.buffered_lines
    }

    #[inline]
    pub fn history(&self) -> &[StateOperation] {
        &self.history
    }

    #[inline]
    pub fn selected(&self) -> &[StateDataLoc] {
        &self.selected
    }

    /// Update the history to go below the history limit.
    #[inline]
    pub fn update_history_add(&mut self, kind: StateDataType, item_num: usize) {
        self.unselect();
        let new_index = kind.assoc_collection(self).length();
        (new_index..new_index + item_num).into_iter().for_each(|i| {
            let data_loc = StateOperation::Add(StateDataLoc(kind, i));
            self.history.push(data_loc);
        });

        while self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }

        //println!("History: {:?}", &self.history);
    }

    /// Unselect all items
    #[inline]
    pub fn unselect(&mut self) {
        self.selected.clear();
    }

    /// Push an item from the history into the selected item.
    #[inline]
    pub fn select_from_history(&mut self) {
        let new_index = if let Some(mut i) = self.last_history_selected.take() {
            self.last_history_selected = Some(i.saturating_sub(1));
            i
        } else {
            let index = self.history.len() - 1;
            self.last_history_selected = Some(index - 1);
            index
        };

        let data_loc = match self.history[new_index] {
            StateOperation::Add(sl) => sl.clone(),
        };
        if !self.selected.contains(&data_loc) {
            self.selected.push(data_loc);
        }
    }

    /// Add a buffered line.
    #[inline]
    pub fn add_buffered_line(&mut self, pt1: Point2D<f32>, pt2: Point2D<f32>) {
        self.buffered_lines.push(BufferedLine([pt1, pt2]));
    }

    /// Drop the buffered lines.
    #[inline]
    pub fn drop_buffered_lines(&mut self) {
        self.buffered_lines.clear();
    }

    /// Convert the buffered items into lines.
    #[inline]
    pub fn convert_buffered_lines(&mut self, brush: usize) {
        let lines = self
            .buffered_lines
            .drain(..)
            .map(|f| StateLine { points: f.0, brush })
            .collect::<SmallVec<[StateLine; 10]>>();

        let len = lines.len();
        self.update_history_add(StateDataType::Line, len);
        self.lines.extend(lines);
    }

    /// Convert the buffered items into a bezier curve.
    pub fn bezierify_buffered_lines(&mut self, brush: usize, error: f32) {
        let pts: SmallVec<[Vector2F; 12]> = self
            .buffered_lines
            .drain(..)
            .map(|bl| bl.0)
            .flat_map(|l| l.iter().copied().collect::<Vec<Point2D<f32>>>().into_iter())
            .map(|pt| Vector2F::new(pt.x, pt.y))
            .collect();

        let curves = BezierCurve::fit_to(&pts, error)
            .into_iter()
            .map(|v| Curve { curve: v, brush })
            .collect::<SmallVec<[Curve; 10]>>();
        let len = curves.len();

        self.update_history_add(StateDataType::Curve, len);
        self.curves.extend(curves);
    }

    /// Turn a set of beziers or lines into a polygon.
    pub fn polygonify_selected_items(&mut self, create_new_line: bool, duplicate: bool) {}

    /// Select the element closest to a click location.
    pub fn select_closest_element<P: crate::Point<f32> + Sync>(&mut self, loc: P) {
        // build a hash map of all of the lines and their associated data locations
        // TODO: maybe cache this?
        let point_map: Vec<(Point2D<f32>, usize, &(dyn DataObject + Sync + 'static))> = self
            .iter_data_objects()
            .flat_map(|(i, d)| {
                d.points()
                    .into_iter()
                    .map(move |pt| (pt.into_euclid(), i, d))
            })
            .collect();
        if !point_map.is_empty() {
            let (_, index, item) = point_map
                .par_iter()
                .map(|(pt, i, d)| (pt.distance_to(&loc), i, d))
                .min_by(|(dist1, _i1, _d1), (dist2, _i2, _d2)| {
                    NotNan::new(*dist1)
                        .unwrap()
                        .cmp(&NotNan::new(*dist2).unwrap())
                })
                .unwrap();
            self.selected.push(StateDataLoc(item.data_type(), *index));
        }
    }

    /// Get an iterator over all data objects (except for buffered lines).
    pub fn iter_data_objects(
        &self,
    ) -> impl Iterator<Item = (usize, &(dyn DataObject + Sync + 'static))> {
        self.polygons
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p as _))
            .chain(self.curves.iter().enumerate().map(|(i, c)| (i, c as _)))
            .chain(self.lines.iter().enumerate().map(|(i, l)| (i, l as _)))
    }

    /// Rasterize this graphical state onto an image.
    pub fn rasterize(&self, target: &DrawTarget, project: &Project) {
        #[inline]
        fn rasterize_item(
            this: &GraphicalState,
            data_type: StateDataType,
            index: usize,
            target: &DrawTarget,
            item: &dyn Rasterizable,
            ci: usize,
            project: &Project,
        ) {
            // figure out the item location
            let data_loc = StateDataLoc(data_type, index);

            let brush = project.brush(ci).expect("Brush ID Mismatch").clone();
            let brush = if this.selected.contains(&data_loc) {
                const SELECT_BRUSH: Brush = Brush::new_const(DynamicColor::Solid(colors::BLUE), 0);
                let mut select_brush = SELECT_BRUSH.clone();
                select_brush.set_width(brush.width());
                select_brush
            } else {
                brush
            };

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

        self.polygons.par_iter().enumerate().for_each(|(i, pl)| {
            rasterize_item(
                self,
                StateDataType::Polygon,
                i,
                target,
                &pl.polygon,
                pl.brush.clone(),
                project,
            );
        });

        // draw some curves
        self.curves
            .par_iter()
            .enumerate()
            .for_each(|(i, Curve { curve, brush: ci })| {
                // get the brush we are using
                rasterize_item(self, StateDataType::Curve, i, target, curve, *ci, project);
            });

        // rasterize the lines
        self.lines.par_iter().enumerate().for_each(|(i, ln)| {
            let line = match ln.points {
                [Point2D { x: x1, y: y1, .. }, Point2D { x: x2, y: y2, .. }] => LineSegment2F::new(
                    Vector2F::new(x1 as f32, y1 as f32),
                    Vector2F::new(x2 as f32, y2 as f32),
                ),
            };

            rasterize_item(
                self,
                StateDataType::Line,
                i,
                target,
                &line,
                ln.brush,
                project,
            );
        });

        // also rasterize the line buffer
        self.buffered_lines
            .par_iter()
            .enumerate()
            .for_each(|(i, pts)| {
                const BUFFERED_BRUSH: Brush = Brush::new_const(DynamicColor::Solid(colors::RED), 3);

                let line = match pts {
                    BufferedLine([Point2D { x: x1, y: y1, .. }, Point2D { x: x2, y: y2, .. }]) => {
                        LineSegment2F::new(
                            Vector2F::new(*x1 as f32, *y1 as f32),
                            Vector2F::new(*x2 as f32, *y2 as f32),
                        )
                    }
                };

                line.rasterize(target, BUFFERED_BRUSH.clone());
            });
    }
}
