// GPL v3.0 License

mod solid;
pub use solid::*;

use image::{Primitive, Rgba};
use num_traits::{AsPrimitive, Bounded};
use ordered_float::NotNan;
use serde::{Deserialize, Serialize};

/// Information about the current coordinates.
#[derive(Copy, Clone)]
pub struct LocationInfo {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// A trait representing something where a color can be derived from.
pub trait Color<'de>: Serialize + Deserialize<'de> + Copy + Into<DynamicColor> {
    fn parts(&self, loc_info: &LocationInfo) -> [NotNan<f32>; 4];
    #[inline]
    fn r(&self, loc_info: &LocationInfo) -> NotNan<f32> {
        self.parts(loc_info)[0]
    }
    #[inline]
    fn g(&self, loc_info: &LocationInfo) -> NotNan<f32> {
        self.parts(loc_info)[1]
    }
    #[inline]
    fn b(&self, loc_info: &LocationInfo) -> NotNan<f32> {
        self.parts(loc_info)[2]
    }
    #[inline]
    fn a(&self, loc_info: &LocationInfo) -> NotNan<f32> {
        self.parts(loc_info)[3]
    }
    #[inline]
    fn as_rgba<T: Bounded + Copy + Into<f32> + Primitive + 'static>(
        &self,
        loc_info: &LocationInfo,
    ) -> Rgba<T>
    where
        f32: AsPrimitive<T>,
    {
        macro_rules! normalize {
            ($e: expr) => {{
                let max: f32 = T::max_value().into();
                let res: T = ($e * max).as_();
                res
            }};
        }

        let [r, g, b, a] = match self.parts(loc_info) {
            [r, g, b, a] => [
                r.into_inner(),
                g.into_inner(),
                b.into_inner(),
                a.into_inner(),
            ],
        };
        Rgba([normalize!(r), normalize!(g), normalize!(b), normalize!(a)])
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum DynamicColor {
    Solid(SolidColor),
}

impl From<SolidColor> for DynamicColor {
    #[inline]
    fn from(sc: SolidColor) -> Self {
        Self::Solid(sc)
    }
}

impl DynamicColor {
    pub fn from_color<'de, C: Color<'de>>(color: C) -> Self {
        color.into()
    }
}

impl<'de> Color<'de> for DynamicColor {
    #[inline]
    fn parts(&self, loc_info: &LocationInfo) -> [NotNan<f32>; 4] {
        match self {
            Self::Solid(ref s) => s.parts(loc_info),
        }
    }
}
