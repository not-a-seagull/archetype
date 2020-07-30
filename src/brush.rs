// GPL v3.0

use image::{Primitive, Rgba};
use num_traits::{AsPrimitive, Bounded};
use ordered_float::NotNan;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// raw color for ser/deser
#[derive(Serialize, Deserialize)]
struct ColorDeser {
    inner: [f32; 3],
}

/// A three-float color.
#[derive(Copy, Clone)]
pub struct Color {
    r: NotNan<f32>,
    g: NotNan<f32>,
    b: NotNan<f32>,
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let cdr = ColorDeser {
            inner: [
                self.r.into_inner(),
                self.g.into_inner(),
                self.b.into_inner(),
            ],
        };
        Serialize::serialize(&cdr, s)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let cdr: ColorDeser = Deserialize::deserialize(d)?;
        let [r, g, b] = cdr.inner;
        Ok(Color::new(r, g, b).expect("Invalid data"))
    }
}

impl Color {
    #[inline]
    pub fn new(r: f32, g: f32, b: f32) -> Option<Self> {
        Some(Self {
            r: NotNan::new(r).ok()?,
            g: NotNan::new(g).ok()?,
            b: NotNan::new(b).ok()?,
        })
    }

    #[inline]
    pub const unsafe fn new_unchecked(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: NotNan::unchecked_new(r),
            g: NotNan::unchecked_new(g),
            b: NotNan::unchecked_new(b),
        }
    }

    #[inline]
    pub fn r(&self) -> f32 {
        self.r.into_inner()
    }

    #[inline]
    pub fn g(&self) -> f32 {
        self.g.into_inner()
    }

    #[inline]
    pub fn b(&self) -> f32 {
        self.b.into_inner()
    }

    #[inline]
    pub fn into_rgba<T: Primitive + Bounded + Into<f32> + Copy + 'static>(self) -> Rgba<T>
    where
        f32: AsPrimitive<T>,
    {
        #[inline]
        fn clamp_float<T: Primitive + Bounded + Into<f32> + Copy + 'static>(f: f32) -> T
        where
            f32: AsPrimitive<T>,
        {
            (f * T::max_value().into()).as_()
        }

        Rgba([
            clamp_float(self.r()),
            clamp_float(self.g()),
            clamp_float(self.b()),
            T::max_value(),
        ])
    }
}

pub mod colors {
    use super::Color;

    pub const BLACK: Color = unsafe { Color::new_unchecked(0.0, 0.0, 0.0) };
    pub const WHITE: Color = unsafe { Color::new_unchecked(1.0, 1.0, 1.0) };
    pub const RED: Color = unsafe { Color::new_unchecked(1.0, 0.0, 0.0) };
}

/// A brush.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Brush {
    color: Color,
    width: u32,
}

impl Brush {
    #[inline]
    pub const fn new(color: Color, width: u32) -> Self {
        Self { color, width }
    }

    #[inline]
    pub fn color(&self, _x: u32, _y: u32) -> Color {
        self.color
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }
}
