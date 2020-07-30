// GPLv3 License

use super::{Brush, DrawTarget, Line, TCImage};
use imageproc::drawing::{self, BresenhamLineIter, Canvas};
use parking_lot::{Mutex, RwLock, RwLockUpgradableReadGuard};
use rayon::prelude::*;
use std::mem;

// function to rasterize a line with a drawing function
#[inline]
fn rasterize_line_custom<F, Ln: Line<f32>>(c: &DrawTarget, line: &Ln, brush: Brush, f: F)
where
    F: Fn(&DrawTarget, i32, i32, &Brush) + Sync,
{
    let line_iter =
        BresenhamLineIter::new((line.from_x(), line.from_y()), (line.to_x(), line.to_y()));

    let img = c.read();
    let (width, height) = img.0.dimensions();
    mem::drop(img);

    line_iter
        .collect::<Vec<(i32, i32)>>()
        .par_iter()
        .filter(|(x, y)| *x >= 0 && *x < width as i32 && *y >= 0 && *y < height as i32)
        .for_each(|(x, y)| {
            f(&c, *x, *y, &brush);
        });
}

// function to draw a 1-pixel wide line segment onto the canvas
#[inline]
pub fn rasterize_thin_line<Ln: Line<f32>>(c: &DrawTarget, line: &Ln, brush: Brush) {
    rasterize_line_custom(c, line, brush, |c, x, y, brush| {
        println!("Drawing pixel at {}, {}", x, y);
        c.write().0.draw_pixel(
            x as u32,
            y as u32,
            brush.color(x as u32, y as u32).into_rgba(),
        )
    });
}

// function to draw an ellipse
#[inline]
fn rasterize_circle(c: &DrawTarget, x0: f32, y0: f32, radius: u32, brush: Brush) {
    struct CircleRasterizer {
        x: i32,
        y: i32,
        p: i32,
    }

    impl Iterator for CircleRasterizer {
        type Item = (i32, i32);

        fn next(&mut self) -> Option<(i32, i32)> {
            if self.x <= self.y {
                return None;
            }

            let res = (self.x, self.y);

            self.x += 1;
            if self.p < 0 {
                self.p += 2 * self.x + 1;
            } else {
                self.y -= 1;
                self.p += 2 * (self.x - self.y) + 1;
            }

            Some(res)
        }
    }

    let (x0, y0) = (x0 as i32, y0 as i32);

    CircleRasterizer {
        x: 0,
        y: radius as i32,
        p: 1 - radius as i32,
    }
    .collect::<Vec<(i32, i32)>>()
    .par_iter()
    .for_each(|(x, y)| {
        rayon::join(
            || {
                rayon::join(
                    || {
                        rasterize_thin_line(
                            c,
                            &(
                                ((x0 - x) as f32, (y0 + y) as f32),
                                ((x0 + x) as f32, (y0 + y) as f32),
                            ),
                            brush.clone(),
                        )
                    },
                    || {
                        rasterize_thin_line(
                            c,
                            &(
                                ((x0 - y) as f32, (y0 + x) as f32),
                                ((x0 + y) as f32, (y0 + x) as f32),
                            ),
                            brush.clone(),
                        )
                    },
                );
            },
            || {
                rayon::join(
                    || {
                        rasterize_thin_line(
                            c,
                            &(
                                ((x0 - x) as f32, (y0 - y) as f32),
                                ((x0 + x) as f32, (y0 - y) as f32),
                            ),
                            brush.clone(),
                        )
                    },
                    || {
                        rasterize_thin_line(
                            c,
                            &(
                                ((x0 - y) as f32, (y0 - x) as f32),
                                ((x0 + y) as f32, (y0 - x) as f32),
                            ),
                            brush.clone(),
                        )
                    },
                );
            },
        );
    });
}

// function to draw a thicker line segment onto a canvas
#[inline]
pub fn rasterize_thick_line<Ln: Line<f32>>(c: &DrawTarget, line: &Ln, brush: Brush) {
    rasterize_line_custom(c, line, brush, |c, x, y, brush| {
        rasterize_circle(c, x as f32, y as f32, brush.width(), brush.clone());
    });
}

pub trait Rasterizable {
    fn rasterize(&self, target: &DrawTarget, brush: Brush);
}

impl<T: Line<f32>> Rasterizable for T {
    #[inline]
    fn rasterize(&self, target: &DrawTarget, brush: Brush) {
        rasterize_thick_line(target, self, brush);
    }
}
