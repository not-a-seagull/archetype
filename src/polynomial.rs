// GPL v3.0

use num_traits::Zero;
use smallvec::SmallVec;
use std::{
    cmp, fmt,
    marker::PhantomData,
    ops::{Add, Mul, Sub},
};

pub trait Axis {
    fn axis() -> &'static str;
}

/// A simple polynomial type.
#[repr(transparent)]
pub struct Polynomial<T, TAxis> {
    coefs: SmallVec<[T; 5]>,
    _phantom: PhantomData<TAxis>,
}

impl<T: fmt::Debug, TAxis> fmt::Debug for Polynomial<T, TAxis> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.coefs, f)
    }
}

impl<T: PartialEq, TAxis> PartialEq for Polynomial<T, TAxis> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.coefs == other.coefs
    }
}

impl<T: Eq, TAxis> Eq for Polynomial<T, TAxis> {}

impl<T: fmt::Display + Zero, TAxis: Axis> fmt::Display for Polynomial<T, TAxis> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len_minus_one = self.len() - 1;
        let len_minus_two = self.len() - 2;

        self.coefs
            .iter()
            .enumerate()
            .rev()
            .map(
                |(i, coef)| match (coef.is_zero(), i == len_minus_one, i == len_minus_two) {
                    (true, _, _) => Ok(()),
                    (false, true, _) => fmt::Display::fmt(coef, f),
                    (false, false, true) => write!(f, "{}{} + ", coef, TAxis::axis()),
                    (_, _, _) => write!(f, "{}{}^{} + ", coef, TAxis::axis(), i),
                },
            )
            .collect::<Result<_, _>>()?;

        Ok(())
    }
}

impl<T, Axis> Polynomial<T, Axis> {
    pub fn len(&self) -> usize {
        self.coefs.len()
    }

    pub fn new<I: IntoIterator<Item = T>>(i: I) -> Self {
        Self {
            coefs: i.into_iter().collect(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Copy, Axis> Polynomial<T, Axis> {
    pub fn get(&self, i: usize) -> Option<T> {
        self.coefs.iter().copied().nth(i)
    }

    pub fn from_array<const N: usize>(array: [T; N]) -> Self {
        let mut coefs = SmallVec::with_capacity(N);
        coefs.extend_from_slice(&array);

        Self {
            coefs,
            _phantom: PhantomData,
        }
    }
}

impl<T: Zero + Clone, Axis> Polynomial<T, Axis> {
    fn combine_poly<F>(self, other: Self, f: F) -> Self
    where
        F: Fn(T, T) -> T,
    {
        // make sure the iterators have the same length
        let self_len = self.len();
        let other_len = other.len();
        let max_len = cmp::max(self_len, other_len);

        Self::new(
            self.coefs
                .into_iter()
                .chain(std::iter::repeat(T::zero()).take(max_len - self_len))
                .zip(
                    other
                        .coefs
                        .into_iter()
                        .chain(std::iter::repeat(T::zero()).take(max_len - other_len)),
                )
                .map(|(a, b)| f(a, b)),
        )
    }
}

impl<T: Add<Output = T> + Zero + Clone, Axis> Add for Polynomial<T, Axis> {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        self.combine_poly(other, |a, b| a + b)
    }
}

impl<T: Sub<Output = T> + Zero + Clone, Axis> Sub for Polynomial<T, Axis> {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        self.combine_poly(other, |a, b| a - b)
    }
}

/*impl<T: Mul<Output = T> + Clone, Axis> Mul<T> for Polynomial<T, Axis> {
    type Output = Self;

    #[inline]
    fn mul(self, other: T) -> Self {
        Self::new(self.coefs.into_iter().map(|i| i * other.clone()))
    }
}*/

// iterator to diamondize the array of iterators
struct Diamond<I1: Iterator, I2> {
    inner: std::iter::Fuse<I2>,
    diamond: SmallVec<[I1; 7]>,
}

impl<T, I1: ExactSizeIterator<Item = T>, I2: ExactSizeIterator<Item = I1>> Iterator
    for Diamond<I1, I2>
{
    type Item = SmallVec<[T; 7]>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ir) = self.inner.next() {
            self.diamond.push(ir);
        }

        if self.diamond.is_empty() {
            return None;
        }

        let mut res = SmallVec::new();
        let rejects = self
            .diamond
            .iter_mut()
            .enumerate()
            .map(|(i, r)| match r.next() {
                Some(r) => {
                    res.push(r);
                    None
                }
                None => Some(i),
            })
            .filter(Option::is_some)
            .map(Option::unwrap)
            .collect::<SmallVec<[usize; 7]>>();
        rejects.into_iter().for_each(|i| {
            self.diamond.remove(i);
        });

        if res.is_empty() {
            None
        } else {
            Some(res)
        }
    }
}

