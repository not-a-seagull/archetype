// GPL v3.0

use super::BezierCurve;
use nalgebra::{Matrix2, Vector2};
use ordered_float::NotNan;
use pathfinder_geometry::vector::{vec2f, Vector2F};
use rayon::prelude::*;
use smallvec::{smallvec, SmallVec};
use std::{boxed::Box, iter};

// adapted from http://webdocs.cs.ualberta.ca/~graphics/books/GraphicsGems/gems/FitCurves.c

/// Fit a set of points to a curve.
#[inline]
pub fn fit_curve(points: &[Vector2F], error: f32) -> Vec<BezierCurve> {
    let lt = compute_left_tangent(points);
    let rt = compute_right_tangent(points);
    println!("lt: {:?}, rt: {:?}", &lt, &rt);
    debug_assert!(!lt.x().is_nan());
    debug_assert!(!lt.y().is_nan());
    debug_assert!(!rt.x().is_nan());
    debug_assert!(!rt.y().is_nan());
    fit_cubic(points, lt, rt, error)
}

fn fit_cubic(points: &[Vector2F], lt: Vector2F, rt: Vector2F, error: f32) -> Vec<BezierCurve> {
    if points.len() < 2 {
        // the curve cannot be formed with two points, return an empty vector
        return vec![];
    }

    // if there are only two points, take the shortcut
    if points.len() == 2 {
        let dist = distance(points[1], points[0]) / 3.0f32;

        return vec![BezierCurve::from_points([
            points[0],
            points[0] + (lt * dist),
            points[1] + (rt * dist),
            points[1],
        ])];
    }

    // parameterize points and fit curve
    let u = chord_length_parameterize(points);
    let mut curve = generate_bezier(points, &u, lt, rt);

    // find deviation of points from curve
    let (max_error, midpoint) = compute_max_error(points, &curve, &u);
    let mut mp = midpoint;
    if max_error < error {
        // we have a good enough curve, return
        return vec![curve];
    }

    // try reparameterizing if it's worth it and see if that gives us a better curve
    const MAX_ITERATIONS: usize = 4;
    if max_error < (error * error) {
        for _i in 0..MAX_ITERATIONS {
            println!("Iteration #{}", _i);
            let u_prime = reparameterize(points, &u, &curve);
            curve = generate_bezier(points, &u_prime, lt, rt);
            let (max_error, midpoint) = compute_max_error(points, &curve, &u_prime);
            mp = midpoint;

            if max_error < error {
                return vec![curve];
            }
        }
    }

    // split at point and fit recursively
    let mut ct = compute_center_tangent(points, mp);

    // take advantage of parallel iteration
    let data_set = [
        (lt, ct, &points[0..mp]),
        (ct * -1.0, rt, &points[mp..points.len() - 1]),
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
        let mut data: [[f32; 2]; 2] = rhs.iter().fold([[0.0, 0.0], [0.0, 0.0]], |mut mtrx, rhs| {
            mtrx[0][1] += rhs[0].dot(rhs[0]);
            mtrx[0][1] += rhs[0].dot(rhs[1]);
            mtrx[1][1] += rhs[1].dot(rhs[1]);
            mtrx
        });

        data[1][0] = data[0][1];
        data
    };

    let last_point = points[points.len() - 1];
    let x_mtrx = {
        let data: [f32; 2] = rhs
            .iter()
            .zip(u_prime.iter().copied())
            .zip(points.iter().copied())
            .fold([0.0, 0.0], |mut vctr, ((rhs, u), point)| {
                let tmp = point
                    - ((points[0] * b0(u))
                        + (points[0] * b1(u))
                        + (last_point * b2(u))
                        + (last_point * b3(u)));
                vctr[0] += rhs[0].dot(tmp);
                vctr[1] += rhs[1].dot(tmp);
                vctr
            });
        data
    };

    println!("C-Matrix: {:?}", &c_mtrx);

    // calculate determinants
    // TODO: I'm copying the C math here, there's a better way of doing this
    let det_c0_c1 = (c_mtrx[0][0] * c_mtrx[1][1]) - (c_mtrx[1][0] * c_mtrx[0][1]);
    let det_c0_x = (c_mtrx[0][0] * x_mtrx[1]) - (c_mtrx[1][0] * x_mtrx[0]);
    let det_x_c1 = (x_mtrx[0] * c_mtrx[1][1]) - (x_mtrx[1] * c_mtrx[0][1]);

    // compute alpha values
    let alpha_l = if det_c0_c1.abs() < 1.0e-4 {
        0.0
    } else {
        det_x_c1 / det_c0_c1
    };
    let alpha_r = if det_c0_c1.abs() < 1.0e-4 {
        0.0
    } else {
        det_c0_x / det_c0_c1
    };

    println!("Alpha_l: {:?}, Alpha_r: {:?}", &alpha_l, &alpha_r);

    // if the alphas are negative, use the We/Barsky heuristic
    if alpha_l < 10e-6 || alpha_r < 10e-6 {
        let dist = distance(points[0], last_point) / 3.0f32;
        BezierCurve::from_points([
            points[0],
            points[0] + (lt * dist),
            last_point + (rt * dist),
            last_point,
        ])
    } else {
        BezierCurve::from_points([
            points[0],
            points[0] + (lt * alpha_l),
            last_point + (rt * alpha_r),
            last_point,
        ])
    }
}

