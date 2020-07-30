// GPLv3 License

use super::{
    rasterize_thick_line, rasterize_thin_line, BezierCurve, Brush, DrawTarget, IntersectsAt, Line,
    Point, Rasterizable,
};
use euclid::default::Point2D;
use image::{Rgba, RgbaImage};
use itertools::Itertools;
use ordered_float::NotNan;
use parking_lot::RwLock;
use pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F};
use rayon::{iter, prelude::*};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::SmallVec;
use std::{cmp::Ordering, collections::HashMap};

/// Serializable form of polygon edge.
#[derive(Deserialize, Serialize)]
enum PolygonEdgeSerde {
    Straight(Point2D<f32>, Point2D<f32>),
    Curved(BezierCurve),
}

/// A polygon's edge.
#[derive(Clone)]
pub enum PolygonEdge {
    Straight(LineSegment2F),
    Curved(BezierCurve),
}

impl PolygonEdge {
    #[inline]
    pub fn is_straight(&self) -> bool {
        if let Self::Straight(_) = self {
            true
        } else {
            false
        }
    }

    #[inline]
    fn into_serde(&self) -> PolygonEdgeSerde {
        match self {
            Self::Straight(ref l) => PolygonEdgeSerde::Straight(
                Point2D::new(l.from_x(), l.from_y()),
                Point2D::new(l.to_x(), l.to_y()),
            ),
            Self::Curved(ref c) => PolygonEdgeSerde::Curved(c.clone()),
        }
    }
}

impl Serialize for PolygonEdge {
    #[inline]
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        Serialize::serialize(&self.into_serde(), ser)
    }
}

impl<'de> Deserialize<'de> for PolygonEdge {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let repr: PolygonEdgeSerde = Deserialize::deserialize(de)?;
        Ok(match repr {
            PolygonEdgeSerde::Straight(
                Point2D { x: x1, y: y1, .. },
                Point2D { x: x2, y: y2, .. },
            ) => Self::Straight(LineSegment2F::new(
                Vector2F::new(x1, y1),
                Vector2F::new(x2, y2),
            )),
            PolygonEdgeSerde::Curved(c) => Self::Curved(c),
        })
    }
}

impl From<LineSegment2F> for PolygonEdge {
    #[inline]
    fn from(ls: LineSegment2F) -> PolygonEdge {
        Self::Straight(ls)
    }
}

impl From<BezierCurve> for PolygonEdge {
    #[inline]
    fn from(bz: BezierCurve) -> PolygonEdge {
        Self::Curved(bz)
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum PolygonType {
    Fill,
    Outline,
}

/// A filled polygon.
#[derive(Clone, Serialize, Deserialize)]
pub struct Polygon {
    edges: SmallVec<[PolygonEdge; 6]>,
    mode: PolygonType,
}

impl Polygon {
    #[inline]
    fn new<PE, I: IntoIterator<Item = PE>>(iter: I, mode: PolygonType) -> Self
    where
        PE: Into<PolygonEdge>,
    {
        Self {
            edges: iter.into_iter().map(|pe| pe.into()).collect(),
            mode,
        }
    }

    #[inline]
    pub fn as_straight_edges(&self) -> impl Iterator<Item = LineSegment2F> + '_ {
        self.edges
            .iter()
            .filter(|s| s.is_straight())
            .map(|s| {
                if let PolygonEdge::Straight(ref l) = s {
                    l.clone()
                } else {
                    unreachable!()
                }
            })
            .chain(
                self.edges
                    .iter()
                    .filter(|s| !s.is_straight())
                    .map(|s| {
                        if let PolygonEdge::Curved(ref bz) = s {
                            bz
                        } else {
                            unreachable!()
                        }
                    })
                    .flat_map(|bz| bz.edges()),
            )
    }

    #[inline]
    fn fill(&self, target: &DrawTarget, brush: Brush) {
        // primitive home-brewed scanline algorithm
        // first, figure out the bounds of the polygon. min/max x/y
        // map into euclid points so that we can parallelize it
        let edges = self
            .as_straight_edges()
            .map(|l| {
                (
                    Point2D::new(l.from_x(), l.from_y()),
                    Point2D::new(l.to_x(), l.to_y()),
                )
            })
            .collect::<Vec<(Point2D<f32>, Point2D<f32>)>>();

        let x_iter = edges.par_iter().flat_map(|(pt1, pt2)| {
            iter::once(NotNan::new(pt1.x()).unwrap())
                .chain(iter::once(NotNan::new(pt2.x()).unwrap()))
        });
        let y_iter = edges.par_iter().flat_map(|(pt1, pt2)| {
            iter::once(NotNan::new(pt1.y()).unwrap())
                .chain(iter::once(NotNan::new(pt2.y()).unwrap()))
        });

        let min_x = x_iter.clone().min().unwrap();
        let max_x = x_iter.max().unwrap();
        let min_y = y_iter.clone().min().unwrap();
        let max_y = y_iter.max().unwrap();

        let min_x = min_x.floor() as u32;
        let max_x = max_x.ceil() as u32;
        let min_y = min_y.floor() as u32;
        let max_y = max_y.ceil() as u32;

        (min_y..=max_y).into_par_iter().for_each(|y| {
            let y = y as f32;
            let line = LineSegment2F::from_x1_y1_x2_y2(min_x as f32, y, max_x as f32, y);

            let intersections_collected = edges
                .par_iter()
                .map(|l| (*l, line.intersects_at(l)))
                .filter(|(_line, t)| t.is_some())
                .collect::<Vec<((Point2D<f32>, Point2D<f32>), Option<f32>)>>();
            let intersections = intersections_collected
                .par_iter()
                .map(|(line, t)| line.sample_at(t.unwrap()));
            intersections
                .clone()
                .step_by(2)
                .zip(intersections.skip(1).step_by(2))
                .for_each(|(pt1, pt2)| {
                    let pt1: Point2D<f32> = pt1; // this makes the compiler happy

                    rasterize_thin_line(
                        target,
                        &(pt1.into_tuple(), pt2.into_tuple()),
                        brush.clone(),
                    );
                });
        });
    }
}

impl Rasterizable for Polygon {
    #[inline]
    fn rasterize(&self, target: &DrawTarget, brush: Brush) {
        match self.mode {
            PolygonType::Outline => {
                self.as_straight_edges()
                    .for_each(|l| l.rasterize(target, brush.clone()));
            }
            PolygonType::Fill => {
                self.fill(target, brush);
            }
        }
    }
}
