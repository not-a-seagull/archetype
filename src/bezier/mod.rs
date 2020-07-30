// GPL v3.0

use super::{rasterize_line, Brush, DrawTarget, Point, Rasterizable};
use parking_lot::RwLock;
use pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::SmallVec;
use std::{mem, ops::Range};

mod fit;

// I don't know how to write deserialization code, so here's a Vec version
#[derive(serde::Serialize, serde::Deserialize)]
struct BezierDeser {
    coords: Vec<f32>,
}

/// A bezier curve.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct BezierCurve {
    points: [Vector2F; 4],
}

impl Serialize for BezierCurve {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use smallvec::smallvec;

        let ser_form = BezierDeser {
            coords: self
                .points
                .iter()
                .map::<SmallVec<[f32; 2]>, _>(|vctr| smallvec![vctr.x(), vctr.y()])
                .flat_map(|i| i.into_iter())
                .collect(),
        };
        Serialize::serialize(&ser_form, serializer)
    }
}

impl<'de> Deserialize<'de> for BezierCurve {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ser_form: BezierDeser = Deserialize::deserialize(deserializer)?;

        let mut sm: SmallVec<[Vector2F; 4]> = SmallVec::with_capacity(4);
        let mut iter = ser_form.coords.into_iter();
        while let Some(fl1) = iter.next() {
            // group into two
            let fl2 = iter.next().expect("Invalid data");
            sm.push(Vector2F::new(fl1, fl2));
        }
        Ok(Self {
            points: sm.into_inner().unwrap(),
        })
    }
}

impl BezierCurve {
    #[inline]
    pub(crate) fn from_points(points: [Vector2F; 4]) -> Self {
        Self { points: points }
    }

    #[inline]
    pub fn fit_to(points: &[Vector2F], error: f32) -> Vec<Self> {
        fit::fit_curve(points, error).unwrap()
    }

    #[inline]
    pub fn point_at(&self, index: usize) -> Vector2F {
        self.points[index].clone()
    }

    #[inline]
    pub fn eval(&self, param: f32) -> Vector2F {
        match &self.points {
            [ref p1, ref p2, ref p3, ref p4] => de_casteljau4(param, *p1, *p2, *p3, *p4),
        }
    }

    #[inline]
    pub fn into_points(self) -> [Vector2F; 4] {
        self.points
    }

    #[inline]
    pub fn points(&self) -> &[Vector2F; 4] {
        &self.points
    }

    #[inline]
    pub fn edges(&self) -> Edges<'_> {
        let [start, control_a, control_b, end] = self.points();

        let curve_length_bound = distance(start, control_a)
            + distance(control_a, control_b)
            + distance(control_b, end);
        let clb2 = curve_length_bound.powi(2);

        let num_segments = ((clb2 + 800.0).sqrt() / 8.0) as i32;
        let t_interval = 1f32 / (num_segments as f32);

        Edges {
            curve: self,
            internal: (0..num_segments),
            t_interval,
            prev: 0.0,
        }
    }
}

impl Rasterizable for BezierCurve {
    #[inline]
    fn rasterize(&self, target: &DrawTarget, brush: Brush) {
        self.edges()
            .collect::<Vec<LineSegment2F>>()
            .iter()
            .for_each(|l| l.rasterize(target, brush))
    }
}

/// Iterate over a Bezier curve's edges.
pub struct Edges<'a> {
    curve: &'a BezierCurve,
    internal: Range<i32>,
    prev: f32,
    t_interval: f32,
}

impl<'a> Iterator for Edges<'a> {
    type Item = LineSegment2F;

    #[inline]
    fn next(&mut self) -> Option<LineSegment2F> {
        let i = match self.internal.next() {
            Some(i) => i,
            None => return None,
        };

        // figure out which T's to evaluate at
        let mut t1 = (i as f32 + 1.0) * self.t_interval;
        let t2 = t1;
        mem::swap(&mut self.prev, &mut t1);

        // evaluate the points at the t's
        let a1 = self.curve.eval(t1);
        let a2 = self.curve.eval(t2);

        Some(LineSegment2F::new(a1, a2))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.internal.size_hint()
    }
}

impl<'a> ExactSizeIterator for Edges<'a> {}

///
/// de Casteljau's algorithm for cubic bezier curves
///
#[inline]
pub fn de_casteljau4(t: f32, w1: Vector2F, w2: Vector2F, w3: Vector2F, w4: Vector2F) -> Vector2F {
    let wn1 = w1 * (1.0 - t) + w2 * t;
    let wn2 = w2 * (1.0 - t) + w3 * t;
    let wn3 = w3 * (1.0 - t) + w4 * t;

    de_casteljau3(t, wn1, wn2, wn3)
}

///
/// de Casteljau's algorithm for quadratic bezier curves
///
#[inline]
pub fn de_casteljau3(t: f32, w1: Vector2F, w2: Vector2F, w3: Vector2F) -> Vector2F {
    let wn1 = w1 * (1.0 - t) + w2 * t;
    let wn2 = w2 * (1.0 - t) + w3 * t;

    de_casteljau2(t, wn1, wn2)
}

///
/// de Casteljau's algorithm for lines
///
#[inline]
pub fn de_casteljau2(t: f32, w1: Vector2F, w2: Vector2F) -> Vector2F {
    w1 * (1.0 - t) + w2 * t
}

#[inline]
pub fn distance(v1: &Vector2F, v2: &Vector2F) -> f32 {
    let a = (v1.x() - v2.x()).powi(2) + (v1.y() - v2.y()).powi(2);
    a.sqrt()
}