// given a get of points, find a better parameterization
#[inline]
fn reparameterize(points: &[Vector2F], u: &[f32], curve: &BezierCurve) -> Box<[f32]> {
    points
        .par_iter()
        .copied()
        .zip(u.par_iter().copied())
        .map(|(pt, u)| newton_raphson_root_find(curve, pt, u))
        .collect::<Vec<f32>>()
        .into_boxed_slice()
}

// use the Newton-Raphson iteration algoritm to find a better parameter
#[inline]
fn newton_raphson_root_find(curve: &BezierCurve, point: Vector2F, u: f32) -> f32 {
    // compute Q(u)
    let qt = curve.eval(u);

    let [start, cp1, cp2, end] = curve.points();
    let qn1 = (*cp1 - *start) * 3.0;
    let qn2 = (*cp2 - *cp1) * 3.0;
    let qn3 = (*end - *cp2) * 3.0;

    let qnn1 = (qn2 - qn1) * 2.0;
    let qnn2 = (qn3 - qn2) * 2.0;

    let qnt = de_casteljau3(u, qn1, qn2, qn3);
    let qnnt = de_casteljau2(u, qnn1, qnn2);

    let numerator = (qt - point).dot(qnt);
    let denominator = qnt.dot(qnt) + (qt - point).dot(qnnt);

    if denominator == 0.0 {
        u
    } else {
        u - (numerator / denominator)
    }
}

// assign parameter values to points
#[inline]
fn chord_length_parameterize(points: &[Vector2F]) -> Box<[f32]> {
    let mut chord_length: SmallVec<[f32; 12]> = SmallVec::with_capacity(points.len());
    let mut total_distance = 0.0f32;
    chord_length.push(0.0);

    for i in 1..points.len() {
        total_distance += distance(points[i - 1], points[i]);
        chord_length.push(total_distance);
    }

    chord_length
        .into_iter()
        .map(|d| d / total_distance)
        .collect::<Vec<f32>>()
        .into_boxed_slice()
}

// compute the error
#[inline]
fn compute_max_error(points: &[Vector2F], curve: &BezierCurve, u: &[f32]) -> (f32, usize) {
    points
        .par_iter()
        .copied()
        .enumerate()
        .zip(u.par_iter().copied())
        .map(|((i, pt), u_t)| {
            let p = curve.eval(u_t);
            let v = pt - p;
            let dist = v.square_length();

            (dist, i)
        })
        .max_by(|(dist1, i1), (dist2, i2)| {
            NotNan::new(*dist1)
                .unwrap()
                .cmp(&NotNan::new(*dist2).unwrap())
        })
        .unwrap()
}

/// Compute the left tangent from points.
#[inline]
fn compute_left_tangent(points: &[Vector2F]) -> Vector2F {
    (points[1] - points[0]).normalize()
}

#[inline]
fn compute_right_tangent(points: &[Vector2F]) -> Vector2F {
    (points[points.len() - 2] - points[points.len() - 1]).normalize()
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
    (p1.dot(p2)).sqrt()
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

#[inline]
fn de_casteljau2(u: f32, pt1: Vector2F, pt2: Vector2F) -> Vector2F {
    (pt1 * (1.0 - u)) * (pt2 * u)
}

#[inline]
fn de_casteljau3(u: f32, pt1: Vector2F, pt2: Vector2F, pt3: Vector2F) -> Vector2F {
    let p1 = (pt1 * (1.0 - u)) + (pt2 * u);
    let p2 = (pt2 * (1.0 - u)) + (pt3 * u);
    de_casteljau2(u, p1, p2)
}

#[inline]
pub fn de_casteljau4(
    u: f32,
    pt1: Vector2F,
    pt2: Vector2F,
    pt3: Vector2F,
    pt4: Vector2F,
) -> Vector2F {
    let p1 = (pt1 * (1.0 - u)) + (pt2 * u);
    let p2 = (pt2 * (1.0 - u)) + (pt3 * u);
    let p3 = (pt3 * (1.0 - u)) + (pt4 * u);

    de_casteljau3(u, p1, p2, p3)
}
