// GPL v3.0

use super::BezierCurve;
use nalgebra::{Matrix2, Vector2};
use pathfinder_geometry::vector::{vec2f, Vector2F};
use rayon::prelude::*;
use smallvec::{smallvec, SmallVec};
use std::{boxed::Box, iter};

// adapted from http://webdocs.cs.ualberta.ca/~graphics/books/GraphicsGems/gems/FitCurves.c

/// Fit a set of points to a curve.
#[inline]
pub fn fit_curve(points: &[Vector2F], error: f32) -> Vec<BezierCurve> {
    let lt = compute_left_tangent(points, 0);
    let rt = compute_right_tangent(points, points.len() - 1);
    fit_cubic(points, lt, rt, error)
}

fn fit_cubic(points: &[Vector2F], lt: Vector2F, rt: Vector2F, error: f32) -> Vec<BezierCurve> {
    // if there are only two points, take the shortcut
    if points.len() == 2 {
        let dist = distance(points[1], points[0]) / 3.0f32;

        return vec![BezierCurve::from_points([
            points[0],
            points[0] + (lt * dist),
            points[3] + (rt * dist),
            points[3],
        ])];
    }

    // parameterize points and fit curve
    let u = chord_length_parameterize(points);
    let mut curve = generate_bezier(points, &u, lt, rt);

    // find deviation of points from curve
    let mut midpoint = 0;
    let mut max_error = compute_max_error(points, &curve, &u, &mut midpoint);
    if max_error < error {
        // we have a good enough curve, return
        return vec![curve];
    }

    // try reparameterizing if it's worth it and see if that gives us a better curve
    const MAX_ITERATIONS: usize = 4;
    if max_error < (error * error) {
        for i in 0..MAX_ITERATIONS {
            let u_prime = reparameterize(points, &u, &curve);
            curve = generate_bezier(points, &u_prime, lt, rt);
            max_error = compute_max_error(points, &curve, &u_prime, &mut midpoint);

            if max_error < error {
                return vec![curve];
            }
        }
    }

    // split at point and fit recursively
    let mut ct = compute_center_tangent(points, midpoint);

    // take advantage of parallel iteration
    let data_set = [
        (lt, ct, &points[0..midpoint - 1]),
        (ct * -1.0, rt, &points[midpoint..points.len() - 1]),
    ];
    data_set
        .par_iter()
        .map(|(l, r, s)| fit_cubic(s, *l, *r, error))
        .flat_map(|p| p.into_par_iter())
        .collect()
}

// generate a bezier curve using least-curves algorithm
fn generate_bezier(
    points: &[Vector2F],
    u_prime: &[f32],
    lt: Vector2F,
    rt: Vector2F,
) -> BezierCurve {
    debug_assert_eq!(points.len(), u_prime.len());

    // rhs for equation
    const EPSILON: f32 = 10e-12;
    let rhs: Vec<[Vector2F; 2]> = u_prime
        .par_iter()
        .copied()
        .map(|u| {
            let v1 = lt * b1(u);
            let v2 = rt * b2(u);
            [v1, v2]
        })
        .collect();

    // create the C and X matrix
    let c_mtrx = {
        let mut data: Matrix2<f32> =
            rhs.iter()
                .fold(Matrix2::from_element(0.0), |mut mtrx, rhs| {
                    mtrx[(0, 0)] += rhs[0].dot(rhs[0]);
                    mtrx[(0, 1)] += rhs[0].dot(rhs[1]);
                    mtrx[(1, 1)] += rhs[1].dot(rhs[1]);
                    mtrx
                });

        data[(1, 0)] = data[(0, 1)];
        data
    };

    let li = points.len() - 1;
    let x_mtrx = {
        let data: Vector2<f32> =
            rhs.iter()
                .enumerate()
                .fold(Vector2::from_element(0.0), |mut vctr, (i, rhs)| {
                    let u = u_prime[i];
                    let tmp = points[i]
                        - ((points[0] * b0(u))
                            + (points[0] * b1(u))
                            + (points[li] * b2(u))
                            + (points[li] * b3(u)));
                    vctr[0] += rhs[0].dot(tmp);
                    vctr[1] += rhs[1].dot(tmp);
                    vctr
                });
        data
    };

    // calculate determinants
    // TODO: I'm copying the C math here, there's a better way of doing this
    let mut det_c0_c1 = c_mtrx.determinant();
    let det_c0_x = (c_mtrx[(0, 0)] * x_mtrx[1]) - (c_mtrx[(0, 1)] * x_mtrx[0]);
    let det_x_c1 = (x_mtrx[0] * c_mtrx[(1, 1)]) - (x_mtrx[1] * c_mtrx[(0, 1)]);

    // prevent divide-by-zero panic
    if det_c0_c1 == 0.0 {
        det_c0_c1 = (c_mtrx[(0, 0)] * c_mtrx[(1, 1)]) * EPSILON;
    }

    // compute alpha values
    let alpha_l = det_x_c1 / det_c0_c1;
    let alpha_r = det_c0_x / det_c0_c1;

    // if the alphas are negative, use the We/Barsky heuristic
    if alpha_l < 10e-6 || alpha_r < 10e-6 {
        let dist = distance(points[0], points[li]) / 3.0f32;
        BezierCurve::from_points([
            points[0],
            points[0] + (lt * dist),
            points[li] + (rt * dist),
            points[li],
        ])
    } else {
        BezierCurve::from_points([
            points[0],
            points[0] + (lt * alpha_l),
            points[li] + (rt * alpha_r),
            points[li],
        ])
    }
}

