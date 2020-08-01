// GPLv3 License

use super::Color;
use ordered_float::NotNan;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

// raw color for ser/deser
#[derive(Serialize, Deserialize)]
struct ColorDeser {
    inner: [f32; 3],
}

/// A three-float color.
#[derive(Copy, Clone)]
pub struct SolidColor {
    r: NotNan<f32>,
    g: NotNan<f32>,
    b: NotNan<f32>,
}

impl Serialize for SolidColor {
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

impl<'de> Deserialize<'de> for SolidColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let cdr: ColorDeser = Deserialize::deserialize(d)?;
        let [r, g, b] = cdr.inner;
        Ok(SolidColor::new(r, g, b).expect("Invalid data"))
    }
}

impl SolidColor {
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
}

const MAX_COLOR: NotNan<f32> = unsafe { NotNan::unchecked_new(1.0f32) };

impl<'a> Color<'a> for SolidColor {
    #[inline]
    fn parts(&self, _li: &super::LocationInfo) -> [NotNan<f32>; 4] {
        [self.r, self.g, self.b, MAX_COLOR]
    }
}