trait Diamondize<T, I1: ExactSizeIterator<Item = T> + Sized>:
    ExactSizeIterator<Item = I1> + Sized
{
    fn diamondize(self) -> Diamond<I1, Self> {
        Diamond {
            inner: self.fuse(),
            diamond: SmallVec::new(),
        }
    }
}

impl<T, I1: ExactSizeIterator<Item = T> + Sized, I2: ExactSizeIterator<Item = I1> + Sized>
    Diamondize<T, I1> for I2
{
}

#[test]
fn diamond_test() {
    use smallvec::smallvec;

    let data: SmallVec<[SmallVec<[i32; 3]>; 3]> =
        smallvec![smallvec![1, 2, 3], smallvec![4, 5, 6], smallvec![7, 8, 9],];

    let res = data
        .into_iter()
        .map(|i| i.into_iter())
        .diamondize()
        .collect::<SmallVec<[SmallVec<[i32; 7]>; 3]>>();
    let desired_res: SmallVec<[SmallVec<[i32; 7]>; 3]> = smallvec![
        smallvec![1],
        smallvec![2, 4],
        smallvec![3, 5, 7],
        smallvec![6, 8],
        smallvec![9],
    ];

    assert_eq!(res, desired_res);
}

impl<T: Mul<Output = T> + Copy + std::iter::Sum, Axis> Mul for Polynomial<T, Axis> {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self {
        Self::new(
            self.coefs
                .into_iter()
                .map(|coef1| other.coefs.iter().copied().map(move |coef2| coef1 * coef2))
                .diamondize()
                .map(|sm| sm.into_iter().sum()),
        )
    }
}

#[cfg(test)]
struct TestAxis;

#[cfg(test)]
impl Axis for TestAxis {
    fn axis() -> &'static str {
        "T"
    }
}

#[test]
fn polynomial_add() {
    let a = Polynomial::<i32, TestAxis>::from_array([1, 2, 3]);
    let b = Polynomial::<i32, TestAxis>::from_array([1, 1, 1, 1, 1, 1]);
    let c = Polynomial::<i32, TestAxis>::from_array([2, 3, 4, 1, 1, 1]);

    assert_eq!(a + b, c);
}

#[test]
fn polynomial_sub() {
    let a = Polynomial::<i32, TestAxis>::from_array([1, 2, 3]);
    let b = Polynomial::<i32, TestAxis>::from_array([1, 1, 1, 1, 1, 1]);
    let c = Polynomial::<i32, TestAxis>::from_array([0, 1, 2, -1, -1, -1]);

    assert_eq!(a - b, c);
}

#[test]
fn polynomial_mult() {
    let a = Polynomial::<i32, TestAxis>::new(vec![1i32, 2, 1]);
    let b = Polynomial::<i32, TestAxis>::new(vec![3i32, 5, 5, 9]);
    let c = Polynomial::<i32, TestAxis>::new(vec![3i32, 11, 18, 24, 23, 9]);

    assert_eq!(a * b, c)
}
