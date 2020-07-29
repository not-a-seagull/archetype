// GPLv3 License

use euclid::{Point2D, Rect};
use num_traits::{Float, Num, Pow, Zero};
use pathfinder_geometry::{
    line_segment::LineSegment2F,
    vector::{Vector2F, Vector2I},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp,
    ops::{Mul, Sub},
};

pub trait PathfinderVector<T: Copy>: Point<T> {}

pub trait Point<T: Copy>: Copy {
    fn x(&self) -> T;
    fn y(&self) -> T;
    fn set_x(&mut self, x: T);
    fn set_y(&mut self, y: T);
    fn from_x_y(x: T, y: T) -> Self;

    #[inline]
    fn into_euclid<Dim>(self) -> Point2D<T, Dim> {
        Point2D::new(self.x(), self.y())
    }
    #[inline]
    fn into_tuple(self) -> (T, T) {
        (self.x(), self.y())
    }
    #[inline]
    fn into_pathfinder<V: PathfinderVector<T>>(self) -> V {
        V::from_x_y(self.x(), self.y())
    }
    #[inline]
    fn distance_to<Other: Point<T>>(&self, other: &Other) -> T
    where
        T: Pow<i32, Output = T> + Float + Sub,
    {
        let a = (self.x() - other.x())
            .pow(2)
            .sub((self.y() - other.y()).pow(2));
        a.sqrt()
    }
    #[inline]
    fn cross_product<Other: Point<T>>(&self, other: &Other) -> T
    where
        T: Mul<T, Output = T> + Sub<T, Output = T>,
    {
        (self.x().mul(other.y())).sub(self.y().mul(other.x()))
    }
    #[inline]
    fn subtract<Other: Point<T>, Res: Point<T>>(&self, other: &Other) -> Res
    where
        T: Sub<T, Output = T>,
    {
        Res::from_x_y(self.x().sub(other.x()), self.y().sub(other.y()))
    }
}

impl<T: Copy, Dim> Point<T> for Point2D<T, Dim> {
    #[inline]
    fn x(&self) -> T {
        self.x
    }
    #[inline]
    fn y(&self) -> T {
        self.y
    }
    #[inline]
    fn set_x(&mut self, x: T) {
        self.x = x;
    }
    #[inline]
    fn set_y(&mut self, y: T) {
        self.y = y;
    }
    #[inline]
    fn from_x_y(x: T, y: T) -> Self {
        euclid::point2(x, y)
    }
}

impl<T: Copy> Point<T> for (T, T) {
    #[inline]
    fn x(&self) -> T {
        self.0
    }
    #[inline]
    fn y(&self) -> T {
        self.1
    }
    #[inline]
    fn set_x(&mut self, x: T) {
        self.0 = x;
    }
    #[inline]
    fn set_y(&mut self, y: T) {
        self.1 = y;
    }
    #[inline]
    fn from_x_y(x: T, y: T) -> Self {
        (x, y)
    }
}

impl Point<i32> for Vector2I {
    #[inline]
    fn x(&self) -> i32 {
        Vector2I::x(*self)
    }
    #[inline]
    fn y(&self) -> i32 {
        Vector2I::y(*self)
    }
    #[inline]
    fn set_x(&mut self, x: i32) {
        Vector2I::set_x(self, x);
    }
    #[inline]
    fn set_y(&mut self, y: i32) {
        Vector2I::set_y(self, y);
    }
    #[inline]
    fn from_x_y(x: i32, y: i32) -> Self {
        Self::new(x, y)
    }
}

impl PathfinderVector<i32> for Vector2I {}

impl Point<f32> for Vector2F {
    #[inline]
    fn x(&self) -> f32 {
        Vector2F::x(*self)
    }
    #[inline]
    fn y(&self) -> f32 {
        Vector2F::y(*self)
    }
    #[inline]
    fn set_x(&mut self, x: f32) {
        Vector2F::set_x(self, x);
    }
    #[inline]
    fn set_y(&mut self, y: f32) {
        Vector2F::set_y(self, y);
    }
    #[inline]
    fn from_x_y(x: f32, y: f32) -> Self {
        Self::new(x, y)
    }
}

impl PathfinderVector<f32> for Vector2F {}

pub trait Line<T: Copy>: Copy {
    fn to_x(&self) -> T;
    fn from_x(&self) -> T;
    fn to_y(&self) -> T;
    fn from_y(&self) -> T;

    #[inline]
    fn to<Pt: Point<T>>(&self) -> Pt {
        Pt::from_x_y(self.to_x(), self.to_y())
    }
    #[inline]
    fn from<Pt: Point<T>>(&self) -> Pt {
        Pt::from_x_y(self.from_x(), self.from_y())
    }

    fn from_x1_y1_x2_y2(x1: T, y1: T, x2: T, y2: T) -> Self;
    #[inline]
    fn from_points<Pt1: Point<T>, Pt2: Point<T>>(pt1: Pt1, pt2: Pt2) -> Self {
        Self::from_x1_y1_x2_y2(pt1.x(), pt1.y(), pt2.x(), pt2.y())
    }

