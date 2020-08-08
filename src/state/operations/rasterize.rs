// GPLv3 License

use super::{BufferedLine, Curve, GraphicalState, StateDataLoc, StateDataType, StateLine};
use crate::{colors, Brush, DrawTarget, DynamicColor, Project, Rasterizable};
use euclid::default::Point2D;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F};
use rayon::prelude::*;
use std::{borrow::Cow, mem};

impl GraphicalState {
    /// Rasterize this graphical state onto an image.
    pub fn rasterize(&self, target: &DrawTarget, project: &Project) {
        #[inline]
        fn rasterize_item(
            this: &GraphicalState,
            sel_guard: &[StateDataLoc],
            data_type: StateDataType,
            index: &usize,
            target: &DrawTarget,
            item: &dyn Rasterizable,
            ci: usize,
            project: &Project,
        ) {
            // figure out the item location
            let data_loc = StateDataLoc(data_type, *index);

            let mut brush = Cow::Borrowed(project.brush(ci).expect("Brush ID Mismatch"));
            if sel_guard.contains(&data_loc) {
                const SELECT_COLOR: DynamicColor = DynamicColor::Solid(colors::BLUE);
                brush.to_mut().set_color(SELECT_COLOR);
            }

            item.rasterize(target, &*brush);
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

        self.polygons.par_iter().for_each(|(i, pl)| {
            rasterize_item(
                self,
                &self.selected,
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
            .for_each(|(i, Curve { curve, brush: ci })| {
                // get the brush we are using
                rasterize_item(
                    self,
                    &self.selected,
                    StateDataType::Curve,
                    i,
                    target,
                    curve,
                    *ci,
                    project,
                );
            });

        // rasterize the lines
        self.lines.par_iter().for_each(|(i, ln)| {
            let line = match ln.points {
                [Point2D { x: x1, y: y1, .. }, Point2D { x: x2, y: y2, .. }] => LineSegment2F::new(
                    Vector2F::new(x1 as f32, y1 as f32),
                    Vector2F::new(x2 as f32, y2 as f32),
                ),
            };

            rasterize_item(
                self,
                &self.selected,
                StateDataType::Line,
                i,
                target,
                &line,
                ln.brush,
                project,
            );
        });

        // also rasterize the line buffer
        self.buffered_lines.par_iter().for_each(|pts| {
            const BUFFERED_BRUSH: Brush = Brush::new_const(DynamicColor::Solid(colors::RED), 3);

            let line = match pts {
                BufferedLine([Point2D { x: x1, y: y1, .. }, Point2D { x: x2, y: y2, .. }]) => {
                    LineSegment2F::new(
                        Vector2F::new(*x1 as f32, *y1 as f32),
                        Vector2F::new(*x2 as f32, *y2 as f32),
                    )
                }
            };

            line.rasterize(target, &BUFFERED_BRUSH);
        });
    }
}