// given a get of points, find a better parameterization
#[inline]
fn reparameterize(points: &[Vector2F], u: &[f32], curve: &BezierCurve) -> Box<[f32]> {
    points
        .par_iter()
        .copied()
        .enumerate()
        .map(|(i, pt)| newton_raphson_root_find(curve, pt, u[i]))
        .collect::<Vec<f32>>()
        .into_boxed_slice()
}

// use the Newton-Raphson iteration algoritm to find a better parameter
#[inline]
fn newton_raphson_root_find(curve: &BezierCurve, point: Vector2F, u: f32) -> f32 {
    // compute Q(u)
    let q_of_u = curve.eval(u);

    // generate Q'
    // TODO: make this a functional algorithm
    let mut data = SmallVec::<[Vector2F; 3]>::new();
    for i in 0..=2 {
        data.push((curve.point_at(i + 1) - curve.point_at(i)) * 3.0f32);
    }
    let q_1 = BezierCurve::new(data);

    // generate Q''
    let mut data = SmallVec::<[Vector2F; 2]>::new();
    for i in 0..=1 {
        data.push((curve.point_at(i + 1) - curve.point_at(i)) * 2.0f32);
    }
    let q_2 = BezierCurve::new(data);

    let q_1_of_u = q_1.eval(u);
    let q_2_of_u = q_2.eval(u);

    // compute f(u)/f'(u)
    let numerator =
        ((q_of_u.x() - point.x()) * q_1_of_u.x()) + ((q_of_u.y() - point.y()) * q_1_of_u.y());
    let denominator = (q_1_of_u.x().powi(2))
        + (q_1_of_u.y().powi(2))
        + ((q_of_u.x() - point.x()) * q_2_of_u.x())
        + ((q_of_u.y() - point.y()) * q_2_of_u.y());

    u - (numerator / denominator)
}

// assign parameter values to points
#[inline]
fn chord_length_parameterize(points: &[Vector2F]) -> Box<[f32]> {
    let mut chord_length: SmallVec<[f32; 12]> = SmallVec::with_capacity(points.len());
    chord_length.extend(iter::repeat(0.0f32).take(points.len()));
    for i in 1..points.len() {
        chord_length[i] = chord_length[i - 1] + distance(points[i], points[i - 1]);
    }

    for i in 1..points.len() {
        chord_length[i] = chord_length[i] / chord_length[points.len() - 1];
    }

    chord_length.into_vec().into_boxed_slice()
}

// compute the error
#[inline]
fn compute_max_error(
    points: &[Vector2F],
    curve: &BezierCurve,
    u: &[f32],
    midpoint: &mut usize,
) -> f32 {
    *midpoint = points.len() / 2;

    points
        .iter()
        .copied()
        .enumerate()
        .fold(0.0f32, |max_dist, (i, pt)| {
            let p = curve.eval(u[i]);
            let v = p - pt;
            let dist = v.square_length();

            if dist > max_dist {
                *midpoint = i;
                dist
            } else {
                max_dist
            }
        })
}

/// Compute the left tangent from points.
#[inline]
fn compute_left_tangent(points: &[Vector2F], end: usize) -> Vector2F {
    (points[end + 1] - points[end]).normalize()
}

#[inline]
fn compute_right_tangent(points: &[Vector2F], end: usize) -> Vector2F {
    (points[end - 1] - points[end]).normalize()
}

#[inline]
fn compute_center_tangent(points: &[Vector2F], center: usize) -> Vector2F {
    let v1 = points[center - 1] - points[center];
    let v2 = points[center] - points[center + 1];
    ((v1 + v2) / 2.0f32).normalize()
}

// distance between vectors
#[inline]
fn distance(p1: Vector2F, p2: Vector2F) -> f32 {
    let a = (p2.x() - p1.x()).powi(2);
    let b = (p2.y() - p1.y()).powi(2);
    (a + b).sqrt()
}

// bezier multipliers
#[inline]
fn b0(u: f32) -> f32 {
    let a = 1.0 - u;
    a * a * a
}

#[inline]
fn b1(u: f32) -> f32 {
    let a = 1.0 - u;
    3.0 * a * (u * u)
}

#[inline]
fn b2(u: f32) -> f32 {
    let a = 1.0 - u;
    3.0 * u * u * a
}

#[inline]
fn b3(u: f32) -> f32 {
    u * u * u
}