    #[inline]
    fn length(&self) -> T
    where
        T: Pow<i32, Output = T> + Float + Sub,
    {
        self.to::<(T, T)>().distance_to(&self.from::<(T, T)>())
    }

    /*
        #[inline]
        fn bounding_box<Dim>(&self) -> Rect<T, Dim>
        where
            T: PartialOrd + std::fmt::Display + Copy,
        {
            macro_rules! unstable_cmp {
                ($x1: expr, $x2: expr, $ret_if_x1_ge_x2: expr, $ret_if_x1_le_x2: expr) => {{
                    match std::cmp::PartialOrd::partial_cmp(&$x1, &$x2) {
                        None => panic!("Bad comparison encountered: {} <-> {}", $x1, $x2),
                        Some(cmp::Ordering::Greater) => $ret_if_x1_ge_x2,
                        Some(cmp::Ordering::Less) => $ret_if_x1_le_x2,
                        _ => $x1, // doesn't matter
                    }
                }};
            }

            macro_rules! unstable_min {
                ($x1: expr, $x2: expr) => {
                    unstable_cmp!($x1, $x2, $x2, $x1)
                };
            }

            macro_rules! unstable_max {
                ($x1: expr, $x2: expr) => {
                    unstable_cmp!($x1, $x2, $x1, $x2)
                };
            }

            euclid::rect(
                unstable_min!(self.to_x(), self.from_x()),
                unstable_min!(self.to_y(), self.from_y()),
                unstable_max!(self.to_x(), self.from_x()),
                unstable_max!(self.to_y(), self.from_y()),
            )
        }

        #[inline]
        fn contains_point<Pt: Point<T>>(&self, pt: &Pt) -> bool where T: Sub + Mul + Zero + approx::AbsDiffEq {
            let a_tmp: Vector2F = self.to().subtract(self.from());
            let b_tmp: Vector2F = other.subtract(self.from());
            let r = a_tmp.cross_product(b_tmp);
            approx::abs_diff_eq!(r, T::zero())
        }

        #[inline]
        fn is_point_right_of_line<Pt: Point<T>>(&self, pt: &Pt) -> bool where T: Sub + Mul + PartialOrd + Zero {
            let a_tmp: Vector2F = self.to().subtract(self.from());
            let b_tmp: Vector2F = other.subtract(self.from());
            let r = a_tmp.cross_product(b_tmp);
            if let Some(cmp::Ordering::Less) = r.partial_cmp(T::zero()) { true } else { false }
        }

        #[inline]
        fn touches_or_crosses<Other: Line<T>>(&self, other: &Other) -> bool where T: Sub + Mul + PartialOrd + Zero + approx::AbsDiffEq {
            self.contains_point(other.to::<Vector2F>()) || self.contains_point(other.from::<Vector2F>()) || self.is_point_right_of_line(other.to::<Vector2F>()) || self.is_point_right_of_line(other.from::<Vector2F>())
        }
    */
}

pub trait IntersectsAt: Line<f32> {
    #[inline]
    fn intersects_at<Other: Line<f32>>(&self, other: &Other) -> Option<f32> {
        LineSegment2F::new(self.to::<Vector2F>(), self.from::<Vector2F>()).intersection_t(
            LineSegment2F::new(other.to::<Vector2F>(), other.from::<Vector2F>()),
        )
    }

    #[inline]
    fn sample_at<Res: Point<f32>>(&self, t: f32) -> Res {
        let r: Vector2F =
            LineSegment2F::new(self.to::<Vector2F>(), self.from::<Vector2F>()).sample(t);
        Res::from_x_y(r.x(), r.y())
    }
}

impl<T: Line<f32>> IntersectsAt for T {}

impl<T: Copy, Dim> Line<T> for (Point2D<T, Dim>, Point2D<T, Dim>) {
    #[inline]
    fn to_x(&self) -> T {
        self.1.x
    }
    #[inline]
    fn from_x(&self) -> T {
        self.0.x
    }
    #[inline]
    fn to_y(&self) -> T {
        self.1.y
    }
    #[inline]
    fn from_y(&self) -> T {
        self.0.y
    }
    #[inline]
    fn from_x1_y1_x2_y2(x1: T, y1: T, x2: T, y2: T) -> Self {
        (euclid::point2(x1, y1), euclid::point2(x2, y2))
    }
}

impl Line<f32> for LineSegment2F {
    #[inline]
    fn to_x(&self) -> f32 {
        LineSegment2F::to_x(*self)
    }
    #[inline]
    fn from_x(&self) -> f32 {
        LineSegment2F::from_y(*self)
    }
    #[inline]
    fn to_y(&self) -> f32 {
        LineSegment2F::to_y(*self)
    }
    #[inline]
    fn from_y(&self) -> f32 {
        LineSegment2F::from_y(*self)
    }
    #[inline]
    fn from_x1_y1_x2_y2(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self::new(Vector2F::new(x1, y1), Vector2F::new(x2, y2))
    }
}
