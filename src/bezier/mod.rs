// GPL v3.0

use pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::SmallVec;

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
            [ref p1, ref p2, ref p3, ref p4] => fit::de_casteljau4(param, *p1, *p2, *p3, *p4),
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
}
