// GPLv3 License

use euclid::{Point2D, Rect};
use num_traits::{AsPrimitive, Float, Num, Pow, Zero};
use pathfinder_geometry::{
    line_segment::LineSegment2F,
    vector::{Vector2F, Vector2I},
};
use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp, iter,
    ops::{Add, Div, Mul, Neg, Sub},
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
    fn set_to_x(&mut self, val: T);
    fn set_from_x(&mut self, val: T);
    fn set_to_y(&mut self, val: T);
    fn set_from_y(&mut self, val: T);

    #[inline]
    fn to<Pt: Point<T>>(&self) -> Pt {
        Pt::from_x_y(self.to_x(), self.to_y())
    }
    #[inline]
    fn from<Pt: Point<T>>(&self) -> Pt {
        Pt::from_x_y(self.from_x(), self.from_y())
    }
    #[inline]
    fn set_to<Pt: Point<T>>(&mut self, pt: Pt) {
        self.set_to_x(pt.x());
        self.set_to_y(pt.y());
    }
    #[inline]
    fn set_from<Pt: Point<T>>(&mut self, pt: Pt) {
        self.set_from_x(pt.x());
        self.set_from_y(pt.y());
    }

    fn from_x1_y1_x2_y2(x1: T, y1: T, x2: T, y2: T) -> Self;
    #[inline]
    fn from_points<Pt1: Point<T>, Pt2: Point<T>>(pt1: Pt1, pt2: Pt2) -> Self {
        Self::from_x1_y1_x2_y2(pt1.x(), pt1.y(), pt2.x(), pt2.y())
    }
    #[inline]
    fn fit_to_points<Pt: Point<T>>(points: &[Pt]) -> Self
    where
        Pt: Sync,
        T: Add<Output = T>
            + Sub<Output = T>
            + Mul<Output = T>
            + Div<Output = T>
            + Send
            + Sync
            + PartialOrd
            + Zero
            + 'static,
        usize: AsPrimitive<T>,
    {
        assert!(points.len() != 0);

        // get sums of x, y, x*y, and x^2
        let (x, y, xy, x2) = points
            .par_iter()
            .map(|pt| (pt.x(), pt.y(), pt.x().mul(pt.y()), pt.x().mul(pt.x())))
            .reduce(
                || (T::zero(), T::zero(), T::zero(), T::zero()),
                |(xs, ys, xys, x2s), (x, y, xy, x2)| (xs + x, ys + y, xys + xy, x2s + x2),
            );

        // figure out slope and y-intercept
        let n: T = points.len().as_();
        let m = ((n * xy) - (x * y)) / ((n * x2) - (x * x));
        let c = (y - (m * x)) / n;

        // figure out what the X is for the lowest and highest Y
        let pt_y_iter = points.par_iter().map(Point::<T>::y);

        #[inline]
        fn compare<T: PartialOrd>(t1: &T, t2: &T) -> std::cmp::Ordering {
            t1.partial_cmp(t2).unwrap()
        }

        let min_y = pt_y_iter.clone().min_by(compare).unwrap();
        let max_y = pt_y_iter.max_by(compare).unwrap();

        // figure out, through the reverse process, where the x's are
        // y = m*x + c, y - c = m*x, (y - c)/m = x
        #[inline]
        fn y_to_x<T: PartialOrd + Sub<Output = T> + Div<Output = T>>(y: T, m: T, c: T) -> T {
            (y - c) / m
        }

        let min_x = y_to_x(min_y, m, c);
        let max_x = y_to_x(max_y, m, c);

        Self::from_x1_y1_x2_y2(min_x, min_y, max_x, max_y)
    }

    #[inline]
    fn length(&self) -> T
    where
        T: Pow<i32, Output = T> + Float + Sub,
    {
        self.to::<(T, T)>().distance_to(&self.from::<(T, T)>())
    }

    #[inline]
    fn slope(&self) -> T
    where
        T: Sub<Output = T> + Div<Output = T>,
    {
        (self.to_y() - self.from_y()) / (self.to_x() - self.from_x())
    }

    #[inline]
    fn y_intercept(&self) -> T
    where
        T: Sub<Output = T> + Mul<Output = T> + Div<Output = T>,
    {
        // y = mx + b
        // -b = mx - y
        // b = y - mx
        (self.to_y().sub(self.slope().mul(self.to_x())))
    }

    #[inline]
    fn distance_to_point<Pt: Point<T>>(&self, pt: &Pt) -> T
    where
        T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> + Float,
    {
        let a = self.from_y() - self.to_y();
        let b = self.to_x() - self.from_x();
        let c = (self.from_x() * self.to_y()) - (self.to_x() * self.from_y());

        // dist = abs(a * x + b * x + c) / sqrt(a * a + b * b);
        let p1 = (a * pt.x()) + (b * pt.x()) + c;
        a.abs() / (((a * a) + (b * b)).sqrt())
    }

    #[inline]
    fn closest_point<PtIn: Point<T>, PtOut: Point<T>>(&self, pt: &PtIn) -> PtOut
    where
        T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Float + Pow<i32, Output = T>,
    {
        let a_to_b = (self.to_x() - self.from_x(), self.to_y() - self.from_y());
        let dist = self.from::<(T, T)>().distance_to(pt);
        PtOut::from_x_y(dist * a_to_b.0, dist * a_to_b.1)
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

    #[inline]
    fn point_t<Pt: Point<f32>>(&self, pt: &Pt) -> f32 {
        LineSegment2F::new(self.to::<Vector2F>(), self.from::<Vector2F>()).solve_t_for_x(pt.x())
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
    fn set_to_x(&mut self, val: T) {
        self.1.x = val;
    }
    #[inline]
    fn set_from_x(&mut self, val: T) {
        self.0.x = val;
    }
    #[inline]
    fn set_to_y(&mut self, val: T) {
        self.1.y = val;
    }
    #[inline]
    fn set_from_y(&mut self, val: T) {
        self.0.y = val;
    }
    #[inline]
    fn from_x1_y1_x2_y2(x1: T, y1: T, x2: T, y2: T) -> Self {
        (euclid::point2(x1, y1), euclid::point2(x2, y2))
    }
}

impl<T: Copy, Dim> Line<T> for [Point2D<T, Dim>; 2] {
    #[inline]
    fn to_x(&self) -> T {
        self[1].x
    }
    #[inline]
    fn from_x(&self) -> T {
        self[0].x
    }
    #[inline]
    fn to_y(&self) -> T {
        self[1].y
    }
    #[inline]
    fn from_y(&self) -> T {
        self[0].y
    }
    #[inline]
    fn set_to_x(&mut self, val: T) {
        self[1].x = val;
    }
    #[inline]
    fn set_from_x(&mut self, val: T) {
        self[0].x = val;
    }
    #[inline]
    fn set_to_y(&mut self, val: T) {
        self[1].y = val;
    }
    #[inline]
    fn set_from_y(&mut self, val: T) {
        self[0].y = val;
    }
    #[inline]
    fn from_x1_y1_x2_y2(x1: T, y1: T, x2: T, y2: T) -> Self {
        [euclid::point2(x1, y1), euclid::point2(x2, y2)]
    }
}

impl Line<f32> for LineSegment2F {
    #[inline]
    fn to_x(&self) -> f32 {
        LineSegment2F::to_x(*self)
    }
    #[inline]
    fn from_x(&self) -> f32 {
        LineSegment2F::from_x(*self)
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
    fn set_to_x(&mut self, val: f32) {
        LineSegment2F::set_to_x(self, val);
    }
    #[inline]
    fn set_from_x(&mut self, val: f32) {
        LineSegment2F::set_from_x(self, val);
    }
    #[inline]
    fn set_to_y(&mut self, val: f32) {
        LineSegment2F::set_to_y(self, val);
    }
    #[inline]
    fn set_from_y(&mut self, val: f32) {
        LineSegment2F::set_from_y(self, val);
    }
    #[inline]
    fn from_x1_y1_x2_y2(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self::new(Vector2F::new(x1, y1), Vector2F::new(x2, y2))
    }
}

impl<T: Copy> Line<T> for ((T, T), (T, T)) {
    #[inline]
    fn to_x(&self) -> T {
        (self.1).0
    }
    #[inline]
    fn from_x(&self) -> T {
        (self.0).0
    }
    #[inline]
    fn to_y(&self) -> T {
        (self.1).1
    }
    #[inline]
    fn from_y(&self) -> T {
        (self.0).1
    }
    #[inline]
    fn set_to_x(&mut self, val: T) {
        (self.1).0 = val;
    }
    #[inline]
    fn set_from_x(&mut self, val: T) {
        (self.0).0 = val;
    }
    #[inline]
    fn set_to_y(&mut self, val: T) {
        (self.1).1 = val;
    }
    #[inline]
    fn set_from_y(&mut self, val: T) {
        (self.0).1 = val;
    }
    #[inline]
    fn from_x1_y1_x2_y2(x1: T, y1: T, x2: T, y2: T) -> Self {
        ((x1, y1), (x2, y2))
    }
}
