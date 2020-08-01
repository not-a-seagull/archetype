// GPLv3 License

mod color;
pub use color::*;

use serde::{Deserialize, Serialize};
use std::boxed::Box;

pub mod colors {
    use super::SolidColor;

    pub const BLACK: SolidColor = unsafe { SolidColor::new_unchecked(0.0, 0.0, 0.0) };
    pub const WHITE: SolidColor = unsafe { SolidColor::new_unchecked(1.0, 1.0, 1.0) };
    pub const RED: SolidColor = unsafe { SolidColor::new_unchecked(1.0, 0.0, 0.0) };
    pub const BLUE: SolidColor = unsafe { SolidColor::new_unchecked(0.0, 0.0, 1.0) };
}

/// A brush.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Brush {
    color: DynamicColor,
    width: u32,
}

impl Brush {
    #[inline]
    pub fn new<'de, C: Color<'de>>(color: C, width: u32) -> Self {
        Self {
            color: DynamicColor::from_color(color),
            width,
        }
    }

    #[inline]
    pub const fn new_const(dn: DynamicColor, width: u32) -> Self {
        Self { color: dn, width }
    }

    #[inline]
    pub fn color(&self) -> &DynamicColor {
        &self.color
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn set_width(&mut self, val: u32) {
        self.width = val;
    }
}
