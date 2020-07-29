// GPLv3 License

use super::{Brush, DrawTarget, Line};
use imageproc::drawing::{self, BresenhamLineIter};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};

// function to draw a thicker line segment onto a canvas
#[inline]
pub fn rasterize_line<Ln: Line<f32>>(c: &DrawTarget, line: &Ln, brush: Brush) {
    let line_iter =
        BresenhamLineIter::new((line.from_x(), line.from_y()), (line.to_x(), line.to_y()));

    let color = brush.color().into_rgba();
    let line_width = brush.width() as i32;

    let img = RwLock::upgradable_read(c);
    let (width, height) = img.0.dimensions();
    let mut writer = RwLockUpgradableReadGuard::upgrade(img);
    line_iter
        .filter(|(x, y)| *x >= 0 && *x < width as i32 && *y >= 0 && *y < height as i32)
        .for_each(|pt| {
            drawing::draw_filled_ellipse_mut(
                &mut writer.0,
                (pt.0, pt.1),
                line_width,
                line_width,
                color,
            );
        });
}

pub trait Rasterizable {
    fn rasterize(&self, target: &DrawTarget, brush: Brush);
}

impl<T: Line<f32>> Rasterizable for T {
    #[inline]
    fn rasterize(&self, target: &DrawTarget, brush: Brush) {
        rasterize_line(target, self, brush);
    }
}
