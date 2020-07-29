// MIT License

use super::{Color, DrawTarget, Project, TCImage};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use parking_lot::RwLock;

#[derive(Copy, Clone)]
pub enum RenderTarget {
    SingleImage,
    Mp4,
}

#[derive(Copy, Clone)]
pub enum AlphaMaskTarget<'a> {
    AlphaMask(&'a str),
    Background(Color),
}

impl RenderTarget {
    #[inline]
    pub fn from_char(c: char) -> Option<RenderTarget> {
        Some(match c {
            's' => Self::SingleImage,
            'm' => Self::Mp4,
            _ => return None,
        })
    }

    #[inline]
    pub fn is_single_image(&self) -> bool {
        match self {
            Self::SingleImage => true,
            _ => false,
        }
    }
}

#[inline]
pub fn single_image<'a>(
    project: &Project,
    filename: &str,
    alpha: AlphaMaskTarget<'a>,
) -> Result<(), &'static str> {
    // rasterize onto an image
    let img = RwLock::new((
        TCImage::from_pixel(project.width(), project.height(), Rgba([0, 0, 0, 0])),
        true,
    ));
    project.current_frame().rasterize(&img, project);
    let img = DynamicImage::ImageRgba16(img.into_inner().0);

    img.save_with_format(filename, ImageFormat::Png)
        .map_err(|e| {
            eprintln!("{:?}", e);
            "Unable to write image to file"
        })?;

    Ok(())
}

#[inline]
pub fn render<'a>(
    project: &Project,
    filename: &str,
    target: RenderTarget,
    alpha: AlphaMaskTarget<'a>,
) -> Result<(), &'static str> {
    match target {
        RenderTarget::SingleImage => single_image(project, filename, alpha),
        _ => todo!(),
    }
}
