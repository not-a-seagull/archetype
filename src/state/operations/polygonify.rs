// GPLv3 License

use super::{StateDataType, StateLine, Curve, Polyshape, DataObjectContainer, GraphicalState, StateDataLoc, StateOperation};
use crate::{PolygonEdge, PolygonType, BezierCurve, Line, Point, Polygon};
use euclid::default::Point2D;
use itertools::Itertools;
use pathfinder_geometry::vector::Vector2F;
use smallvec::SmallVec;

impl GraphicalState {
    /// Turn a set of beziers or lines into a polygon.
    pub fn polygonify_selected_items(
        &mut self,
        brush: usize,
        create_new_line: bool,
        duplicate: bool,
    ) {
        trait HasEndPoints {
            fn endpoint1(&self) -> Vector2F;
            fn endpoint2(&self) -> Vector2F;
            fn set_endpoint1(&mut self, vctr: Vector2F);
            fn set_endpoint2(&mut self, vctr: Vector2F);
        }

        impl HasEndPoints for BezierCurve {
            #[inline]
            fn endpoint1(&self) -> Vector2F {
                self.points()[0]
            }
            #[inline]
            fn endpoint2(&self) -> Vector2F {
                self.points()[3]
            }
            #[inline]
            fn set_endpoint1(&mut self, vctr: Vector2F) {
                self.points_mut()[0] = vctr;
            }
            #[inline]
            fn set_endpoint2(&mut self, vctr: Vector2F) {
                self.points_mut()[3] = vctr;
            }
        }

        impl<Ln: Line<f32>> HasEndPoints for Ln {
            #[inline]
            fn endpoint1(&self) -> Vector2F {
                self.from()
            }
            #[inline]
            fn endpoint2(&self) -> Vector2F {
                self.to()
            }
            #[inline]
            fn set_endpoint1(&mut self, vctr: Vector2F) {
                self.set_from(vctr);
            }
            #[inline]
            fn set_endpoint2(&mut self, vctr: Vector2F) {
                self.set_to(vctr);
            }
        }

        impl HasEndPoints for DataObjectContainer {
            #[inline]
            fn endpoint1(&self) -> Vector2F {
                match self {
                    Self::Curve(Curve { ref curve, .. }) => curve.endpoint1(),
                    Self::StateLine(StateLine { ref points, .. }) => points.endpoint1(),
                    _ => unreachable!(),
                }
            }

            #[inline]
            fn endpoint2(&self) -> Vector2F {
                match self {
                    Self::Curve(Curve { ref curve, .. }) => curve.endpoint2(),
                    Self::StateLine(StateLine { ref points, .. }) => points.endpoint2(),
                    _ => unreachable!(),
                }
            }

            #[inline]
            fn set_endpoint1(&mut self, val: Vector2F) {
                match self {
                    Self::Curve(Curve { ref mut curve, .. }) => curve.set_endpoint1(val),
                    Self::StateLine(StateLine { ref mut points, .. }) => points.set_endpoint1(val),
                    _ => unreachable!(),
                }
            }

            #[inline]
            fn set_endpoint2(&mut self, val: Vector2F) {
                match self {
                    Self::Curve(Curve { ref mut curve, .. }) => curve.set_endpoint2(val),
                    Self::StateLine(StateLine { ref mut points, .. }) => points.set_endpoint2(val),
                    _ => unreachable!(),
                }
            }
        }

        #[inline]
        fn connect_endpoints(
            t1: &mut dyn HasEndPoints,
            t2: &mut dyn HasEndPoints,
            create_new_line: bool,
            new_lines: &mut SmallVec<[[Point2D<f32>; 2]; 2]>,
        ) {
            const TOLERANCE: f32 = 2.0;

            // if we're creating a new line OR we're in tolerance range, set the two endpoints to be the same
            let (pt1, pt2) = (t1.endpoint2(), t2.endpoint1());
            let dist = pt1.distance_to(&pt2);

            if !create_new_line || dist < TOLERANCE {
                let (avg_x, avg_y) = ((pt1.x() + pt2.x()) / 2.0, (pt1.y() + pt2.y()) / 2.0);
                let avg = Vector2F::new(avg_x, avg_y);

                t1.set_endpoint2(avg);
                t2.set_endpoint1(avg);
            } else {
                let line = [pt1.into_euclid(), pt2.into_euclid()];
                new_lines.push(line);
            }
        }

        let locs: SmallVec<[StateDataLoc; 12]> = self.selected.drain(..).collect();

        let lines: Option<SmallVec<[DataObjectContainer; 12]>> = locs
            .into_iter()
            .sorted()
            .rev()
            .map(|s| {
                if let StateDataType::Line | StateDataType::Curve = s.0 {
                    Some(if duplicate {
                        s.item(self).clone_into_container()
                    } else {
                        s.take_item(self)
                    })
                } else {
                    None
                }
            })
            .collect();

        let mut new_lines = SmallVec::new();
        match lines {
            None => {
                println!("Found a non-line element in the selection.");
            }
            Some(mut sm) => {
                for i in 1..=sm.len() {
                    let (p1, p2) = if i == sm.len() {
                        let (p2, p1) = sm.split_at_mut(1);
                        (p1, p2)
                    } else {
                        sm.split_at_mut(i)
                    };

                    connect_endpoints(
                        &mut p1[p1.len() - 1],
                        &mut p2[0],
                        create_new_line,
                        &mut new_lines,
                    );
                }

                let poly = Polygon::new(
                    sm.into_iter()
                        .map(|d| match d {
                            DataObjectContainer::StateLine(StateLine { points, .. }) => {
                                points.into()
                            }
                            DataObjectContainer::Curve(Curve { curve, .. }) => curve.into(),
                            _ => unreachable!(),
                        })
                        .collect::<Vec<PolygonEdge>>(),
                    PolygonType::Outline,
                );
                let poly = Polyshape {
                    polygon: poly,
                    brush,
                };
                let did = self.next_data_id();
                self.history.push(StateOperation::Add(StateDataLoc(
                    StateDataType::Polygon,
                    did,
                )));
                self.polygons.insert(did, poly);

                self.update_history_add(
                    StateDataType::Line,
                    self.current_data_id(),
                    new_lines.len(),
                );
                self.lines.extend(
                    new_lines
                        .into_iter()
                        .map(|l| (self.next_data_id(), StateLine { points: l, brush })),
                );

                self.last_history_selected.take();
            }
        }
    }
}
