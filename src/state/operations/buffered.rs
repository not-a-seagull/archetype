// GPLv3 License

use super::{BufferedLine, Curve, GraphicalState, StateDataType, StateLine};
use crate::BezierCurve;
use euclid::default::Point2D;
use pathfinder_geometry::vector::Vector2F;
use smallvec::SmallVec;

impl GraphicalState {
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
        let data_id = self.current_data_id();

        // generate the next ID's ahead of time, so we don't need imm.
        // access during the iterator
        let data_ids = self.next_data_ids(self.buffered_lines.len());

        let lines = self
            .buffered_lines
            .drain(..)
            .enumerate()
            .map(|(i, f)| (data_ids[i], StateLine { points: f.0, brush }))
            .collect::<SmallVec<[(usize, StateLine); 10]>>();

        let len = lines.len();
        self.update_history_add(StateDataType::Line, data_id, len);
        self.lines.extend(lines);
    }

    /// Convert the buffered items into a bezier curve.
    pub fn bezierify_buffered_lines(&mut self, brush: usize, error: f32) {
        let pts: SmallVec<[Vector2F; 12]> = self
            .buffered_lines
            .drain(..)
            .flat_map(|l| SmallVec::<[Point2D<f32>; 2]>::from_buf(l.0).into_iter())
            .map(|pt| Vector2F::new(pt.x, pt.y))
            .collect();

        let data_id = self.current_data_id();
        let curves = BezierCurve::fit_to(pts, error)
            .into_iter()
            .map(|v| (self.next_data_id(), Curve { curve: v, brush }))
            .collect::<SmallVec<[(usize, Curve); 10]>>();
        let len = curves.len();

        self.update_history_add(StateDataType::Curve, data_id, len);
        self.curves.extend(curves);
    }
}
